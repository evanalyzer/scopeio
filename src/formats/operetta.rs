//! # operetta
//!
//! PerkinElmer/Revvity Operetta and Opera Phenix (Harmony export) reader.
//!
//! A Harmony export is a folder with `Index.idx.xml` (or the older
//! `Index.ref.xml` / `Index.xml`): plates, wells and one `Image` entry per
//! plane naming its TIFF file (`URL`), well row and column, field, plane
//! (z), timepoint and channel. Each (well, field) is one series, in row,
//! column and field order; channels, planes and timepoints are the sorted
//! distinct IDs. Planes are read with the TIFF reader; a missing or
//! unreadable plane file reads as zeros. Pixel sizes from
//! `ImageResolutionX/Y`, the z step from `PositionZ`, the time increment
//! from `MeasurementTimeOffset`, channels from `ChannelName` and the
//! excitation / emission wavelengths, the objective from
//! `ObjectiveMagnification` / `ObjectiveNA`.
//!
//! Provenance: own analysis of the public Harmony exports on the OME
//! sample server (CC BY 4.0); checked against Bio-Formats as a black box.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta,
    Position, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Operetta, "PerkinElmer Operetta / Opera Phenix", "1");

/// Largest index file parsed.
const MAX_INDEX: u64 = 256 << 20;

/// Returns `true` for a Harmony index file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    name.ends_with(".xml") && String::from_utf8_lossy(head).contains("<EvaluationInputData")
}

fn local<'a>(n: Node<'a, '_>) -> &'a str {
    n.tag_name().name()
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| local(*c) == name)
}

fn text(n: Node, name: &str) -> Option<String> {
    child(n, name).and_then(|c| c.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn num(n: Node, name: &str) -> Option<f64> {
    text(n, name)?.parse().ok()
}

/// A value in metres (attribute `Unit="m"`, the default) as nanometres.
fn metres_nm(n: Node, name: &str) -> Option<f64> {
    let c = child(n, name)?;
    let v: f64 = c.text()?.trim().parse().ok()?;
    let f = match c.attribute("Unit").unwrap_or("m") {
        "m" => 1e9,
        "mm" => 1e6,
        "um" | "µm" => 1e3,
        "nm" => 1.0,
        _ => return None,
    };
    v.is_finite().then_some(v * f)
}

/// The plane file a `URL` names: next to the index, or (exports with
/// absolute URLs of the acquisition server) `<folder>/<file>` from the URL
/// below one of the index's parent folders.
fn locate(dir: &Path, url: &str) -> PathBuf {
    let parts: Vec<&str> = url.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    let file = parts.last().copied().unwrap_or(url);
    let near = dir.join(file);
    if near.exists() || parts.len() < 2 {
        return near;
    }
    let folder = parts[parts.len() - 2];
    dir.ancestors().take(4).map(|a| a.join(folder).join(file)).find(|p| p.exists()).unwrap_or(near)
}

/// One plane entry of the index.
#[derive(Debug, Clone)]
struct Entry {
    file: PathBuf,
    c: u32,
    z: u32,
    t: u32,
    x_nm: Option<f64>,
    y_nm: Option<f64>,
    z_nm: Option<f64>,
    time_s: Option<f64>,
}

/// One series: plane files by `c + size_c * (z + size_z * t)`.
#[derive(Debug)]
struct Series {
    planes: Vec<Option<PathBuf>>,
    times: Vec<Option<f64>>,
}

/// Harmony export.
#[derive(Debug)]
pub(super) struct Operetta {
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Operetta {
    /// Opens a Harmony index file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or an index without images
    /// and [`Error::UnsupportedVariant`] when no plane file can be read.
    pub fn open(path: &Path) -> Result<Self> {
        if std::fs::metadata(path)?.len() > MAX_INDEX {
            return Err(Error::Corrupt("Harmony index too large".into()));
        }
        let raw = std::fs::read(path)?;
        let xml = String::from_utf8_lossy(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw)).into_owned();
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("Harmony index: {e}")))?;
        let root = doc.root_element();
        let dir = path.parent().unwrap_or(Path::new("."));
        let images = child(root, "Images").ok_or_else(|| Error::Corrupt("Harmony index without images".into()))?;
        // (row, column, field) -> entries; channel names and settings by id.
        let mut groups: BTreeMap<(u32, u32, u32), Vec<Entry>> = BTreeMap::new();
        let mut channels: BTreeMap<u32, ChannelMeta> = BTreeMap::new();
        let mut objective = None;
        let mut first: Option<(Option<f64>, Option<f64>, u64, u64)> = None;
        for im in images.children().filter(|n| local(*n) == "Image") {
            let id = |k: &str| num(im, k).map(|v| v as u32);
            let (Some(row), Some(col), Some(url)) = (id("Row"), id("Col"), text(im, "URL")) else { continue };
            let entry = Entry {
                file: locate(dir, &url),
                c: id("ChannelID").unwrap_or(1),
                z: id("PlaneID").unwrap_or(1),
                t: id("TimepointID").unwrap_or(0),
                x_nm: metres_nm(im, "PositionX"),
                y_nm: metres_nm(im, "PositionY"),
                z_nm: metres_nm(im, "PositionZ"),
                time_s: num(im, "MeasurementTimeOffset"),
            };
            channels.entry(entry.c).or_insert_with(|| ChannelMeta {
                name: text(im, "ChannelName"),
                excitation_wavelength_nm: num(im, "MainExcitationWavelength").filter(|v| *v > 0.0),
                emission_wavelength_nm: num(im, "MainEmissionWavelength").filter(|v| *v > 0.0),
                exposure_s: num(im, "ExposureTime").filter(|v| *v > 0.0),
                contrast_method: text(im, "ChannelType"),
                ..Default::default()
            });
            if objective.is_none() {
                let o = ObjectiveMeta {
                    magnification: num(im, "ObjectiveMagnification").filter(|v| *v > 0.0),
                    numerical_aperture: num(im, "ObjectiveNA").filter(|v| *v > 0.0),
                    ..Default::default()
                };
                objective = (o != ObjectiveMeta::default()).then_some(o);
            }
            if first.is_none() {
                first = Some((
                    metres_nm(im, "ImageResolutionX"),
                    metres_nm(im, "ImageResolutionY"),
                    num(im, "ImageSizeX").unwrap_or(0.0) as u64,
                    num(im, "ImageSizeY").unwrap_or(0.0) as u64,
                ));
            }
            groups.entry((row, col, id("FieldID").unwrap_or(1))).or_default().push(entry);
        }
        if groups.is_empty() {
            return Err(Error::Corrupt("Harmony index without image entries".into()));
        }
        // Geometry and sample type from the first readable plane file.
        let (pixel_type, width, height) = groups
            .values()
            .flatten()
            .find_map(|e| {
                let tiff = TiffFile::open(&e.file).ok()?;
                let img = TiffImage::new(&tiff, &tiff.ifds[0]).ok()?;
                (img.samples_out == 1 && img.spp == 1).then_some((img.pixel_type, img.width as u64, img.height as u64))
            })
            .ok_or_else(|| Error::UnsupportedVariant("Harmony export without readable gray plane files".into()))?;
        let (res_x, res_y, _, _) = first.unwrap_or((None, None, 0, 0));
        let plate = child(root, "Plates").and_then(|p| child(p, "Plate"));
        let plate_name = plate.and_then(|p| text(p, "Name").or_else(|| text(p, "PlateID")));
        let date = plate.and_then(|p| text(p, "MeasurementStartTime"));
        let channel_ids: Vec<u32> = channels.keys().copied().collect();
        let mut series = Vec::new();
        let mut metas = Vec::new();
        for ((row, col, field), entries) in groups {
            let zs: Vec<u32> = entries.iter().map(|e| e.z).collect::<BTreeSet<_>>().into_iter().collect();
            let ts: Vec<u32> = entries.iter().map(|e| e.t).collect::<BTreeSet<_>>().into_iter().collect();
            let (size_c, size_z, size_t) = (channel_ids.len(), zs.len(), ts.len());
            let mut planes = vec![None; size_c * size_z * size_t];
            let mut times = vec![None; planes.len()];
            for e in &entries {
                let (Some(c), Some(z), Some(t)) = (channel_ids.iter().position(|v| *v == e.c), zs.iter().position(|v| *v == e.z), ts.iter().position(|v| *v == e.t)) else { continue };
                let i = c + size_c * (z + size_z * t);
                planes[i].get_or_insert_with(|| e.file.clone());
                times[i] = e.time_s;
            }
            // z step from the plane positions of the first channel and timepoint.
            let positions: Vec<f64> = zs.iter().filter_map(|&z| entries.iter().find(|e| e.z == z && e.t == ts[0] && e.c == channel_ids[0]).and_then(|e| e.z_nm)).collect();
            let z_nm = (positions.len() > 1).then(|| (positions[positions.len() - 1] - positions[0]).abs() / (positions.len() - 1) as f64).filter(|v| *v > 0.0);
            let time_increment_s = (size_t > 1)
                .then(|| {
                    let at = |t: usize| times.get(size_c * size_z * t).copied().flatten();
                    Some((at(size_t - 1)? - at(0)?) / (size_t - 1) as f64).filter(|v| *v > 0.0)
                })
                .flatten();
            let well = format!("{}{:02}", (b'A' + (row.saturating_sub(1) % 26) as u8) as char, col);
            let pos = entries.first();
            metas.push(SeriesMeta {
                name: Some(match &plate_name {
                    Some(p) => format!("{p} well {well} field {field}"),
                    None => format!("well {well} field {field}"),
                }),
                acquisition_date: date.clone(),
                size_c: size_c as u32,
                size_z: size_z as u32,
                size_t: size_t as u32,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: res_x.filter(|v| *v > 0.0), y_nm: res_y.filter(|v| *v > 0.0), z_nm },
                time_increment_s,
                channels: channel_ids.iter().map(|c| channels[c].clone()).collect(),
                objective: objective.clone(),
                stage_position: pos.map(|e| Position { x_nm: e.x_nm, y_nm: e.y_nm, z_nm: e.z_nm }),
                ..Default::default()
            });
            series.push(Series { planes, times });
        }
        let mut original = OriginalMeta::default();
        if let Some(p) = plate {
            for c in p.children().filter(|c| c.is_element() && c.text().is_some_and(|t| !t.trim().is_empty())) {
                original.pairs.push((format!("Plate {}", local(c)), c.text().unwrap_or_default().trim().to_string()));
            }
        }
        Ok(Operetta { series, metas, original })
    }
}

impl Backend for Operetta {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        Ok(PlaneMeta { delta_t_s: self.series[series].times.get(index).copied().flatten(), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let zeros = |sink: &mut Sink| -> Result<()> {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            Ok(())
        };
        let Some(file) = &self.series[series].planes[index] else { return zeros(sink) };
        let Ok(tiff) = TiffFile::open(file) else { return zeros(sink) };
        let img = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if (img.width as u64, img.height as u64, img.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt(format!("Harmony plane {} differs from the first plane", file.display())));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

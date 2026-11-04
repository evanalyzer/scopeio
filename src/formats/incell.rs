//! # incell
//!
//! GE / Cytiva IN Cell Analyzer reader (`.xdce` and its TIFF files).
//!
//! The `.xdce` file is XML: `ImageStack` with the acquisition settings
//! (`ObjectiveCalibration` with the pixel size, `Wavelength` entries with
//! excitation / emission filters and exposure, `Plate` with rows and
//! columns, `Creation`) and `Images` listing every image: `filename`,
//! `Well` (`Row` / `Column` numbers from 1), `Identifier` (`field_index`,
//! `wave_index`, `z_index`, `time_index`), `PlatePosition_um`,
//! `FocusPosition` and `acquisition_time_ms`. Each (well, field) is one
//! series in row, column and field order; channels are the waves, z and t
//! the indices. A wave imaged in 2-D within a z stack (only `z_index` 0)
//! serves its plane at every z. The image files sit next to the `.xdce` file (or in its
//! folder named by `filename`); a missing or unreadable file reads as zeros.
//! Older files list no images (`Images number="0"`): the image files are
//! then found by name, `<row> - <col>(fld <n> wv <ex> - <em>[ z <n>]
//! [ time <n>]).tif`, the wave matched to the `Wavelength` filter names.
//!
//! Provenance: IN Cell Analyzer 2200 user manual (Cytiva, `doc/incell/`;
//! image stack and TIFF output); own analysis of the public IN Cell
//! acquisitions on the OME sample server (CC BY 4.0 / IDR); checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
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
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta, PlateMeta, Position, Region,
    Result, SeriesMeta, WellSample,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::InCell, "IN Cell Analyzer", "1").extensions(&["xdce"]);

/// Largest `.xdce` parsed.
const MAX_INDEX: u64 = 256 << 20;

/// Returns `true` for an IN Cell image stack file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xdce")) && String::from_utf8_lossy(head).contains("<ImageStack")
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn descendant<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.descendants().find(|c| c.tag_name().name() == name)
}

fn num(n: Option<Node>, attr: &str) -> Option<f64> {
    n?.attribute(attr)?.trim().parse().ok().filter(|v: &f64| v.is_finite())
}

/// Fields of an image file name `<row> - <col>(fld <n> wv <wave>[ z <n>]
/// [ time <n>]).tif`: row (from 1), column, field (from 0), wave name, z
/// and time index (from 0).
fn parse_name(name: &str) -> Option<(u32, u32, u32, String, u32, u32)> {
    let stem = name.strip_suffix(".tif").or_else(|| name.strip_suffix(".TIF"))?;
    let (well, rest) = stem.split_once('(')?;
    let rest = rest.strip_suffix(')')?;
    let (row, col) = well.split_once(" - ")?;
    let row = row.trim().bytes().try_fold(0u32, |a, b| b.is_ascii_uppercase().then(|| a * 26 + (b - b'A' + 1) as u32))?;
    let col: u32 = col.trim().parse().ok()?;
    let rest = rest.strip_prefix("fld ")?;
    let (field, rest) = rest.split_once(" wv ")?;
    let field: u32 = field.trim().parse().ok()?;
    // Optional " z <n>" and " time <n>" suffixes, counted from 1.
    let mut wave = rest.to_string();
    let mut take = |key: &str| -> Option<u32> {
        let at = wave.rfind(key)?;
        let v: u32 = wave[at + key.len()..].trim().parse().ok()?;
        wave.truncate(at);
        Some(v)
    };
    let t = take(" time ").map_or(0, |v| v.saturating_sub(1));
    let z = take(" z ").map_or(0, |v| v.saturating_sub(1));
    (row > 0 && field > 0).then(|| (row, col, field - 1, wave.trim().to_string(), z, t))
}

/// Image entries from the file names in `dir`, for indexes listing none.
fn images_by_name(root: Node, dir: &Path) -> BTreeMap<(u32, u32, u32), Vec<Entry>> {
    // Wave index by "<excitation> - <emission>" filter names.
    let waves: Vec<String> = descendant(root, "Wavelengths")
        .map(|w| {
            w.children()
                .filter(|n| n.tag_name().name() == "Wavelength")
                .map(|n| {
                    let f = |t: &str| child(n, t).and_then(|c| c.attribute("name")).unwrap_or_default().to_string();
                    format!("{} - {}", f("ExcitationFilter"), f("EmissionFilter"))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut extra: Vec<String> = Vec::new();
    let mut groups: BTreeMap<(u32, u32, u32), Vec<Entry>> = BTreeMap::new();
    let Ok(read) = std::fs::read_dir(dir) else { return groups };
    let mut names: Vec<String> = read.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for name in names {
        let Some((row, col, field, wave, z, t)) = parse_name(&name) else { continue };
        let wave_index = match waves.iter().position(|w| *w == wave) {
            Some(i) => i,
            None => {
                waves.len()
                    + extra.iter().position(|w| *w == wave).unwrap_or_else(|| {
                        extra.push(wave.clone());
                        extra.len() - 1
                    })
            }
        };
        groups.entry((row, col, field)).or_default().push(Entry {
            file: dir.join(&name),
            wave: wave_index as u32,
            z,
            t,
            time_s: None,
            position: None,
        });
    }
    groups
}

/// One image entry.
#[derive(Debug, Clone)]
struct Entry {
    file: PathBuf,
    wave: u32,
    z: u32,
    t: u32,
    time_s: Option<f64>,
    position: Option<Position>,
}

#[derive(Debug)]
struct Series {
    /// Image files by `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<PathBuf>>,
    plane_meta: Vec<PlaneMeta>,
}

/// IN Cell acquisition.
#[derive(Debug)]
pub(super) struct InCell {
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    original: OriginalMeta,
}

impl InCell {
    /// Opens an `.xdce` file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or no image entries and
    /// [`Error::UnsupportedVariant`] when no image file can be read.
    pub fn open(path: &Path) -> Result<Self> {
        if std::fs::metadata(path)?.len() > MAX_INDEX {
            return Err(Error::Corrupt("IN Cell index too large".into()));
        }
        let raw = std::fs::read(path)?;
        // Units are written as a Latin-1 "µ".
        let xml = match std::str::from_utf8(&raw) {
            Ok(s) => s.to_string(),
            Err(_) => raw.iter().map(|&b| b as char).collect(),
        };
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("IN Cell index: {e}")))?;
        let root = doc.root_element();
        let dir = path.parent().unwrap_or(Path::new("."));
        let images = descendant(root, "Images").ok_or_else(|| Error::Corrupt("IN Cell index without Images".into()))?;
        let mut groups: BTreeMap<(u32, u32, u32), Vec<Entry>> = BTreeMap::new();
        for im in images.children().filter(|n| n.tag_name().name() == "Image") {
            let Some(name) = im.attribute("filename") else { continue };
            let well = child(im, "Well");
            let id = child(im, "Identifier");
            let index = |a: &str| num(id, a).map_or(0, |v| v.max(0.0) as u32);
            let (Some(row), Some(col)) = (num(well.and_then(|w| child(w, "Row")), "number"), num(well.and_then(|w| child(w, "Column")), "number"))
            else {
                continue;
            };
            // Only the file name: never leave the acquisition folder.
            let Some(base) = Path::new(&name.replace('\\', "/")).file_name().map(|b| b.to_os_string()) else { continue };
            let pos = child(im, "PlatePosition_um");
            let um = |n: Option<Node>, a: &str| num(n, a).map(|v| v * 1e3);
            let position = Position { x_nm: um(pos, "x"), y_nm: um(pos, "y"), z_nm: um(child(im, "FocusPosition"), "z") };
            let entry = Entry {
                file: dir.join(base),
                wave: index("wave_index"),
                z: index("z_index"),
                t: index("time_index"),
                time_s: im.attribute("timestamp_sec").and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite()),
                position: (position != Position::default()).then_some(position),
            };
            groups.entry((row as u32, col as u32, index("field_index"))).or_default().push(entry);
        }
        if groups.is_empty() {
            groups = images_by_name(root, dir);
        }
        if groups.is_empty() {
            return Err(Error::Corrupt("IN Cell index without image entries".into()));
        }
        // Geometry and sample type from the first readable image file.
        let (pixel_type, width, height) = groups
            .values()
            .flatten()
            .find_map(|e| {
                let tiff = TiffFile::open_first(&e.file).ok()?;
                let img = TiffImage::new(&tiff, &tiff.ifds[0]).ok()?;
                (img.samples_out == 1).then_some((img.pixel_type, img.width as u64, img.height as u64))
            })
            .ok_or_else(|| Error::UnsupportedVariant("IN Cell acquisition without readable image files".into()))?;
        let waves: Vec<u32> = groups.values().flatten().map(|e| e.wave).collect::<BTreeSet<_>>().into_iter().collect();
        let zs: Vec<u32> = groups.values().flatten().map(|e| e.z).collect::<BTreeSet<_>>().into_iter().collect();
        let ts: Vec<u32> = groups.values().flatten().map(|e| e.t).collect::<BTreeSet<_>>().into_iter().collect();
        let (size_c, size_z, size_t) = (waves.len(), zs.len(), ts.len());
        // Waves without z planes besides the first, in a z stack.
        let flat: Vec<bool> = waves.iter().map(|&w| groups.values().flatten().filter(|e| e.wave == w).all(|e| e.z == zs[0])).collect();
        let n = size_c.checked_mul(size_z).and_then(|v| v.checked_mul(size_t)).filter(|&v| v.saturating_mul(groups.len()) <= 1 << 26);
        let n = n.ok_or_else(|| Error::Corrupt("IN Cell acquisition with too many planes".into()))?;

        // Acquisition settings.
        let cal = descendant(root, "ObjectiveCalibration");
        let pixel = |a: &str| num(cal, a).filter(|v| *v > 0.0).map(|um| um * 1e3);
        let objective = cal.map(|c| ObjectiveMeta {
            magnification: num(Some(c), "magnification").filter(|v| *v > 0.0),
            numerical_aperture: num(Some(c), "numerical_aperture").filter(|v| *v > 0.0),
            ..Default::default()
        });
        let wave_nodes: Vec<Node> =
            descendant(root, "Wavelengths").map(|w| w.children().filter(|n| n.tag_name().name() == "Wavelength").collect()).unwrap_or_default();
        let channels: Vec<ChannelMeta> = waves
            .iter()
            .map(|&w| {
                let node = wave_nodes
                    .iter()
                    .find(|n| n.attribute("index").and_then(|v| v.parse::<u32>().ok()) == Some(w))
                    .or(wave_nodes.get(w as usize))
                    .copied();
                let filter = |f: &str| node.and_then(|n| child(n, f));
                let exposure = filter("Exposure");
                ChannelMeta {
                    name: node.and_then(|n| n.attribute("name")).filter(|s| !s.is_empty()).map(String::from),
                    excitation_wavelength_nm: num(filter("ExcitationFilter"), "wavelength").filter(|v| *v > 0.0),
                    emission_wavelength_nm: num(filter("EmissionFilter"), "wavelength").filter(|v| *v > 0.0),
                    exposure_s: num(exposure, "time")
                        .filter(|v| *v > 0.0)
                        .map(|v| if exposure.and_then(|e| e.attribute("unit")) == Some("s") { v } else { v / 1e3 }),
                    ..Default::default()
                }
            })
            .collect();
        let z_step = descendant(root, "ZDimensionParameters").and_then(|z| num(Some(z), "step")).filter(|v| *v > 0.0).map(|um| um * 1e3);
        let creation = descendant(root, "Creation").and_then(|c| Some(format!("{}T{}", c.attribute("date")?, c.attribute("time")?)));
        let plate_node = descendant(root, "Plate");
        let plate_name = root.attribute("PlateID").filter(|s| !s.is_empty()).map(String::from);

        let mut series = Vec::new();
        let mut metas = Vec::new();
        let mut wells = Vec::new();
        for ((row, col, field), entries) in groups {
            let mut s = Series { planes: vec![None; n], plane_meta: vec![PlaneMeta::default(); n] };
            let mut t0 = None;
            for e in &entries {
                let (Ok(c), Ok(z), Ok(t)) = (waves.binary_search(&e.wave), zs.binary_search(&e.z), ts.binary_search(&e.t)) else { continue };
                let i = c + size_c * (z + size_z * t);
                s.planes[i].get_or_insert_with(|| e.file.clone());
                s.plane_meta[i] = PlaneMeta { delta_t_s: e.time_s, exposure_s: channels.get(c).and_then(|ch| ch.exposure_s), position: e.position };
                t0 = match (t0, e.time_s) {
                    (Some(a), Some(b)) => Some(f64::min(a, b)),
                    (a, b) => a.or(b),
                };
            }
            // 2-D waves: the first plane at every z.
            for (c, _) in flat.iter().enumerate().filter(|(_, f)| **f) {
                for t in 0..size_t {
                    let first = c + size_c * size_z * t;
                    for z in 1..size_z {
                        let i = c + size_c * (z + size_z * t);
                        if s.planes[i].is_none() {
                            s.planes[i] = s.planes[first].clone();
                            s.plane_meta[i] = s.plane_meta[first];
                        }
                    }
                }
            }
            // Times relative to the first image of the series.
            for pm in &mut s.plane_meta {
                pm.delta_t_s = pm.delta_t_s.zip(t0).map(|(v, t0)| v - t0);
            }
            let well = format!("{}{:02}", (b'A' + (row.saturating_sub(1) % 26) as u8) as char, col);
            wells.push(WellSample {
                row: row.saturating_sub(1),
                col: col.saturating_sub(1),
                field,
                series: metas.len(),
                position: entries.first().and_then(|e| e.position),
            });
            metas.push(SeriesMeta {
                name: Some(format!("well {well} field {}", field + 1)),
                acquisition_date: creation.clone(),
                size_c: size_c as u32,
                size_z: size_z as u32,
                size_t: size_t as u32,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: pixel("pixel_width"), y_nm: pixel("pixel_height"), z_nm: z_step.filter(|_| size_z > 1) },
                channels: channels.clone(),
                objective: objective.clone(),
                stage_position: entries.first().and_then(|e| e.position),
                ..Default::default()
            });
            series.push(s);
        }
        let rows = num(plate_node, "rows").map_or(0, |v| v as u32);
        let cols = num(plate_node, "columns").map_or(0, |v| v as u32);
        let plates = vec![PlateMeta { name: plate_name, rows, cols, wells }];
        let mut original = OriginalMeta::default();
        if let Some(a) = descendant(root, "Application") {
            for at in a.attributes() {
                original.pairs.push((format!("Application {}", at.name()), at.value().to_string()));
            }
        }
        if let Some(p) = plate_node {
            original.pairs.push(("Plate".into(), p.attribute("name").unwrap_or_default().to_string()));
        }
        Ok(InCell { series, metas, plates, original })
    }
}

impl Backend for InCell {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        Ok(self.series[series].plane_meta.get(i).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let zeros = |sink: &mut Sink| -> Result<()> {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            Ok(())
        };
        let Some(file) = &self.series[series].planes[i] else { return zeros(sink) };
        let Ok(tiff) = TiffFile::open_first(file) else { return zeros(sink) };
        let img = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if (img.width as u64, img.height as u64, img.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt(format!("IN Cell image {} differs from the first image", file.display())));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_image_names() {
        assert_eq!(parse_name("A - 1(fld 1 wv Cy3 - Cy3).tif"), Some((1, 1, 0, "Cy3 - Cy3".into(), 0, 0)));
        assert_eq!(parse_name("C - 05(fld 2 wv DAPI - DAPI z 03).tif"), Some((3, 5, 1, "DAPI - DAPI".into(), 2, 0)));
        assert_eq!(parse_name("AB - 12(fld 1 wv FITC - FITC z 1 time 4).tif"), Some((28, 12, 0, "FITC - FITC".into(), 0, 3)));
        assert_eq!(parse_name("gain(wv DAPI).tif"), None);
    }
}

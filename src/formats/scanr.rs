//! # scanr
//!
//! Olympus scan^R high-content screening reader.
//!
//! A scan^R experiment is a folder with `experiment_descriptor.xml` (a
//! LabVIEW XML document, `<LVData>`: typed elements with `Name` and `Val`,
//! nested in `Cluster` and `Array` elements) and a `data/` folder with one
//! TIFF per plane, named `--W<well>--P<position>--Z<z>--T<t>--<channel>.tif`.
//! Wells are numbered from 1, row by row. The plate geometry follows from
//! the number of entries of the well selection table (the standard plates
//! of 6 to 1536 wells), else from the custom rows and columns. Each imaged
//! (well, position) is one series, in well and position order; channels are
//! the descriptor's channel configurations (name, exposure time in ms) in
//! their order, Z and T the distinct indices of the file names. Planes are
//! read with the TIFF reader; a missing plane reads as zeros.
//!
//! Provenance: own analysis of the public scan^R plate of IDR study
//! idr0009 (OME sample server, CC BY 4.0); plate layout and series order
//! checked against Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
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
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, Plane, PlateMeta, Region, Result, SeriesMeta,
    WellSample,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::ScanR, "Olympus scan^R", "1");

const DESCRIPTOR: &str = "experiment_descriptor.xml";
/// Largest descriptor parsed.
const MAX_DESCRIPTOR: u64 = 64 << 20;
/// Standard plates: (wells, rows, columns).
const PLATES: &[(usize, u32, u32)] = &[(6, 2, 3), (12, 3, 4), (24, 4, 6), (48, 6, 8), (96, 8, 12), (384, 16, 24), (1536, 32, 48)];

/// The descriptor of `path`: the file itself, or the one next to an
/// `experiment_descriptor.dat` or `AcquisitionLog.dat`.
fn descriptor(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    match name.as_str() {
        DESCRIPTOR => Some(path.to_path_buf()),
        "experiment_descriptor.dat" | "acquisitionlog.dat" => Some(path.with_file_name(DESCRIPTOR)).filter(|p| p.is_file()),
        _ => None,
    }
}

/// Returns `true` for a scan^R experiment descriptor or log file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let Some(d) = descriptor(path) else { return false };
    if d == path {
        return String::from_utf8_lossy(head).contains("<LVData");
    }
    let mut start = [0u8; 256];
    let n = std::fs::File::open(&d).and_then(|mut f| std::io::Read::read(&mut f, &mut start)).unwrap_or(0);
    String::from_utf8_lossy(&start[..n]).contains("<LVData")
}

/// LabVIEW XML values as (path of names, value), in document order.
fn flatten(n: Node, path: &mut Vec<String>, out: &mut Vec<(Vec<String>, String)>) {
    for c in n.children().filter(|c| c.is_element()) {
        let tag = c.tag_name().name();
        let name = c.children().find(|x| x.tag_name().name() == "Name").and_then(|x| x.text()).unwrap_or("").trim().to_string();
        match tag {
            "Name" | "NumElts" | "Dimsize" | "Version" => {}
            "Cluster" | "Array" => {
                path.push(name);
                flatten(c, path, out);
                path.pop();
            }
            _ => {
                let value = c.children().find(|x| x.tag_name().name() == "Val").and_then(|x| x.text()).unwrap_or("").trim().to_string();
                let mut p = path.clone();
                p.push(name);
                out.push((p, value));
            }
        }
    }
}

/// Indices parsed from a plane file name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PlaneName {
    well: u32,
    position: u32,
    z: u32,
    t: u32,
    channel: String,
}

fn parse_name(name: &str) -> Option<PlaneName> {
    let stem = name.strip_suffix(".tif").or_else(|| name.strip_suffix(".TIF"))?;
    let mut parts = stem.trim_start_matches('-').splitn(5, "--");
    let mut index = |prefix: char| -> Option<u32> { parts.next()?.strip_prefix(prefix)?.parse().ok() };
    let (well, position, z, t) = (index('W')?, index('P')?, index('Z')?, index('T')?);
    let channel = parts.next()?.to_string();
    Some(PlaneName { well, position, z, t, channel })
}

/// One series: plane files by `c + size_c * (z + size_z * t)`.
#[derive(Debug)]
struct Series {
    planes: Vec<Option<PathBuf>>,
}

/// scan^R experiment.
#[derive(Debug)]
pub(super) struct ScanR {
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    original: OriginalMeta,
}

impl ScanR {
    /// Opens a scan^R experiment from its descriptor (or log) file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid descriptor or a folder
    /// without plane files and [`Error::UnsupportedVariant`] when no plane
    /// file can be read.
    pub fn open(path: &Path) -> Result<Self> {
        let desc = descriptor(path).ok_or_else(|| Error::Corrupt("not a scan^R descriptor".into()))?;
        if std::fs::metadata(&desc)?.len() > MAX_DESCRIPTOR {
            return Err(Error::Corrupt("scan^R descriptor too large".into()));
        }
        let raw = std::fs::read(&desc)?;
        let xml = String::from_utf8_lossy(&raw).into_owned();
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("scan^R descriptor: {e}")))?;
        let mut values = Vec::new();
        flatten(doc.root_element(), &mut Vec::new(), &mut values);
        let key = |k: &str| values.iter().find(|(p, _)| p.last().is_some_and(|l| l == k)).map(|(_, v)| v.as_str());
        let num = |k: &str| key(k).and_then(|v| v.parse::<f64>().ok());
        // Channel configurations, in order.
        let mut channels: Vec<ChannelMeta> = Vec::new();
        let mut channel_names: Vec<String> = Vec::new();
        // The exposure time (ms) comes before the name in each configuration.
        let mut exposure: Option<f64> = None;
        for (p, v) in &values {
            if p.len() >= 2 && p[p.len() - 2] == "channel config" {
                match p[p.len() - 1].as_str() {
                    "exposure time" => exposure = v.parse().ok(),
                    "name" => {
                        channel_names.push(v.clone());
                        channels.push(ChannelMeta {
                            name: Some(v.clone()).filter(|v| !v.is_empty()),
                            exposure_s: exposure.take().map(|ms| ms / 1e3).filter(|v| *v > 0.0),
                            ..Default::default()
                        });
                    }
                    _ => {}
                }
            }
        }
        // Plane files.
        let data = desc.with_file_name("data");
        let mut files: Vec<(PlaneName, PathBuf)> = std::fs::read_dir(&data)?
            .filter_map(|e| e.ok())
            .filter_map(|e| Some((parse_name(&e.file_name().to_string_lossy())?, e.path())))
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(Error::Corrupt("scan^R experiment without plane files".into()));
        }
        for f in &files {
            if !channel_names.contains(&f.0.channel) {
                channel_names.push(f.0.channel.clone());
                channels.push(ChannelMeta { name: Some(f.0.channel.clone()), ..Default::default() });
            }
        }
        let zs: Vec<u32> = files.iter().map(|f| f.0.z).collect::<BTreeSet<_>>().into_iter().collect();
        let ts: Vec<u32> = files.iter().map(|f| f.0.t).collect::<BTreeSet<_>>().into_iter().collect();
        let (size_c, size_z, size_t) = (channels.len(), zs.len(), ts.len());
        // Geometry and sample type from the first readable plane file.
        let (pixel_type, width, height) = files
            .iter()
            .find_map(|(_, p)| {
                let tiff = TiffFile::open(p).ok()?;
                let img = TiffImage::new(&tiff, &tiff.ifds[0]).ok()?;
                (img.samples_out == 1 && img.spp == 1).then_some((img.pixel_type, img.width as u64, img.height as u64))
            })
            .ok_or_else(|| Error::UnsupportedVariant("scan^R experiment without readable gray plane files".into()))?;
        // Plate geometry.
        // The well selection table holds one well number (and a cDNA text)
        // per well of the plate.
        let wells = values
            .iter()
            .filter(|(p, v)| p.iter().any(|s| s.starts_with("well selection table")) && v.parse::<u32>().is_ok())
            .count();
        let custom = (num("custom row").unwrap_or(0.0) as u32, num("custom column").unwrap_or(0.0) as u32);
        let (rows, cols) = PLATES
            .iter()
            .find(|p| p.0 == wells)
            .map(|p| (p.1, p.2))
            .or(Some(custom).filter(|c| c.0 > 0 && c.1 > 0))
            .unwrap_or((1, files.iter().map(|f| f.0.well).max().unwrap_or(1)));
        let plate_name = key("plate name").map(str::to_string).filter(|v| !v.is_empty());
        let mut groups: BTreeMap<(u32, u32), Vec<&(PlaneName, PathBuf)>> = BTreeMap::new();
        for f in &files {
            groups.entry((f.0.well, f.0.position)).or_default().push(f);
        }
        let mut series = Vec::new();
        let mut metas = Vec::new();
        let mut samples = Vec::new();
        for ((well, position), entries) in groups {
            let mut planes = vec![None; size_c * size_z * size_t];
            for (n, p) in entries {
                let (Some(c), Some(z), Some(t)) =
                    (channel_names.iter().position(|v| *v == n.channel), zs.iter().position(|v| *v == n.z), ts.iter().position(|v| *v == n.t))
                else {
                    continue;
                };
                planes[c + size_c * (z + size_z * t)] = Some(p.clone());
            }
            let index = well.saturating_sub(1);
            let (row, col) = (index / cols.max(1), index % cols.max(1));
            let label = format!("{}{:02}", (b'A' + (row % 26) as u8) as char, col + 1);
            samples.push(WellSample { row, col, field: position.saturating_sub(1), series: metas.len(), position: None });
            metas.push(SeriesMeta {
                name: Some(match &plate_name {
                    Some(p) => format!("{p} well {label} position {position}"),
                    None => format!("well {label} position {position}"),
                }),
                size_c: size_c as u32,
                size_z: size_z as u32,
                size_t: size_t as u32,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                channels: channels.clone(),
                ..Default::default()
            });
            series.push(Series { planes });
        }
        let plates = vec![PlateMeta { name: plate_name, rows, cols, wells: samples }];
        let mut original = OriginalMeta::default();
        for (p, v) in values.iter().filter(|(p, v)| p.len() == 2 && !v.is_empty()) {
            original.pairs.push((p[1].clone(), v.clone()));
        }
        Ok(ScanR { series, metas, plates, original })
    }
}

impl Backend for ScanR {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
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
            return Err(Error::Corrupt(format!("scan^R plane {} differs from the first plane", file.display())));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_names() {
        let n = parse_name("--W00002--P00001--Z00000--T00003--nucleus-dapi.tif").unwrap();
        assert_eq!((n.well, n.position, n.z, n.t, n.channel.as_str()), (2, 1, 0, 3, "nucleus-dapi"));
        assert!(parse_name("2").is_none());
        assert!(parse_name("--W00002--P00001.tif").is_none());
    }
}

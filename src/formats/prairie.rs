//! # prairie
//!
//! Bruker Prairie View reader (two-photon and confocal recordings of the
//! Ultima / Investigator systems).
//!
//! A recording is a folder with an XML file whose root is `PVScan`: global
//! acquisition settings (`PVStateShard` with `PVStateValue` keys, indexed
//! by axis or device), then one `Sequence` per cycle with its `Frame`s.
//! Each frame names one TIFF file per channel (`File` with `channel`,
//! `channelName`, `filename` and, for multi-page files of Prairie View 5.7
//! and later, a 1-based `page`), its time (`absoluteTime` since the start
//! of the recording) and its own settings such as the current position.
//! The image data are plain (OME-)TIFF files next to the XML file.
//!
//! Z series (`ZSeries`, `TSeries ZSeries Element`) map the frames of a
//! sequence to z and the sequences to time; frames of a bidirectional z
//! series run backwards in every second cycle and are put back in order.
//! Every other sequence type (`TSeries Timed Element`, brightness over
//! time, single images) maps all frames of all sequences to time. Channels
//! are the distinct channel numbers. Pixel sizes from `micronsPerPixel`,
//! the z step from the frame positions, per-plane times from
//! `absoluteTime`. A plane whose file is missing reads as zeros.
//!
//! Provenance: own analysis of the CatalystNeuro Bruker test recordings
//! (GIN `ophys_testing_data`) and their README; checked against
//! Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use roxmltree::{Document, Node};

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta, Position, Region, Result,
    SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Prairie, "Bruker Prairie View", "1").extensions(&["xml"]);

/// Largest XML file read.
const MAX_XML: u64 = 1 << 30;
/// Open TIFF files kept for reuse.
const CACHED_FILES: usize = 8;

/// Returns `true` for a Prairie View recording XML (`<PVScan` root).
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let xml = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml"));
    xml && head.windows(7).any(|w| w == b"<PVScan")
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(move |c| c.has_tag_name(name))
}

/// Flattens a `PVStateShard`: `key`, `key[index]` and
/// `key[index][subindex]` to their values.
fn state(shard: Node, out: &mut BTreeMap<String, String>) {
    for v in children(shard, "PVStateValue") {
        let Some(key) = v.attribute("key") else { continue };
        if let Some(value) = v.attribute("value") {
            out.insert(key.to_string(), value.to_string());
        }
        for iv in children(v, "IndexedValue") {
            if let (Some(i), Some(value)) = (iv.attribute("index"), iv.attribute("value")) {
                out.insert(format!("{key}[{i}]"), value.to_string());
            }
        }
        for sv in children(v, "SubindexedValues") {
            let Some(i) = sv.attribute("index") else { continue };
            for s in children(sv, "SubindexedValue") {
                if let (Some(j), Some(value)) = (s.attribute("subindex"), s.attribute("value")) {
                    out.insert(format!("{key}[{i}][{j}]"), value.to_string());
                }
            }
        }
    }
}

/// ISO 8601 from `5/27/2024 7:26:53 PM`.
fn pv_date(s: &str) -> Option<String> {
    let mut parts = s.split_whitespace();
    let d: Vec<u32> = parts.next()?.split('/').filter_map(|v| v.parse().ok()).collect();
    let t: Vec<u32> = parts.next()?.split(':').filter_map(|v| v.parse().ok()).collect();
    let pm = parts.next().map(|p| p.eq_ignore_ascii_case("PM"));
    let (&[month, day, year], &[h, m, sec]) = (&d[..], &t[..]) else { return None };
    let h = match pm {
        Some(true) if h < 12 => h + 12,
        Some(false) if h == 12 => 0,
        _ => h,
    };
    ((1..=12).contains(&month) && (1..=31).contains(&day) && h < 24 && m < 60 && sec < 61)
        .then(|| format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{sec:02}"))
}

/// One frame: its files per channel, time and z position.
#[derive(Debug)]
struct FrameInfo {
    files: Vec<(u32, String, u32)>,
    time: Option<f64>,
    z: Vec<f64>,
}

/// Reader for Prairie View recordings.
#[derive(Debug)]
pub(super) struct Prairie {
    meta: [SeriesMeta; 1],
    files: Vec<PathBuf>,
    /// File index and page per plane `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<(u32, u32)>>,
    times: Vec<Option<f64>>,
    zs: Vec<Option<f64>>,
    cache: Mutex<Vec<(u32, Arc<TiffFile>)>>,
    original: OriginalMeta,
}

impl Prairie {
    /// Opens a Prairie View recording from its XML file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or a recording without
    /// readable image file.
    pub fn open(path: &Path) -> Result<Self> {
        let len = std::fs::metadata(path)?.len();
        if len > MAX_XML {
            return Err(Error::Corrupt("Prairie View XML larger than 1 GiB".into()));
        }
        let text = std::fs::read_to_string(path)?;
        let doc = Document::parse_with_options(&text, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
            .map_err(|e| Error::Corrupt(format!("Prairie View XML: {e}")))?;
        let root = doc.root_element();
        if !root.has_tag_name("PVScan") {
            return Err(Error::Corrupt("Prairie View XML without PVScan root".into()));
        }
        let dir = path.parent().unwrap_or(Path::new("."));
        let mut global = BTreeMap::new();
        if let Some(shard) = children(root, "PVStateShard").next() {
            state(shard, &mut global);
        }

        // Sequences and their frames.
        let sequences: Vec<Node> = children(root, "Sequence").collect();
        let zseries = !sequences.is_empty() && sequences.iter().all(|s| s.attribute("type").is_some_and(|t| t.contains("ZSeries")));
        let mut cycles: Vec<Vec<FrameInfo>> = Vec::new();
        for s in &sequences {
            let mut frames: Vec<FrameInfo> = children(*s, "Frame")
                .map(|f| {
                    let files = children(f, "File")
                        .filter_map(|file| {
                            let channel = file.attribute("channel")?.trim().parse().ok()?;
                            let page = file.attribute("page").and_then(|p| p.trim().parse::<u32>().ok()).unwrap_or(1).max(1) - 1;
                            Some((channel, file.attribute("filename")?.to_string(), page))
                        })
                        .collect();
                    let mut local = BTreeMap::new();
                    if let Some(shard) = children(f, "PVStateShard").next() {
                        state(shard, &mut local);
                    }
                    let z = (0..4).filter_map(|j| local.get(&format!("positionCurrent[ZAxis][{j}]")).and_then(|v| v.parse().ok())).collect();
                    FrameInfo { files, time: f.attribute("absoluteTime").and_then(|t| t.parse().ok()), z }
                })
                .collect();
            if zseries && s.attribute("bidirectionalZ").is_some_and(|b| b.eq_ignore_ascii_case("true")) && cycles.len() % 2 == 1 {
                frames.reverse();
            }
            cycles.push(frames);
        }

        // Channels: distinct numbers with their names.
        let mut channel_names: BTreeMap<u32, Option<String>> = BTreeMap::new();
        for s in &sequences {
            for file in s.descendants().filter(|n| n.has_tag_name("File")) {
                if let Some(c) = file.attribute("channel").and_then(|c| c.trim().parse().ok()) {
                    let name = file.attribute("channelName").map(String::from);
                    channel_names
                        .entry(c)
                        .and_modify(|n| {
                            if n.is_none() {
                                *n = name.clone()
                            }
                        })
                        .or_insert(name);
                }
            }
        }
        if channel_names.is_empty() {
            return Err(Error::Corrupt("Prairie View recording without image files".into()));
        }
        let channel_index: HashMap<u32, usize> = channel_names.keys().enumerate().map(|(i, &c)| (c, i)).collect();
        let size_c = channel_names.len();
        let (size_z, size_t) = if zseries {
            (cycles.iter().map(Vec::len).max().unwrap_or(1).max(1), cycles.len().max(1))
        } else {
            (1, cycles.iter().map(Vec::len).sum::<usize>().max(1))
        };
        let count = size_c.checked_mul(size_z).and_then(|n| n.checked_mul(size_t)).filter(|n| *n <= 1 << 28);
        let count = count.ok_or_else(|| Error::Corrupt("Prairie View recording with too many planes".into()))?;

        // Planes, deduplicated file list, times and z positions.
        let mut files: Vec<PathBuf> = Vec::new();
        let mut file_index: HashMap<String, u32> = HashMap::new();
        let mut planes = vec![None; count];
        let mut times = vec![None; count];
        let mut zs = vec![None; count];
        let mut z_device: Option<usize> = None;
        let mut t0: Option<f64> = None;
        let frames = cycles.iter().enumerate().flat_map(|(cycle, frames)| frames.iter().enumerate().map(move |(i, f)| (cycle, i, f)));
        let all: Vec<_> = frames.collect();
        // The z device that moves (focus motor, piezo or tunable lens).
        if zseries {
            let first: Vec<&FrameInfo> = cycles.first().map(|c| c.iter().collect()).unwrap_or_default();
            z_device = (0..4).find(|&j| first.windows(2).any(|w| matches!((w[0].z.get(j), w[1].z.get(j)), (Some(a), Some(b)) if a != b)));
        }
        for (cycle, i, frame) in all {
            let (z, t) = if zseries { (i, cycle) } else { (0, cycles[..cycle].iter().map(Vec::len).sum::<usize>() + i) };
            if frame.time.is_some() && t0.is_none() {
                t0 = frame.time;
            }
            for (channel, name, page) in &frame.files {
                let c = channel_index[channel];
                let index = c + size_c * (z + size_z * t);
                let n = files.len() as u32;
                let f = *file_index.entry(name.clone()).or_insert_with(|| {
                    files.push(dir.join(name));
                    n
                });
                planes[index] = Some((f, *page));
                times[index] = frame.time.zip(t0).map(|(a, b)| a - b);
                zs[index] = z_device.and_then(|j| frame.z.get(j).copied());
            }
        }

        // Geometry from the first readable plane.
        let (pixel_type, width, height) = planes
            .iter()
            .flatten()
            .find_map(|&(f, page)| {
                let tiff = TiffFile::from_file(std::fs::File::open(&files[f as usize]).ok()?, page as usize + 1).ok()?;
                let img = TiffImage::new(&tiff, tiff.ifds.get(page as usize)?).ok()?;
                (img.samples_out == 1 && img.spp == 1).then_some((img.pixel_type, img.width as u64, img.height as u64))
            })
            .ok_or_else(|| Error::UnsupportedVariant("Prairie View recording without readable gray image file".into()))?;

        let num = |k: &str| global.get(k).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0);
        let first_cycle_z: Vec<f64> = (0..size_z).filter_map(|z| zs[size_c * z]).collect();
        let z_step = (first_cycle_z.len() > 1)
            .then(|| (first_cycle_z[first_cycle_z.len() - 1] - first_cycle_z[0]).abs() / (first_cycle_z.len() - 1) as f64)
            .filter(|s| *s > 0.0)
            .map(|s| s * 1e3);
        let time_increment_s = if size_t < 2 {
            None
        } else if zseries {
            times[size_c * size_z].zip(times[0]).map(|(a, b)| a - b).filter(|d| *d > 0.0)
        } else {
            num("framePeriod")
        };
        let channels = channel_names
            .iter()
            .map(|(c, name)| ChannelMeta {
                name: name.clone().or_else(|| Some(format!("Ch{c}"))),
                excitation_wavelength_nm: num("laserWavelength[0]"),
                ..Default::default()
            })
            .collect();
        let objective = (global.contains_key("objectiveLens") || num("objectiveLensMag").is_some()).then(|| ObjectiveMeta {
            model: global.get("objectiveLens").cloned(),
            magnification: num("objectiveLensMag"),
            numerical_aperture: num("objectiveLensNA"),
            ..Default::default()
        });
        let meta = SeriesMeta {
            name: path.file_stem().map(|s| s.to_string_lossy().into_owned()),
            acquisition_date: root.attribute("date").and_then(pv_date),
            size_c: size_c as u32,
            size_z: size_z as u32,
            size_t: size_t as u32,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize {
                x_nm: num("micronsPerPixel[XAxis]").map(|v| v * 1e3),
                y_nm: num("micronsPerPixel[YAxis]").map(|v| v * 1e3),
                z_nm: z_step,
            },
            time_increment_s,
            channels,
            objective,
            stage_position: match (global.get("positionCurrent[XAxis][0]"), global.get("positionCurrent[YAxis][0]")) {
                (Some(x), Some(y)) => {
                    Some(Position { x_nm: x.parse::<f64>().ok().map(|v| v * 1e3), y_nm: y.parse::<f64>().ok().map(|v| v * 1e3), z_nm: None })
                }
                _ => None,
            },
            ..Default::default()
        };
        let mut pairs: Vec<(String, String)> = vec![("PVScan.version".into(), root.attribute("version").unwrap_or_default().to_string())];
        pairs.extend(sequences.first().and_then(|s| s.attribute("type")).map(|t| ("Sequence.type".to_string(), t.to_string())));
        pairs.push(("Sequences".into(), sequences.len().to_string()));
        pairs.extend(global);
        Ok(Prairie { meta: [meta], files, planes, times, zs, cache: Mutex::new(Vec::new()), original: OriginalMeta { pairs, documents: Vec::new() } })
    }

    /// The open TIFF file `f`, from the cache or opened now; `None` if it
    /// cannot be read.
    fn tiff(&self, f: u32) -> Option<Arc<TiffFile>> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = cache.iter().position(|(k, _)| *k == f) {
            let entry = cache.remove(i);
            let tiff = entry.1.clone();
            cache.push(entry);
            return Some(tiff);
        }
        let tiff = Arc::new(TiffFile::open(&self.files[f as usize]).ok()?);
        if cache.len() >= CACHED_FILES {
            cache.remove(0);
        }
        cache.push((f, tiff.clone()));
        Some(tiff)
    }
}

impl Backend for Prairie {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.meta
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.meta[0];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let position = self.zs.get(index).copied().flatten().map(|z| Position { x_nm: None, y_nm: None, z_nm: Some(z * 1e3) });
        Ok(PlaneMeta { delta_t_s: self.times.get(index).copied().flatten(), position, ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.meta[0];
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
        let Some((f, page)) = self.planes[index] else { return zeros(sink) };
        let Some(tiff) = self.tiff(f) else { return zeros(sink) };
        let ifd =
            tiff.ifds.get(page as usize).ok_or_else(|| Error::Corrupt(format!("{} has no page {}", self.files[f as usize].display(), page + 1)))?;
        let img = TiffImage::new(&tiff, ifd)?;
        if (img.width as u64, img.height as u64, img.pixel_type, img.samples_out) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type, 1) {
            return Err(Error::Corrupt(format!("Prairie View plane {} differs from the first plane", self.files[f as usize].display())));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates() {
        assert_eq!(pv_date("5/27/2024 7:26:53 PM").as_deref(), Some("2024-05-27T19:26:53"));
        assert_eq!(pv_date("11/3/2022 12:05:01 AM").as_deref(), Some("2022-11-03T00:05:01"));
        assert_eq!(pv_date("13/3/2022 1:05:01 AM"), None);
    }

    #[test]
    fn flattens_state() {
        let doc = Document::parse(
            "<PVStateShard><PVStateValue key=\"a\" value=\"1\"/><PVStateValue key=\"m\"><IndexedValue index=\"XAxis\" value=\"0.5\"/></PVStateValue>\
             <PVStateValue key=\"p\"><SubindexedValues index=\"ZAxis\"><SubindexedValue subindex=\"1\" value=\"-3\"/></SubindexedValues></PVStateValue></PVStateShard>",
        )
        .unwrap();
        let mut out = BTreeMap::new();
        state(doc.root_element(), &mut out);
        assert_eq!(out["a"], "1");
        assert_eq!(out["m[XAxis]"], "0.5");
        assert_eq!(out["p[ZAxis][1]"], "-3");
    }
}

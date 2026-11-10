//! # scanimage
//!
//! ScanImage TIFF reader (laser scanning microscopy, MBF Bioscience /
//! Vidrio Technologies).
//!
//! ScanImage writes one TIFF page per frame and saved channel, in the
//! order volume, slice (flyback frames included), frame, channel. The
//! acquisition settings tell how the pages map to channels, z and time:
//!
//! - File format 3 and 4 (releases 5.3 to today): a BigTIFF with a binary
//!   header at byte 16 (magic 117637889, format version, lengths) holding
//!   the non-varying `SI.*` settings and the ROI groups as JSON.
//! - Format 1 (5.1): `scanimage.SI.*` settings in every page's
//!   ImageDescription after the frame header.
//! - Format 2 (2015): the frame header and the settings as JSON-like text
//!   in the first ImageDescription, the ROI groups as JSON after the last
//!   page, followed by their length.
//! - Releases 4.x (`scanimage.SI4.*`) and 3.x (`state.*`) in the first
//!   ImageDescription.
//!
//! Planar recordings map pages to time points. Fast z stacks (piezo or
//! tunable lens) map the slices of each volume to z and the volumes to
//! time; their flyback frames are skipped. Slow (motor) stacks map slices
//! to z and the saved frames of each slice, over all volumes, to time.
//! Files holding fewer pages than one volume (a stack stopped early, a
//! recording split over files) keep their complete slices. Multi-ROI pages
//! (mesoscopes) stack the regions of interest top to bottom, separated by
//! the lines recorded while the scanner flies between them; each region
//! becomes a series of its own. Pixel sizes come from the TIFF resolution
//! tags when written in centimetres, else from the imaging field of view
//! (`SI.hRoiManager.imagingFovUm`) or, per region, from its scan field size
//! and `SI.objectiveResolution`; per-plane times from each page's
//! `frameTimestamps_sec`. Pixel sizes of a millimetre or more come from
//! uncalibrated rigs and are left out.
//!
//! Provenance: ScanImage BigTiff Specification (vendor documentation,
//! `doc/scanimage/`); tifffile (BSD-3-Clause, see THIRD_PARTY_NOTICES.md)
//! for the static header; own analysis of the CatalystNeuro ScanImage test
//! files (GIN `ophys_testing_data`) and their README. No GPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::path::Path;

use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::{Samples, TiffImage};
use super::tiff::{original_meta, resolution};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta, Position, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::ScanImage, "ScanImage TIFF", "1").extensions(&["tif", "tiff"]);

/// Magic number of the format 3 / 4 static header.
const MAGIC: u32 = 117_637_889;
/// Largest settings or ROI block read.
const MAX_TEXT: u64 = 64 << 20;

/// Returns `true` for a ScanImage file, from its first bytes and the first
/// page's Software and ImageDescription tags.
pub(super) fn matches(head: &[u8], software: &str, description: &str) -> bool {
    let header = head.len() >= 24 && head.starts_with(b"II+\0") && u32::from_le_bytes([head[16], head[17], head[18], head[19]]) == MAGIC;
    header
        || software.starts_with("SI.")
        || description.starts_with("state.")
        || description.contains("scanimage.SI")
        || (description.contains("\"FrameHeader\"") && description.contains("\"TIFF_FORMAT_VERSION\""))
}

// --- Settings ---------------------------------------------------------------

/// Acquisition settings as `key -> MATLAB value text`, keys without the
/// `scanimage.` prefix (`SI.*`, `SI4.*`, `state.*`).
#[derive(Debug, Default)]
struct Settings {
    pairs: Vec<(String, String)>,
    index: HashMap<String, usize>,
}

impl Settings {
    fn insert(&mut self, key: &str, value: &str) {
        let key = key.strip_prefix("scanimage.").unwrap_or(key);
        if let Some(&i) = self.index.get(key) {
            self.pairs[i].1 = value.to_string();
        } else {
            self.index.insert(key.to_string(), self.pairs.len());
            self.pairs.push((key.to_string(), value.to_string()));
        }
    }

    /// `key = value` (or `key=value`) lines.
    fn parse_lines(&mut self, text: &str) {
        for line in text.split(['\n', '\r']) {
            let Some((k, v)) = line.split_once('=') else { continue };
            let k = k.trim();
            if k.starts_with("SI") || k.starts_with("scanimage.") || k.starts_with("state.") {
                self.insert(k, v.trim());
            }
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.index.get(key).map(|&i| self.pairs[i].1.as_str())
    }

    fn has(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// A number; `Inf`, `true` and `false` included.
    fn num(&self, key: &str) -> Option<f64> {
        parse_num(self.get(key)?)
    }

    /// A positive whole number.
    fn count(&self, key: &str) -> Option<u64> {
        self.num(key).filter(|v| v.is_finite() && *v >= 1.0 && *v < 1e12).map(|v| v.round() as u64)
    }

    fn flag(&self, key: &str) -> bool {
        self.num(key).is_some_and(|v| v != 0.0)
    }

    /// Numbers of a vector or matrix (`[1;2]`, `[1 2]`, `3`).
    fn list(&self, key: &str) -> Vec<f64> {
        self.get(key).map(parse_list).unwrap_or_default()
    }

    /// A string without its quotes.
    fn text(&self, key: &str) -> Option<&str> {
        let v = self.get(key)?.trim();
        Some(v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')).unwrap_or(v))
    }

    /// Strings of a cell array (`{'a' 'b'}`).
    fn strings(&self, key: &str) -> Vec<String> {
        self.get(key).map(|v| v.split('\'').skip(1).step_by(2).map(String::from).collect()).unwrap_or_default()
    }
}

fn parse_num(v: &str) -> Option<f64> {
    let v = v.trim().trim_matches(['[', ']']).trim();
    match v {
        "Inf" | "inf" | "_Inf_" => Some(f64::INFINITY),
        "true" => Some(1.0),
        "false" => Some(0.0),
        _ => v.parse().ok(),
    }
}

fn parse_list(v: &str) -> Vec<f64> {
    v.trim().trim_matches(['[', ']']).split([' ', ';', ',', '\t']).filter(|s| !s.is_empty()).filter_map(parse_num).collect()
}

/// Flattens the JSON-like settings of format 2 (numbers in arrays may be
/// separated by spaces, `_Inf_` / `_NaN_` strings) into `SI.a.b` keys with
/// MATLAB value text: numbers as written, strings quoted, arrays of numbers
/// as `[a b]`, arrays of strings as `{'a' 'b'}`.
fn flatten_json(text: &str, settings: &mut Settings) {
    struct P<'a> {
        b: &'a [u8],
        i: usize,
    }
    impl P<'_> {
        fn ws(&mut self) {
            while self.i < self.b.len() && (self.b[self.i].is_ascii_whitespace() || self.b[self.i] == b',') {
                self.i += 1;
            }
        }
        fn string(&mut self) -> String {
            self.i += 1;
            let start = self.i;
            while self.i < self.b.len() && self.b[self.i] != b'"' {
                self.i += if self.b[self.i] == b'\\' { 2 } else { 1 };
            }
            let s = String::from_utf8_lossy(&self.b[start..self.i.min(self.b.len())]).into_owned();
            self.i += 1;
            s
        }
        fn scalar(&mut self) -> String {
            let start = self.i;
            while self.i < self.b.len() && !b",]}\n\r\t ".contains(&self.b[self.i]) {
                self.i += 1;
            }
            String::from_utf8_lossy(&self.b[start..self.i]).into_owned()
        }
        /// One value; objects are flattened into `out` under `key`.
        fn value(&mut self, key: &str, depth: u32, out: &mut Settings) -> Option<String> {
            self.ws();
            if depth > 32 || self.i >= self.b.len() {
                self.i = self.b.len();
                return None;
            }
            match self.b[self.i] {
                b'{' => {
                    self.i += 1;
                    loop {
                        self.ws();
                        if self.i >= self.b.len() || self.b[self.i] == b'}' {
                            self.i += 1;
                            return None;
                        }
                        if self.b[self.i] != b'"' {
                            self.i += 1;
                            continue;
                        }
                        let name = self.string();
                        self.ws();
                        if self.b.get(self.i) == Some(&b':') {
                            self.i += 1;
                        }
                        let child = if key.is_empty() { name } else { format!("{key}.{name}") };
                        if let Some(v) = self.value(&child, depth + 1, out) {
                            out.insert(&child, &v);
                        }
                    }
                }
                b'[' => {
                    self.i += 1;
                    let (mut nums, mut strs) = (Vec::new(), Vec::new());
                    loop {
                        self.ws();
                        if self.i >= self.b.len() || self.b[self.i] == b']' {
                            self.i += 1;
                            break;
                        }
                        match self.b[self.i] {
                            b'"' => strs.push(self.string()),
                            b'[' | b'{' => {
                                // Nested arrays (matrices) as rows.
                                let before = self.i;
                                if let Some(v) = self.value("", depth + 1, &mut Settings::default()) {
                                    nums.push(v.trim_matches(['[', ']']).to_string());
                                }
                                if self.i == before {
                                    self.i += 1;
                                }
                            }
                            _ => {
                                let s = self.scalar();
                                if s.is_empty() {
                                    self.i += 1;
                                } else {
                                    nums.push(s);
                                }
                            }
                        }
                    }
                    Some(if !strs.is_empty() {
                        format!("{{{}}}", strs.iter().map(|s| format!("'{s}'")).collect::<Vec<_>>().join(" "))
                    } else {
                        format!("[{}]", nums.join(" "))
                    })
                }
                b'"' => Some(format!("'{}'", self.string())),
                _ => Some(self.scalar()),
            }
        }
    }
    let mut p = P { b: text.as_bytes(), i: 0 };
    p.value("", 0, settings);
}

/// Region of interest of a multi-ROI page.
#[derive(Debug, Clone)]
struct Roi {
    name: String,
    width: u32,
    height: u32,
    /// Scan field size and centre in scan angle degrees.
    size_deg: Option<(f64, f64)>,
    center_deg: Option<(f64, f64)>,
}

/// Imaging ROIs from the ROI group JSON.
fn parse_rois(json: &str) -> Vec<Roi> {
    use serde_json::Value;
    let Ok(v) = serde_json::from_str::<Value>(json) else { return Vec::new() };
    let group = v.pointer("/RoiGroups/imagingRoiGroup").or_else(|| v.pointer("/imagingRoiGroup"));
    let Some(rois) = group.and_then(|g| g.get("rois")) else { return Vec::new() };
    let list: Vec<&Value> = match rois {
        Value::Array(a) => a.iter().collect(),
        Value::Object(_) => vec![rois],
        _ => Vec::new(),
    };
    let pair = |v: Option<&Value>| -> Option<(f64, f64)> {
        let a = v?.as_array()?;
        Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
    };
    list.into_iter()
        .filter(|r| r.get("enable").is_none_or(|e| e.as_f64() != Some(0.0) && e.as_bool() != Some(false)))
        .filter_map(|r| {
            let sf = match r.get("scanfields")? {
                Value::Array(a) => a.first()?,
                s => s,
            };
            let (w, h) = pair(sf.get("pixelResolutionXY"))?;
            (w >= 1.0 && h >= 1.0 && w < 1e6 && h < 1e6).then(|| Roi {
                name: r.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
                width: w as u32,
                height: h as u32,
                size_deg: pair(sf.get("sizeXY")),
                center_deg: pair(sf.get("centerXY")),
            })
        })
        .collect()
}

/// Reads the settings, the ROI group JSON and the TIFF format version.
fn read_settings(tiff: &TiffFile, first_description: &str) -> Result<(Settings, Option<String>, String)> {
    let mut s = Settings::default();
    let mut head = [0u8; 32];
    if tiff.big && tiff.little && tiff.len >= 32 {
        crate::io::read_exact_at(&tiff.file, &mut head, 0)?;
    }
    let le = |o: usize| u32::from_le_bytes([head[o], head[o + 1], head[o + 2], head[o + 3]]) as u64;
    if tiff.big && le(16) == MAGIC as u64 {
        let (version, l0, l1) = (le(20), le(24), le(28));
        if l0 > MAX_TEXT || l1 > MAX_TEXT || 32 + l0 + l1 > tiff.len {
            return Err(Error::Corrupt("ScanImage header extends beyond the end of the file".into()));
        }
        let text = tiff.read_bytes(32, l0 as usize)?;
        s.parse_lines(&String::from_utf8_lossy(&text));
        let rois = (l1 > 1).then(|| tiff.read_bytes(32 + l0, l1 as usize)).transpose()?;
        let rois = rois.map(|r| String::from_utf8_lossy(&r).trim_end_matches('\0').to_string());
        return Ok((s, rois, version.to_string()));
    }
    if first_description.trim_start().starts_with('{') {
        // Format 2: settings in the first page, ROI groups after the last
        // page followed by their length.
        flatten_json(first_description, &mut s);
        let mut rois = None;
        if tiff.len > 8 {
            let tail = tiff.read_bytes(tiff.len - 4, 4)?;
            let n = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]) as u64;
            if n > 1 && n <= MAX_TEXT && n + 4 < tiff.len {
                let text = tiff.read_bytes(tiff.len - 4 - n, n as usize)?;
                let text = String::from_utf8_lossy(&text).trim_end_matches('\0').to_string();
                if text.trim_start().starts_with('{') {
                    rois = Some(text);
                }
            }
        }
        return Ok((s, rois, "2".into()));
    }
    s.parse_lines(first_description);
    let version = s.text("SI.TIFF_FORMAT_VERSION").map(String::from).unwrap_or_else(|| "-".into());
    Ok((s, None, version))
}

// --- Layout -----------------------------------------------------------------

/// How the pages map to channels, slices, frames and volumes: page
/// `((v * slices_all + z) * frames + f) * channels + c`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    channels: u32,
    /// Slices per volume as stored, flyback frames included.
    slices_all: u64,
    /// Slices returned (the first `slices_keep` of each volume).
    slices_keep: u64,
    /// Saved frames per slice.
    frames: u64,
    volumes: u64,
}

impl Layout {
    fn size_t(&self) -> u64 {
        self.volumes * self.frames
    }

    fn page(&self, c: u32, z: u32, t: u32) -> u64 {
        let (v, f) = (t as u64 / self.frames, t as u64 % self.frames);
        ((v * self.slices_all + z as u64) * self.frames + f) * self.channels as u64 + c as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Stack {
    Planar,
    /// Fast z: slices per volume with and without flyback frames.
    Fast {
        all: u64,
        keep: u64,
    },
    /// Motor stack: slices and saved frames per slice.
    Slow {
        slices: u64,
        frames: u64,
    },
}

/// Channel numbers saved, stack kind, frame period and z step (µm) from
/// the settings.
fn interpret(s: &Settings) -> (Vec<u32>, Stack) {
    let to_channels = |v: Vec<f64>| -> Vec<u32> { v.into_iter().filter(|c| c.is_finite() && *c >= 1.0 && *c < 1e4).map(|c| c as u32).collect() };
    if s.has("SI4.channelsSave") {
        let channels = to_channels(s.list("SI4.channelsSave"));
        let slices = s.count("SI4.stackNumSlices").unwrap_or(1);
        let stack = if s.flag("SI4.fastZEnable") {
            let discard = if s.flag("SI4.fastZDiscardFlybackFrames") { s.count("SI4.fastZNumDiscardFrames").unwrap_or(0) } else { 0 };
            Stack::Fast { all: slices + discard, keep: slices }
        } else if slices > 1 {
            let frames = s.count("SI4.acqNumFrames").unwrap_or(1) / s.count("SI4.acqNumAveragedFrames").unwrap_or(1).max(1);
            Stack::Slow { slices, frames: frames.max(1) }
        } else {
            Stack::Planar
        };
        return (channels, stack);
    }
    if s.has("state.acq.numberOfZSlices") || s.has("state.acq.savingChannel1") {
        let channels: Vec<u32> = (1..=8).filter(|c| s.flag(&format!("state.acq.savingChannel{c}"))).collect();
        let slices = s.count("state.acq.numberOfZSlices").unwrap_or(1);
        let frames = if s.flag("state.acq.averaging") { 1 } else { s.count("state.acq.numberOfFrames").unwrap_or(1) };
        let stack = if slices > 1 { Stack::Slow { slices, frames } } else { Stack::Planar };
        return (channels, stack);
    }
    let channels = to_channels(s.list("SI.hChannels.channelSave"));
    let avg = s.count("SI.hScan2D.logAverageFactor").unwrap_or(1).max(1);
    let frames =
        s.num("SI.hStackManager.framesPerSlice").filter(|v| v.is_finite() && *v >= 1.0 && *v < 1e12).map(|v| (v as u64 / avg).max(1)).unwrap_or(1);
    let stack = if s.has("SI.hStackManager.enable") {
        // Format 4.
        if !s.flag("SI.hStackManager.enable") {
            Stack::Planar
        } else if s.text("SI.hStackManager.stackMode") == Some("fast") {
            let keep = s.count("SI.hStackManager.numFramesPerVolume").or(s.count("SI.hStackManager.actualNumSlices")).unwrap_or(1);
            let all = s.count("SI.hStackManager.numFramesPerVolumeWithFlyback").unwrap_or(keep).max(keep);
            Stack::Fast { all, keep }
        } else {
            let slices = s.count("SI.hStackManager.actualNumSlices").or(s.count("SI.hStackManager.numSlices")).unwrap_or(1);
            Stack::Slow { slices, frames }
        }
    } else if s.flag("SI.hFastZ.enable") {
        let all = s.count("SI.hFastZ.numFramesPerVolume").or(s.count("SI.hStackManager.numSlices")).unwrap_or(1);
        let discard = if s.flag("SI.hFastZ.discardFlybackFrames") { s.count("SI.hFastZ.numDiscardFlybackFrames").unwrap_or(0) } else { 0 };
        Stack::Fast { all, keep: all.saturating_sub(discard).max(1) }
    } else if s.count("SI.hStackManager.numSlices").unwrap_or(1) > 1 {
        Stack::Slow { slices: s.count("SI.hStackManager.numSlices").unwrap_or(1), frames }
    } else {
        Stack::Planar
    };
    (channels, stack)
}

/// Fits the stack to the pages present.
fn layout(channels: u32, stack: Stack, pages: u64) -> Layout {
    let channels = channels.max(1);
    let (all, keep, frames) = match stack {
        Stack::Planar => (1, 1, 1),
        Stack::Fast { all, keep } => (all, keep, 1),
        Stack::Slow { slices, frames } => (slices, slices, frames),
    };
    let volume = all.saturating_mul(frames).saturating_mul(channels as u64);
    if volume > 0 && pages >= volume {
        return Layout { channels, slices_all: all, slices_keep: keep, frames, volumes: pages / volume };
    }
    // Less than one volume: its complete slices, else its frames.
    let slices = pages / (frames * channels as u64).max(1);
    if slices > 0 {
        return Layout { channels, slices_all: all, slices_keep: keep.min(slices), frames, volumes: 1 };
    }
    Layout { channels, slices_all: 1, slices_keep: 1, frames: (pages / channels as u64).max(1), volumes: 1 }
}

/// ISO 8601 from `epoch = [2016 5 30 18 56 2.286]`.
fn epoch_date(description: &str) -> Option<String> {
    let line = description.lines().find(|l| l.trim_start().trim_start_matches('"').starts_with("epoch"))?;
    let v = parse_list(line.split_once([':', '='])?.1.trim().trim_end_matches(','));
    let [y, mo, d, h, mi, sec] = v[..] else { return None };
    let valid = (1.0..=12.0).contains(&mo)
        && (1.0..=31.0).contains(&d)
        && (0.0..24.0).contains(&h)
        && (0.0..60.0).contains(&mi)
        && (0.0..61.0).contains(&sec);
    valid.then(|| {
        let ms = ((sec - sec.floor()) * 1000.0).round() as u32;
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{ms:03}", y as u32, mo as u32, d as u32, h as u32, mi as u32, sec.floor() as u32)
    })
}

/// `frameTimestamps_sec` of a page description.
fn frame_time(description: &str) -> Option<f64> {
    let at = description.find("frameTimestamps_sec")?;
    let rest = description[at + "frameTimestamps_sec".len()..].trim_start_matches(['"', ' ', ':', '=']);
    let end = rest.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'e' | 'E' | '+'))).unwrap_or(rest.len());
    rest[..end].parse().ok().filter(|v: &f64| v.is_finite())
}

// --- Reader -----------------------------------------------------------------

/// One series: a region of interest (rows `y0..y0 + height` of each page)
/// or the whole page.
#[derive(Debug)]
struct Part {
    y0: u32,
}

/// Reader for ScanImage TIFF files.
#[derive(Debug)]
pub(super) struct ScanImage {
    tiff: TiffFile,
    layout: Layout,
    /// Page geometry and sample type, from the first page.
    width: u32,
    height: u32,
    metas: Vec<SeriesMeta>,
    parts: Vec<Part>,
    /// Depth of each returned slice in µm, for arbitrary stacks.
    zs: Vec<f64>,
    original: OriginalMeta,
}

impl ScanImage {
    /// Opens a ScanImage file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for files without frames (an acquisition
    /// stopped before its first frame) or pages of differing layout.
    pub fn open(path: &Path) -> Result<Self> {
        let tiff = TiffFile::open(path)?;
        let desc0 = tiff.string(&tiff.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
        let (settings, roi_json, tiff_version) = read_settings(&tiff, &desc0)?;
        let first = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if first.samples_out != 1 || first.spp != 1 {
            return Err(Error::UnsupportedVariant("ScanImage TIFF with more than one sample per pixel".into()));
        }
        let (mut channel_numbers, stack) = interpret(&settings);
        if channel_numbers.is_empty() {
            channel_numbers.push(1);
        }
        let pages = tiff.ifds.len() as u64;
        let layout = layout(channel_numbers.len() as u32, stack, pages);
        let (width, height) = (first.width, first.height);
        if layout.size_t() > u32::MAX as u64 || layout.slices_keep > u32::MAX as u64 {
            return Err(Error::Corrupt("ScanImage file with too many frames".into()));
        }

        // Multi-ROI pages: regions top to bottom with equal fly-to gaps.
        let rois = roi_json.as_deref().map(parse_rois).unwrap_or_default();
        let roi_parts = (rois.len() > 1)
            .then(|| {
                let sum: u64 = rois.iter().map(|r| r.height as u64).sum();
                let rest = (height as u64).checked_sub(sum)?;
                let gaps = rois.len() as u64 - 1;
                (rest % gaps == 0).then_some(rest / gaps)
            })
            .flatten();

        let obj = settings.num("SI.objectiveResolution").filter(|v| v.is_finite() && *v > 0.0);
        let fov = settings.list("SI.hRoiManager.imagingFovUm");
        let (ppl, lpf) = (settings.count("SI.hRoiManager.pixelsPerLine"), settings.count("SI.hRoiManager.linesPerFrame"));
        let fov_px = match (fov.len(), ppl, lpf) {
            (8, Some(ppl), Some(lpf)) => {
                // Corners [x y; x y; x y; x y] in row-major order.
                let (wx, hy) = ((fov[2] - fov[0]).abs(), (fov[5] - fov[3]).abs());
                (wx > 0.0 && hy > 0.0).then(|| (wx * 1e3 / ppl as f64, hy * 1e3 / lpf as f64))
            }
            _ => None,
        };
        // A single region of interest gives the pixel size when the field
        // of view is missing.
        let roi_px = |roi: &Roi| match (obj, roi.size_deg) {
            (Some(o), Some((sx, sy))) => Some((sx.abs() * o * 1e3 / roi.width as f64, sy.abs() * o * 1e3 / roi.height as f64)),
            _ => None,
        };
        let fov_px = fov_px.or_else(|| (rois.len() == 1).then(|| roi_px(&rois[0])).flatten());
        // Releases that write the TIFF resolution in centimetres give the
        // scan's pixel size there (older ones write a 72 dpi placeholder);
        // it stays right when a file's pages were cropped.
        let tag_px = if tiff.uint(&tiff.ifds[0], tag::RESOLUTION_UNIT)? == Some(3) {
            let r = resolution(&tiff, &tiff.ifds[0])?;
            r.x_nm.zip(r.y_nm)
        } else {
            None
        };
        // Uncalibrated rigs write objective resolutions that give pixels of
        // millimetres; no laser scanning microscope images such pixels.
        let plausible = |p: Option<(f64, f64)>| p.filter(|(x, y)| *x > 0.0 && *y > 0.0 && *x < 1e6 && *y < 1e6);

        // Z step (µm) and per-slice depths of arbitrary stacks.
        let arbitrary = settings.text("SI.hStackManager.stackDefinition") == Some("arbitrary");
        let zs: Vec<f64> = if arbitrary { settings.list("SI.hStackManager.arbitraryZs") } else { Vec::new() };
        let zs = if zs.len() as u64 >= layout.slices_keep { zs[..layout.slices_keep as usize].to_vec() } else { Vec::new() };
        let step = ["SI.hStackManager.actualStackZStepSize", "SI.hStackManager.stackZStepSize", "SI4.stackZStepSize", "state.acq.zStepSize"]
            .iter()
            .find_map(|k| settings.num(k))
            .filter(|v| v.is_finite() && *v != 0.0)
            .map(|v| v.abs() * 1e3);
        let z_nm = if layout.slices_keep > 1 && !arbitrary { step } else { None };

        // Time between time points.
        let avg = settings.count("SI.hScan2D.logAverageFactor").unwrap_or(1) as f64;
        let period = settings.num("SI.hRoiManager.scanFramePeriod").or(settings.num("SI4.scanFramePeriod")).filter(|v| v.is_finite() && *v > 0.0);
        let period = period.or_else(|| settings.num("state.acq.frameRate").filter(|v| v.is_finite() && *v > 0.0).map(|r| 1.0 / r));
        let time_increment_s = match stack {
            Stack::Fast { .. } if layout.volumes > 1 => settings
                .num("SI.hRoiManager.scanVolumeRate")
                .filter(|v| v.is_finite() && *v > 0.0)
                .map(|r| 1.0 / r)
                .or(period.map(|p| p * layout.slices_all as f64)),
            Stack::Fast { .. } => None,
            _ => period.map(|p| p * avg),
        }
        .filter(|_| layout.size_t() > 1);

        let names = settings.strings("SI.hChannels.channelName");
        let channels: Vec<ChannelMeta> = channel_numbers
            .iter()
            .map(|&n| ChannelMeta { name: Some(names.get(n as usize - 1).cloned().unwrap_or_else(|| format!("Channel {n}"))), ..Default::default() })
            .collect();
        let date = epoch_date(&desc0).or_else(|| {
            // 4.x: '17-04-2015 14:35:56.050'.
            let t = settings.text("SI4.triggerClockTimeFirst")?;
            let (d, time) = t.split_once(' ')?;
            let p: Vec<&str> = d.split('-').collect();
            (p.len() == 3 && time.len() >= 8).then(|| format!("{}-{}-{}T{}", p[2], p[1], p[0], time))
        });
        let base = SeriesMeta {
            acquisition_date: date,
            size_c: layout.channels,
            size_z: layout.slices_keep as u32,
            size_t: layout.size_t() as u32,
            pixel_type: first.pixel_type,
            samples_per_pixel: 1,
            time_increment_s,
            channels,
            ..Default::default()
        };
        let level = |w: u32, h: u32| LevelMeta { size_x: w as u64, size_y: h as u64, tile: None, downsample: 1.0 };
        let (mut metas, mut parts) = (Vec::new(), Vec::new());
        match roi_parts {
            Some(gap) => {
                let mut y0 = 0u32;
                for (i, roi) in rois.iter().enumerate() {
                    let unique = !roi.name.is_empty() && rois.iter().filter(|r| r.name == roi.name).count() == 1;
                    let px = plausible(tag_px.or(roi_px(roi)));
                    metas.push(SeriesMeta {
                        name: Some(if unique { roi.name.clone() } else { format!("ROI {}", i + 1) }),
                        levels: vec![level(width, roi.height)],
                        physical_size: PhysicalSize { x_nm: px.map(|p| p.0), y_nm: px.map(|p| p.1), z_nm },
                        stage_position: match (obj, roi.center_deg) {
                            (Some(o), Some((cx, cy))) => Some(Position { x_nm: Some(cx * o * 1e3), y_nm: Some(cy * o * 1e3), z_nm: None }),
                            _ => None,
                        },
                        ..base.clone()
                    });
                    parts.push(Part { y0 });
                    y0 += roi.height + gap as u32;
                }
            }
            None => {
                metas.push(SeriesMeta {
                    levels: vec![level(width, height)],
                    physical_size: PhysicalSize {
                        x_nm: plausible(tag_px.or(fov_px)).map(|p| p.0),
                        y_nm: plausible(tag_px.or(fov_px)).map(|p| p.1),
                        z_nm,
                    },
                    ..base
                });
                parts.push(Part { y0: 0 });
            }
        }

        let mut original = OriginalMeta { pairs: original_meta(&tiff, &tiff.ifds[0])?, documents: Vec::new() };
        original.pairs.push(("ScanImage.TiffFormatVersion".into(), tiff_version));
        original.pairs.push(("ScanImage.Pages".into(), pages.to_string()));
        if layout.slices_all > layout.slices_keep {
            original.pairs.push(("ScanImage.FlybackFramesSkipped".into(), (layout.slices_all - layout.slices_keep).to_string()));
        }
        original.pairs.extend(settings.pairs);
        if let Some(r) = roi_json {
            original.documents.push(("RoiGroups".into(), r));
        }
        original.documents.push(("ImageDescription".into(), desc0));
        Ok(ScanImage { tiff, layout, width, height, metas, parts, zs, original })
    }
}

impl Backend for ScanImage {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        let page = self.layout.page(plane.c, plane.z, plane.t) as usize;
        let delta_t_s = match self.tiff.ifds.get(page) {
            Some(ifd) => self.tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.as_deref().and_then(frame_time),
            None => None,
        };
        let position = self.zs.get(plane.z as usize).map(|z| Position { x_nm: None, y_nm: None, z_nm: Some(z * 1e3) });
        Ok(PlaneMeta { delta_t_s, position, ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let page = self.layout.page(plane.c, plane.z, plane.t) as usize;
        let ifd = self.tiff.ifds.get(page).ok_or_else(|| Error::Corrupt(format!("ScanImage page {page} missing")))?;
        let img = TiffImage::new(&self.tiff, ifd)?;
        let m = &self.metas[series];
        if img.width != self.width || img.height != self.height || img.pixel_type != m.pixel_type || img.samples_out != 1 {
            return Err(Error::Corrupt(format!("ScanImage page {page} differs from the first page")));
        }
        let region = Region::new(region.x, region.y + self.parts[series].y0 as u64, region.width, region.height);
        img.read_native(&self.tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(text: &str) -> Settings {
        let mut s = Settings::default();
        s.parse_lines(text);
        s
    }

    #[test]
    fn parses_matlab_values() {
        let s = settings("SI.hChannels.channelSave = [1;3]\nscanimage.SI.a = Inf\nSI.b = {'Channel 1' 'Channel 2'}\nstate.acq.x=5\r");
        assert_eq!(s.list("SI.hChannels.channelSave"), [1.0, 3.0]);
        assert_eq!(s.num("SI.a"), Some(f64::INFINITY));
        assert_eq!(s.strings("SI.b"), ["Channel 1", "Channel 2"]);
        assert_eq!(s.count("state.acq.x"), Some(5));
        assert_eq!(parse_list("[1 2 3]"), [1.0, 2.0, 3.0]);
        assert_eq!(parse_list("2"), [2.0]);
    }

    #[test]
    fn flattens_format_2_json() {
        let mut s = Settings::default();
        flatten_json(
            "{\"FrameHeader\": {\"epoch\": [2016  5 15 17  2 56.927]},\n\"SI\": {\"hChannels\": {\"channelSave\": [1,2], \"channelName\": [\"Channel 1\", \"Channel 2\"]}, \"x\": \"_Inf_\", \"fov\": [[-1,1],[2,3]]}}",
            &mut s,
        );
        assert_eq!(s.list("SI.hChannels.channelSave"), [1.0, 2.0]);
        assert_eq!(s.strings("SI.hChannels.channelName"), ["Channel 1", "Channel 2"]);
        assert_eq!(s.text("SI.x"), Some("_Inf_"));
        assert_eq!(s.list("SI.fov"), [-1.0, 1.0, 2.0, 3.0]);
        assert_eq!(s.list("FrameHeader.epoch"), [2016.0, 5.0, 15.0, 17.0, 2.0, 56.927]);
    }

    #[test]
    fn maps_pages_of_each_stack_kind() {
        // Fast z: 9 slices + 7 flyback frames, 2 channels, 96 pages.
        let l = layout(2, Stack::Fast { all: 16, keep: 9 }, 96);
        assert_eq!((l.slices_keep, l.size_t()), (9, 3));
        assert_eq!(l.page(1, 8, 2), (2 * 16 + 8) * 2 + 1);
        // Slow stack: 2 slices, 2 frames each, 2 channels, 3 volumes.
        let l = layout(2, Stack::Slow { slices: 2, frames: 2 }, 24);
        assert_eq!((l.slices_keep, l.size_t()), (2, 6));
        assert_eq!(l.page(0, 1, 3), ((2 + 1) * 2 + 1) * 2);
        // Stopped after 7 of 102 slices (2 channels).
        let l = layout(2, Stack::Slow { slices: 102, frames: 1 }, 14);
        assert_eq!((l.slices_keep, l.size_t()), (7, 1));
        // Planar: pages over channels.
        let l = layout(3, Stack::Planar, 30);
        assert_eq!((l.slices_keep, l.size_t()), (1, 10));
        // Stopped inside the first slice: its frames.
        let l = layout(1, Stack::Slow { slices: 28, frames: 10 }, 4);
        assert_eq!((l.slices_keep, l.size_t()), (1, 4));
    }

    #[test]
    fn reads_frame_values() {
        assert_eq!(epoch_date("frameNumbers = 1\nepoch = [2016  5 30 18 56 2.286]\n").as_deref(), Some("2016-05-30T18:56:02.286"));
        assert_eq!(epoch_date("\t\t\"epoch\": [2016  5 15 17  2 56.927],").as_deref(), Some("2016-05-15T17:02:56.927"));
        assert_eq!(frame_time("frameNumbers = 3\nframeTimestamps_sec = 0.066666667\n"), Some(0.066666667));
        assert_eq!(frame_time("\"frameTimestamps_sec\": 1.5,"), Some(1.5));
    }
}

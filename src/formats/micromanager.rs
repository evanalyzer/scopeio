//! # micromanager
//!
//! Micro-Manager reader: image file stacks (`*_MMStack*.ome.tif`), separate
//! image files with `metadata.txt`, and NDTiff datasets.
//!
//! - **Image file stack**: TIFF files whose header is followed by the offset
//!   of an index map (header word 54773648 at byte 8) and the summary
//!   metadata (JSON, header word 2355492 at byte 32). The index map lists
//!   channel, slice, frame and position index and the IFD offset of every
//!   image, so the IFD chain is never walked. All files of one acquisition
//!   (`<prefix>_MMStack[_<position>][_<n>].ome.tif` in one folder) form one
//!   dataset; each position is one series. Opening the
//!   `<prefix>_MMStack_<position>_metadata.txt` companion opens the stack.
//! - **Separate image files**: one TIFF per plane and a `metadata.txt` (JSON)
//!   in each position folder, with `FrameKey-<t>-<c>-<z>` entries naming the
//!   file (Micro-Manager 1.4) or `Coords-<file>` entries (2.0). Sibling
//!   `Pos<n>` folders of the same acquisition are further series.
//! - **NDTiff** (v1 to v3): a folder (or its `Full resolution` subfolder)
//!   with `NDTiff.index`, which lists for each image its axes (JSON), file,
//!   pixel offset, size, pixel type and metadata location. The axes
//!   `channel`, `z` and `time` become C, Z and T; `position` and every other
//!   axis (e.g. Magellan `row` / `column` tiles) select the series. Values
//!   are ordered numerically (integers) or by first appearance (strings);
//!   an image written twice keeps the last entry. Downsampled pyramid
//!   folders are not read.
//!
//! Sizes come from the images present, so an aborted acquisition has as
//! many time points as were written; a missing image reads as zeros. Pixel
//! size, z step, interval, channel names and colours come from the summary
//! (`PixelSize_um`, `z-step_um`, `Interval_ms`, `ChNames`, `ChColors`), the
//! pixel size from the first image's metadata if the summary has none.
//! Per-plane time, exposure and stage position come from the image
//! metadata (`ElapsedTime-ms`, `Exposure-ms`, `X/Y/ZPositionUm`).
//!
//! Provenance: Micro-Manager file format page (micro-manager.org) and the
//! NDTiff specification (NDStorage README, BSD-3, both in
//! `doc/micro-manager/`); own analysis of the public sample datasets on the
//! OME sample server and Zenodo (CC BY 4.0); checked against Bio-Formats and
//! bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::io::{ReadAt, read_exact_at};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OpenOptions, OriginalMeta, PhysicalSize, PixelType, Plane, PlaneMeta,
    Position, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::MicroManager, "Micro-Manager", "1")
    .extensions(&["ome.tif", "tif", "tiff", "txt", "index"])
    .file_names(&["metadata.txt", "NDTiff.index"])
    .folder();

/// Header word before the index map offset of an image stack.
const INDEX_MAP_OFFSET_HEADER: u32 = 54_773_648;
/// First word of the index map.
const INDEX_MAP_HEADER: u32 = 3_453_623;
/// Header word before the summary metadata length.
const SUMMARY_HEADER: u32 = 2_355_492;
/// Word at byte 8 (v2, v3) or 24 (v1) of NDTiff files.
const NDTIFF_MAGIC: u32 = 483_729;
/// TIFF tag holding the image metadata (JSON).
const TAG_MM_METADATA: u16 = 51123;
const INDEX_NAME: &str = "NDTiff.index";

/// Largest JSON document parsed (summary, image metadata, `metadata.txt`).
const MAX_JSON: u64 = 1 << 30;
/// Most planes of one dataset, against corrupt indices.
const MAX_PLANES: usize = 1 << 26;

/// Returns `true` for a folder holding an NDTiff dataset.
pub(super) fn matches_dir(path: &Path) -> bool {
    ndtiff_dir(path).is_some()
}

/// Returns `true` for any file of a Micro-Manager dataset.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let name = file_name(path);
    if name == INDEX_NAME {
        return true;
    }
    if name.to_ascii_lowercase().ends_with("metadata.txt") {
        return is_summary_json(head);
    }
    if head.len() < 40 || !TiffFile::matches(head) || head[2] == 43 || head[3] == 43 {
        return false;
    }
    let ndtiff = word(head, 8) == NDTIFF_MAGIC || word(head, 24) == NDTIFF_MAGIC;
    if ndtiff && path.parent().is_some_and(|d| d.join(INDEX_NAME).is_file()) {
        return true;
    }
    // Image stacks and NDTiff v1 files carry an index map.
    if word(head, 8) == INDEX_MAP_OFFSET_HEADER {
        return has_index_map(path, head);
    }
    // A separate image file next to its metadata.txt.
    name.starts_with("img_") && separate_metadata(path).is_some()
}

/// `u32` at `at` in the byte order of the TIFF header in `head`.
fn word(head: &[u8], at: usize) -> u32 {
    let b: [u8; 4] = head[at..at + 4].try_into().expect("4 bytes");
    if head[0] == b'I' { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
}

/// Whether the image stack whose header is `head` points to a valid index map.
fn has_index_map(path: &Path, head: &[u8]) -> bool {
    let mut b = [0u8; 4];
    if File::open(path).and_then(|f| f.read_exact_at_io(&mut b, word(head, 12) as u64)).is_err() {
        return false;
    }
    let v = if head[0] == b'I' { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) };
    v == INDEX_MAP_HEADER
}

fn is_summary_json(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    text.trim_start().starts_with('{') && text.contains("\"Summary\"")
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The `metadata.txt` next to a separate image file, if it is Micro-Manager's.
fn separate_metadata(path: &Path) -> Option<PathBuf> {
    let meta = path.parent()?.join("metadata.txt");
    let mut head = Vec::with_capacity(512);
    std::io::Read::read_to_end(&mut std::io::Read::take(File::open(&meta).ok()?, 512), &mut head).ok()?;
    is_summary_json(&head).then_some(meta)
}

/// The folder holding `NDTiff.index` for a dataset folder (or its
/// `Full resolution` subfolder, NDTiff v2).
fn ndtiff_dir(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    [path.to_path_buf(), path.join("Full resolution")].into_iter().find(|d| d.join(INDEX_NAME).is_file())
}

// --- JSON helpers ------------------------------------------------------------

fn read_json(path: &Path) -> Result<Value> {
    if std::fs::metadata(path)?.len() > MAX_JSON {
        return Err(Error::Corrupt(format!("{} too large", path.display())));
    }
    parse_json(&std::fs::read(path)?).map_err(|e| Error::Corrupt(format!("{}: {e}", path.display())))
}

/// Parses JSON written by Micro-Manager; trailing NULs and padding are ignored.
fn parse_json(bytes: &[u8]) -> std::result::Result<Value, String> {
    let end = bytes.iter().rposition(|&b| b != 0 && !b.is_ascii_whitespace()).map_or(0, |i| i + 1);
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes[..end]);
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// A number, also when written as a string.
fn num(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|f: &f64| f.is_finite())
}

fn string(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Micro-Manager time "2016-01-31 08:01:46 +0100" (fraction optional) as
/// ISO 8601.
fn mm_date(s: &str) -> Option<String> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b' ' | b'T') || b[13] != b':' || !s.is_char_boundary(19) {
        return None;
    }
    let (date, rest) = s.split_at(10);
    let mut parts = rest[1..].split_whitespace();
    let time = parts.next()?;
    let zone = match parts.next() {
        Some(z) if z.len() == 5 && (z.starts_with('+') || z.starts_with('-')) => format!("{}:{}", &z[..3], &z[3..]),
        _ => String::new(),
    };
    Some(format!("{date}T{time}{zone}"))
}

/// ARGB colour as written by Java (`ChColors`) as RGBA.
fn java_color(v: &Value) -> Option<[u8; 4]> {
    let c = v.as_i64()? as u32;
    Some([(c >> 16) as u8, (c >> 8) as u8, c as u8, 255])
}

/// Per-plane values of one image's metadata.
fn plane_meta(v: &Value) -> PlaneMeta {
    let um = |k: &str| num(v, k).map(|x| x * 1e3);
    let position = Position { x_nm: um("XPositionUm"), y_nm: um("YPositionUm"), z_nm: um("ZPositionUm") };
    PlaneMeta {
        delta_t_s: num(v, "ElapsedTime-ms").map(|ms| ms / 1e3),
        exposure_s: num(v, "Exposure-ms").map(|ms| ms / 1e3),
        position: (position != Position::default()).then_some(position),
    }
}

/// Top-level scalar values of the summary, for the original metadata.
fn summary_pairs(summary: &Value) -> Vec<(String, String)> {
    let Some(obj) = summary.as_object() else { return Vec::new() };
    obj.iter()
        .filter_map(|(k, v)| {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Number(_) | Value::Bool(_) => v.to_string(),
                Value::Array(a) if a.iter().all(|x| !x.is_object() && !x.is_array()) => v.to_string(),
                _ => return None,
            };
            Some((k.clone(), s))
        })
        .collect()
}

/// Series metadata shared by all positions, from the summary.
fn template(summary: &Value) -> SeriesMeta {
    let positive = |k: &str| num(summary, k).filter(|v| *v > 0.0);
    let pixel = positive("PixelSize_um").map(|um| um * 1e3);
    let names: Vec<String> = summary.get("ChNames").and_then(Value::as_array).map(|a| a.iter().map(|n| n.as_str().map(str::to_string).unwrap_or_else(|| n.to_string())).collect()).unwrap_or_default();
    let colors: Vec<Option<[u8; 4]>> = summary.get("ChColors").and_then(Value::as_array).map(|a| a.iter().map(java_color).collect()).unwrap_or_default();
    let channels = (0..names.len().max(colors.len()))
        .map(|i| ChannelMeta {
            name: names.get(i).filter(|n| !n.is_empty()).cloned(),
            color: colors.get(i).copied().flatten(),
            ..Default::default()
        })
        .collect();
    SeriesMeta {
        acquisition_date: string(summary, "StartTime").or_else(|| string(summary, "Time")).and_then(|d| mm_date(&d)),
        physical_size: PhysicalSize { x_nm: pixel, y_nm: pixel, z_nm: positive("z-step_um").map(|um| um * 1e3) },
        time_increment_s: positive("Interval_ms").map(|ms| ms / 1e3),
        significant_bits: num(summary, "BitDepth").filter(|b| (1.0..64.0).contains(b)).map(|b| b as u8),
        channels,
        ..Default::default()
    }
}

/// Position labels of the summary (`InitialPositionList` in 1.4,
/// `StagePositions` in 2.0).
fn position_labels(summary: &Value) -> Vec<Option<String>> {
    ["InitialPositionList", "StagePositions"]
        .iter()
        .find_map(|k| summary.get(*k).and_then(Value::as_array))
        .map(|list| list.iter().map(|p| string(p, "Label")).collect())
        .unwrap_or_default()
}

// --- Dataset ------------------------------------------------------------------

/// Where one plane is stored.
#[derive(Debug, Clone, Copy)]
struct Loc {
    file: u32,
    /// Image stack: IFD offset; separate files: unused (first IFD);
    /// NDTiff: offset of the pixels.
    offset: u64,
    /// NDTiff: offset and length of the image metadata.
    meta: u64,
    meta_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Stack,
    Separate,
    NdTiff,
}

#[derive(Debug, Default)]
struct Series {
    /// Indexed by `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<Loc>>,
    /// Separate files: plane metadata from `metadata.txt`, same indexing.
    plane_meta: Vec<PlaneMeta>,
}

/// Micro-Manager dataset.
#[derive(Debug)]
pub(super) struct MicroManager {
    kind: Kind,
    files: Vec<PathBuf>,
    /// Open data files, if there are at most `OpenOptions::max_open_files`.
    handles: Option<Vec<OnceLock<std::result::Result<TiffFile, String>>>>,
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

/// Image dimensions shared by all planes.
#[derive(Debug, Clone, Copy)]
struct Geometry {
    width: u64,
    height: u64,
    pixel_type: PixelType,
    spp: u8,
}

/// Plane index of `(c, z, t)`; `None` beyond `sizes`.
fn index(sizes: (usize, usize, usize), c: usize, z: usize, t: usize) -> Option<usize> {
    (c < sizes.0 && z < sizes.1 && t < sizes.2).then(|| c + sizes.0 * (z + sizes.1 * t))
}

/// Checks the plane count of `series` positions with `sizes`.
fn check_planes(series: usize, sizes: (usize, usize, usize)) -> Result<usize> {
    let n = sizes.0.checked_mul(sizes.1).and_then(|n| n.checked_mul(sizes.2)).filter(|&n| n > 0 && n.saturating_mul(series) <= MAX_PLANES);
    n.ok_or_else(|| Error::Corrupt(format!("Micro-Manager dataset of {series} x {sizes:?} planes")))
}

impl MicroManager {
    /// Opens any file of a Micro-Manager dataset, or an NDTiff folder.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid metadata or indices and
    /// [`Error::UnsupportedVariant`] for NDTiff RGB or compressed images.
    pub fn open(path: &Path, opts: &OpenOptions) -> Result<Self> {
        let mut mm = Self::open_dataset(path)?;
        if mm.files.len() <= opts.max_open_files {
            mm.handles = Some(mm.files.iter().map(|_| OnceLock::new()).collect());
        }
        Ok(mm)
    }

    fn open_dataset(path: &Path) -> Result<Self> {
        if let Some(dir) = ndtiff_dir(path) {
            return Self::open_ndtiff(&dir);
        }
        let name = file_name(path);
        if name == INDEX_NAME {
            return Self::open_ndtiff(path.parent().unwrap_or(Path::new(".")));
        }
        if name.to_ascii_lowercase().ends_with("metadata.txt") {
            if let Some(stack) = stack_of_companion(path) {
                return Self::open_stack(&stack);
            }
            return Self::open_separate(path);
        }
        let mut head = [0u8; 32];
        read_exact_at(&File::open(path)?, &mut head, 0)?;
        if word(&head, 8) == NDTIFF_MAGIC || word(&head, 24) == NDTIFF_MAGIC {
            let dir = path.parent().unwrap_or(Path::new("."));
            if dir.join(INDEX_NAME).is_file() {
                return Self::open_ndtiff(dir);
            }
        }
        if word(&head, 8) == INDEX_MAP_OFFSET_HEADER {
            return Self::open_stack(path);
        }
        let meta = separate_metadata(path).ok_or_else(|| Error::UnsupportedFormat(path.display().to_string()))?;
        Self::open_separate(&meta)
    }

    fn new(kind: Kind, files: Vec<PathBuf>, series: Vec<Series>, metas: Vec<SeriesMeta>, original: OriginalMeta) -> Self {
        Self { kind, files, handles: None, series, metas, original }
    }

    /// Image file stacks: all files of the acquisition, one series per position.
    fn open_stack(path: &Path) -> Result<Self> {
        let files = stack_files(path);
        // (c, z, t, p) -> (file, IFD offset); the first file listing an image wins.
        let mut entries: Vec<([u32; 4], u32, u64)> = Vec::new();
        let mut summary = None;
        let mut first_tiff = None;
        let mut kept = Vec::new();
        for f in &files {
            let opened = TiffFile::from_file(File::open(f)?, 1).and_then(|t| stack_index(&t).map(|(s, e)| (t, s, e)));
            let (tiff, s, list) = match opened {
                Ok(v) => v,
                // Other files of the acquisition may be unreadable; the opened one may not.
                Err(e) if f.as_path() == path => return Err(e),
                Err(_) => continue,
            };
            let file = kept.len() as u32;
            kept.push(f.clone());
            if summary.is_none() {
                summary = s;
            }
            if first_tiff.is_none() {
                first_tiff = Some(tiff);
            }
            entries.extend(list.into_iter().map(|e| ([e[0], e[1], e[2], e[3]], file, e[4] as u64)));
        }
        let summary = summary.unwrap_or(Value::Null);
        let tiff = first_tiff.ok_or_else(|| Error::Corrupt("Micro-Manager stack without readable file".into()))?;
        if entries.is_empty() {
            return Err(Error::Corrupt("Micro-Manager index map without images".into()));
        }
        let max = |i: usize| entries.iter().map(|e| e.0[i] as usize).max().unwrap_or(0) + 1;
        let sizes = (max(0), max(1), max(2));
        let mut positions: Vec<u32> = entries.iter().map(|e| e.0[3]).collect();
        positions.sort_unstable();
        positions.dedup();
        let n = check_planes(positions.len(), sizes)?;
        let mut series: Vec<Series> = positions.iter().map(|_| Series { planes: vec![None; n], plane_meta: Vec::new() }).collect();
        for ([c, z, t, p], file, offset) in entries {
            let s = positions.binary_search(&p).expect("listed position");
            if let Some(i) = index(sizes, c as usize, z as usize, t as usize) {
                series[s].planes[i].get_or_insert(Loc { file, offset, meta: 0, meta_len: 0 });
            }
        }
        // Geometry from the first image.
        let first = series.iter().flat_map(|s| s.planes.iter()).flatten().next().expect("at least one image");
        let geometry = if first.file == 0 { stack_geometry(&tiff, first.offset)? } else {
            let t = TiffFile::from_file(File::open(&kept[first.file as usize])?, 1)?;
            stack_geometry(&t, first.offset)?
        };
        let labels = position_labels(&summary);
        let mut mm = Self::new(Kind::Stack, kept, series, Vec::new(), original(&summary));
        let names = positions.iter().map(|&p| labels.get(p as usize).cloned().flatten().unwrap_or_else(|| format!("Pos{p}"))).collect();
        mm.finish(&summary, sizes, geometry, names);
        Ok(mm)
    }

    /// Separate image files: one series per position folder.
    fn open_separate(path: &Path) -> Result<Self> {
        let folders = position_folders(path);
        let mut files: Vec<PathBuf> = Vec::new();
        let mut file_index: HashMap<PathBuf, u32> = HashMap::new();
        // Per folder: (c, z, t) -> (file, metadata).
        let mut folder_planes = Vec::new();
        let mut summary = Value::Null;
        // Pixel size of the first image that has one (2.0 writes none in the summary).
        let mut pixel_nm = None;
        for (k, meta_path) in folders.iter().enumerate() {
            let json = match read_json(meta_path) {
                Ok(j) => j,
                Err(e) if k == 0 => return Err(e),
                Err(_) => continue,
            };
            if summary.is_null() {
                summary = json.get("Summary").cloned().unwrap_or(Value::Null);
            }
            let dir = meta_path.parent().unwrap_or(Path::new("."));
            let mut planes = Vec::new();
            for (key, v) in json.as_object().into_iter().flatten() {
                let (coords, name, meta) = if let Some(frame) = key.strip_prefix("FrameKey-") {
                    // FrameKey-<t>-<c>-<z>
                    let n: Vec<usize> = frame.split('-').filter_map(|s| s.parse().ok()).collect();
                    let [t, c, z] = n[..] else { continue };
                    let Some(name) = string(v, "FileName") else { continue };
                    ((c, z, t), name, v)
                } else if let Some(name) = key.strip_prefix("Coords-") {
                    let get = |k: &str| v.get(k).and_then(Value::as_u64).unwrap_or(0) as usize;
                    let meta = json.get(format!("Metadata-{name}")).unwrap_or(&Value::Null);
                    ((get("channel"), get("z"), get("time")), name.to_string(), meta)
                } else {
                    continue;
                };
                // Only the file name: never leave the position folder.
                let Some(base) = Path::new(&name.replace('\\', "/")).file_name().map(|b| b.to_os_string()) else { continue };
                let file = dir.join(base);
                let next = files.len() as u32;
                let id = *file_index.entry(file.clone()).or_insert_with(|| {
                    files.push(file);
                    next
                });
                if pixel_nm.is_none() {
                    pixel_nm = ["PixelSizeUm", "PixelSize_um"].iter().find_map(|k| num(meta, k)).filter(|v| *v > 0.0).map(|um| um * 1e3);
                }
                planes.push((coords, id, plane_meta(meta)));
            }
            folder_planes.push((dir.file_name().map(|n| n.to_string_lossy().into_owned()), planes));
        }
        let all = folder_planes.iter().flat_map(|(_, p)| p.iter());
        let sizes = all.fold((1, 1, 1), |s, ((c, z, t), _, _)| (s.0.max(c + 1), s.1.max(z + 1), s.2.max(t + 1)));
        let n = check_planes(folder_planes.len(), sizes)?;
        if folder_planes.iter().all(|(_, p)| p.is_empty()) {
            return Err(Error::Corrupt("Micro-Manager metadata.txt without images".into()));
        }
        // Geometry from the first readable image.
        let geometry = files
            .iter()
            .find_map(|f| {
                let tiff = TiffFile::open_first(f).ok()?;
                let img = TiffImage::new(&tiff, &tiff.ifds[0]).ok()?;
                Some(Geometry { width: img.width as u64, height: img.height as u64, pixel_type: img.pixel_type, spp: img.samples_out })
            })
            .ok_or_else(|| Error::UnsupportedVariant("Micro-Manager dataset without readable image files".into()))?;
        let mut series = Vec::new();
        let mut names = Vec::new();
        for (folder, planes) in folder_planes {
            let mut s = Series { planes: vec![None; n], plane_meta: vec![PlaneMeta::default(); n] };
            for ((c, z, t), file, meta) in planes {
                if let Some(i) = index(sizes, c, z, t) {
                    s.planes[i] = Some(Loc { file, offset: 0, meta: 0, meta_len: 0 });
                    s.plane_meta[i] = meta;
                }
            }
            series.push(s);
            names.push(folder.unwrap_or_else(|| format!("Pos{}", names.len())));
        }
        let mut mm = Self::new(Kind::Separate, files, series, Vec::new(), original(&summary));
        mm.finish(&summary, sizes, geometry, names);
        for m in mm.metas.iter_mut().filter(|m| m.physical_size.x_nm.is_none()) {
            m.physical_size.x_nm = pixel_nm;
            m.physical_size.y_nm = pixel_nm;
        }
        Ok(mm)
    }

    /// NDTiff dataset in `dir`.
    fn open_ndtiff(dir: &Path) -> Result<Self> {
        let index_path = dir.join(INDEX_NAME);
        if std::fs::metadata(&index_path)?.len() > MAX_JSON {
            return Err(Error::Corrupt("NDTiff index too large".into()));
        }
        let entries = parse_ndtiff_index(&std::fs::read(&index_path)?);
        if entries.is_empty() {
            return Err(Error::Corrupt("NDTiff index without images".into()));
        }
        let first = &entries[0];
        let (pixel_type, bits) = match first.pixel_type {
            0 => (PixelType::U8, 8),
            1 => (PixelType::U16, 16),
            3 => (PixelType::U16, 10),
            4 => (PixelType::U16, 12),
            5 => (PixelType::U16, 14),
            2 => return Err(Error::UnsupportedVariant("NDTiff RGB images".into())),
            p => return Err(Error::Corrupt(format!("NDTiff pixel type {p}"))),
        };
        if first.compression != 0 {
            return Err(Error::UnsupportedVariant(format!("NDTiff compression {}", first.compression)));
        }
        let (width, height) = (first.width, first.height);
        // Axis values in sorted order; series keys are all axes besides C, Z, T.
        let mut axis_values: BTreeMap<String, Vec<AxisValue>> = BTreeMap::new();
        for e in &entries {
            for (k, v) in &e.axes {
                let list = axis_values.entry(k.clone()).or_default();
                if !list.contains(v) {
                    list.push(v.clone());
                }
            }
        }
        for list in axis_values.values_mut() {
            // Integers numerically, then strings in order of appearance.
            list.sort_by(|a, b| match (a, b) {
                (AxisValue::Int(x), AxisValue::Int(y)) => x.cmp(y),
                (AxisValue::Int(_), AxisValue::Str(_)) => std::cmp::Ordering::Less,
                (AxisValue::Str(_), AxisValue::Int(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            });
        }
        let rank = |axis: &str, v: Option<&AxisValue>| -> usize {
            v.and_then(|v| axis_values.get(axis)?.iter().position(|x| x == v)).unwrap_or(0)
        };
        let count = |axis: &str| axis_values.get(axis).map_or(1, Vec::len);
        let sizes = (count("channel"), count("z"), count("time"));
        let series_axes: Vec<&String> = axis_values.keys().filter(|k| !matches!(k.as_str(), "channel" | "z" | "time")).collect();
        let key_of = |e: &NdEntry| -> Vec<usize> { series_axes.iter().map(|a| rank(a, e.axes.get(*a))).collect() };
        let mut keys: Vec<Vec<usize>> = entries.iter().map(key_of).collect();
        keys.sort();
        keys.dedup();
        let n = check_planes(keys.len(), sizes)?;
        let mut files: Vec<PathBuf> = Vec::new();
        let mut file_index: HashMap<String, u32> = HashMap::new();
        let mut series: Vec<Series> = keys.iter().map(|_| Series { planes: vec![None; n], plane_meta: Vec::new() }).collect();
        for e in &entries {
            if (e.width, e.height, e.pixel_type, e.compression) != (width, height, first.pixel_type, 0) {
                continue;
            }
            // Only the file name: never leave the dataset folder.
            let Some(base) = Path::new(&e.file.replace('\\', "/")).file_name().map(|b| b.to_string_lossy().into_owned()) else { continue };
            let next = files.len() as u32;
            let file = *file_index.entry(base.clone()).or_insert_with(|| {
                files.push(dir.join(&base));
                next
            });
            let s = keys.binary_search(&key_of(e)).expect("listed key");
            let i = index(sizes, rank("channel", e.axes.get("channel")), rank("z", e.axes.get("z")), rank("time", e.axes.get("time"))).expect("ranks within sizes");
            // An image written again replaces the earlier one.
            series[s].planes[i] = Some(Loc { file, offset: e.offset, meta: e.meta, meta_len: e.meta_len });
        }
        let summary = files.first().and_then(|f| ndtiff_summary(f).ok()).unwrap_or(Value::Null);
        let names = keys
            .iter()
            .map(|key| {
                let parts: Vec<String> = series_axes.iter().zip(key).map(|(a, &r)| format!("{a} {}", axis_values[*a][r])).collect();
                if parts.is_empty() { dir_name(dir) } else { parts.join(", ") }
            })
            .collect();
        let geometry = Geometry { width: width as u64, height: height as u64, pixel_type, spp: 1 };
        let mut mm = Self::new(Kind::NdTiff, files, series, Vec::new(), original(&summary));
        mm.finish(&summary, sizes, geometry, names);
        if bits < 16 {
            for m in &mut mm.metas {
                m.significant_bits = Some(bits);
            }
        }
        // String channel values name the channels.
        if let Some(values) = axis_values.get("channel") {
            for m in &mut mm.metas {
                for (ch, v) in m.channels.iter_mut().zip(values) {
                    if let AxisValue::Str(s) = v {
                        ch.name = Some(s.clone());
                    }
                }
            }
        }
        Ok(mm)
    }

    /// Builds the series metadata.
    fn finish(&mut self, summary: &Value, sizes: (usize, usize, usize), g: Geometry, names: Vec<String>) {
        let mut base = template(summary);
        base.channels.resize(sizes.0, ChannelMeta::default());
        if base.significant_bits.is_some_and(|b| b as usize >= g.pixel_type.bytes() * 8) {
            base.significant_bits = None;
        }
        let meta = SeriesMeta {
            size_c: sizes.0 as u32,
            size_z: sizes.1 as u32,
            size_t: sizes.2 as u32,
            pixel_type: g.pixel_type,
            samples_per_pixel: g.spp,
            levels: vec![LevelMeta { size_x: g.width, size_y: g.height, tile: None, downsample: 1.0 }],
            ..base
        };
        self.metas = names.into_iter().map(|name| SeriesMeta { name: Some(name), ..meta.clone() }).collect();
        // Stage position and, if the summary has none, pixel size from each
        // series' first image.
        for s in 0..self.series.len() {
            let Some(first) = self.series[s].planes.iter().position(Option::is_some) else { continue };
            let Ok(json) = self.image_json(s, first) else { continue };
            let pm = match self.kind {
                Kind::Separate => self.series[s].plane_meta[first].clone(),
                _ => plane_meta(&json),
            };
            let m = &mut self.metas[s];
            m.stage_position = pm.position;
            if m.physical_size.x_nm.is_none() {
                let px = ["PixelSizeUm", "PixelSize_um"].iter().find_map(|k| num(&json, k)).filter(|v| *v > 0.0).map(|um| um * 1e3);
                m.physical_size.x_nm = px;
                m.physical_size.y_nm = px;
            }
        }
    }

    /// Metadata JSON of plane `index` of `series` (empty for separate files,
    /// whose metadata is in `metadata.txt`).
    fn image_json(&self, series: usize, index: usize) -> Result<Value> {
        let Some(loc) = self.series[series].planes.get(index).copied().flatten() else { return Ok(Value::Null) };
        let bytes = match self.kind {
            Kind::Separate => return Ok(Value::Null),
            Kind::Stack => self.with_file(loc.file, |tiff| {
                let (ifd, _) = tiff.read_ifd(loc.offset)?;
                ifd.entry(TAG_MM_METADATA).map_or(Ok(Vec::new()), |e| tiff.raw(e))
            })?,
            Kind::NdTiff => self.with_file(loc.file, |tiff| tiff.read_bytes(loc.meta, loc.meta_len as usize))?,
        };
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        parse_json(&bytes).map_err(|e| Error::Corrupt(format!("Micro-Manager image metadata: {e}")))
    }

    /// Calls `f` with data file `file`: the kept handle, or the file opened
    /// for this call.
    fn with_file<R>(&self, file: u32, f: impl FnOnce(&TiffFile) -> Result<R>) -> Result<R> {
        let path = &self.files[file as usize];
        let open = || File::open(path).map_err(Error::from).and_then(|h| TiffFile::from_file(h, 1));
        let Some(handles) = &self.handles else { return f(&open()?) };
        let h = handles[file as usize].get_or_init(|| open().map_err(|e| e.to_string()));
        f(h.as_ref().map_err(|e| Error::Corrupt(format!("{}: {e}", path.display())))?)
    }
}

/// Reads the summary and index map of one image stack file. Entries are
/// `[c, z, t, position, IFD offset]`.
fn stack_index(tiff: &TiffFile) -> Result<(Option<Value>, Vec<[u32; 5]>)> {
    if tiff.big {
        return Err(Error::UnsupportedVariant("Micro-Manager stack as BigTIFF".into()));
    }
    let head = tiff.read_bytes(0, 40)?;
    let w = |at: usize| tiff.u32(&head[at..]);
    if w(8) != INDEX_MAP_OFFSET_HEADER {
        return Err(Error::Corrupt("Micro-Manager stack without index map header".into()));
    }
    let summary = (w(32) == SUMMARY_HEADER && (w(36) as u64) <= MAX_JSON.min(tiff.len))
        .then(|| tiff.read_bytes(40, w(36) as usize).ok().and_then(|b| parse_json(&b).ok()))
        .flatten();
    let at = w(12) as u64;
    let map = tiff.read_bytes(at, 8)?;
    if tiff.u32(&map) != INDEX_MAP_HEADER {
        return Err(Error::Corrupt("Micro-Manager index map header missing".into()));
    }
    let n = tiff.u32(&map[4..]) as u64;
    if n.checked_mul(20).is_none_or(|len| at + 8 + len > tiff.len) {
        return Err(Error::Corrupt(format!("Micro-Manager index map of {n} entries")));
    }
    let raw = tiff.read_bytes(at + 8, n as usize * 20)?;
    let entries = raw
        .chunks_exact(20)
        .map(|e| std::array::from_fn(|i| tiff.u32(&e[4 * i..])))
        // An entry without IFD (offset 0) is a slot that was never written.
        .filter(|e: &[u32; 5]| e[4] != 0)
        .collect();
    Ok((summary, entries))
}

fn stack_geometry(tiff: &TiffFile, ifd_offset: u64) -> Result<Geometry> {
    let (ifd, _) = tiff.read_ifd(ifd_offset)?;
    let img = TiffImage::new(tiff, &ifd)?;
    Ok(Geometry { width: img.width as u64, height: img.height as u64, pixel_type: img.pixel_type, spp: img.samples_out })
}

/// All files of the image stack acquisition of `path`, sorted: files in the
/// same folder whose name starts with the same `<prefix>_MMStack`.
fn stack_files(path: &Path) -> Vec<PathBuf> {
    let name = file_name(path);
    let Some(i) = name.find("MMStack") else { return vec![path.to_path_buf()] };
    let key = &name[..i + "MMStack".len()];
    let dir = path.parent().unwrap_or(Path::new("."));
    let Ok(read) = std::fs::read_dir(dir) else { return vec![path.to_path_buf()] };
    let mut files: Vec<PathBuf> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = file_name(p);
            let lower = n.to_ascii_lowercase();
            n.starts_with(key) && (lower.ends_with(".tif") || lower.ends_with(".tiff")) && p.is_file()
        })
        .collect();
    files.sort();
    if !files.iter().any(|f| f == path) {
        files.insert(0, path.to_path_buf());
    }
    files
}

/// The image stack described by a `<stack stem>_metadata.txt` companion.
fn stack_of_companion(path: &Path) -> Option<PathBuf> {
    let name = file_name(path);
    let stem = name.strip_suffix("_metadata.txt")?;
    let dir = path.parent().unwrap_or(Path::new("."));
    ["ome.tif", "ome.tiff", "tif", "tiff"].iter().map(|ext| dir.join(format!("{stem}.{ext}"))).find(|p| p.is_file())
}

/// `metadata.txt` of each position of a separate-files acquisition: the
/// given one, and those of sibling `Pos<n>` folders with the same prefix.
fn position_folders(meta: &Path) -> Vec<PathBuf> {
    let dir = meta.parent().unwrap_or(Path::new("."));
    let is_pos = |p: &Path| dir_name(p).strip_prefix("Pos").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let (Some(parent), true) = (dir.parent(), is_pos(dir)) else { return vec![meta.to_path_buf()] };
    let prefix = |m: &Path| -> Option<String> {
        let mut head = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(File::open(m).ok()?, 4096), &mut head).ok()?;
        let text = String::from_utf8_lossy(&head).into_owned();
        let at = text.find("\"Prefix\"")?;
        Some(text[at..].split('"').nth(3).unwrap_or_default().to_string())
    };
    let own = prefix(meta);
    let Ok(read) = std::fs::read_dir(parent) else { return vec![meta.to_path_buf()] };
    let mut list: Vec<(u64, PathBuf)> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && is_pos(p))
        .map(|p| p.join("metadata.txt"))
        .filter(|m| m.is_file() && (m.parent() == Some(dir) || prefix(m) == own))
        .map(|m| (dir_name(m.parent().expect("folder"))[3..].parse().unwrap_or(u64::MAX), m))
        .collect();
    list.sort();
    list.into_iter().map(|(_, m)| m).collect()
}

fn dir_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn original(summary: &Value) -> OriginalMeta {
    let mut meta = OriginalMeta { pairs: summary_pairs(summary), documents: Vec::new() };
    if !summary.is_null() {
        meta.documents.push(("Summary".into(), summary.to_string()));
    }
    meta
}

// --- NDTiff index -------------------------------------------------------------

/// Value of one NDTiff axis.
#[derive(Debug, Clone, PartialEq)]
enum AxisValue {
    Int(i64),
    Str(String),
}

impl std::fmt::Display for AxisValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AxisValue::Int(i) => write!(f, "{i}"),
            AxisValue::Str(s) => f.write_str(s),
        }
    }
}

#[derive(Debug)]
struct NdEntry {
    axes: HashMap<String, AxisValue>,
    file: String,
    offset: u64,
    width: u32,
    height: u32,
    pixel_type: u32,
    compression: u32,
    meta: u64,
    meta_len: u32,
}

/// Entries of an `NDTiff.index`; parsing stops at the first incomplete
/// entry (an aborted acquisition).
fn parse_ndtiff_index(b: &[u8]) -> Vec<NdEntry> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
        let s = b.get(*at..at.checked_add(n)?)?;
        *at += n;
        Some(s)
    };
    let u32_at = |s: &[u8], i: usize| u32::from_le_bytes(s[4 * i..4 * i + 4].try_into().expect("4 bytes"));
    loop {
        let Some(k) = take(&mut at, 4).map(|s| u32_at(s, 0) as usize) else { break };
        let Some(axes) = take(&mut at, k).and_then(|s| serde_json::from_slice::<Value>(s).ok()) else { break };
        let Some(n) = take(&mut at, 4).map(|s| u32_at(s, 0) as usize) else { break };
        let Some(file) = take(&mut at, n).map(|s| String::from_utf8_lossy(s).into_owned()) else { break };
        let Some(v) = take(&mut at, 32) else { break };
        let axes = axes
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| {
                let value = match v {
                    Value::Number(n) => AxisValue::Int(n.as_i64()?),
                    Value::String(s) => AxisValue::Str(s.clone()),
                    _ => return None,
                };
                Some((k.clone(), value))
            })
            .collect();
        out.push(NdEntry {
            axes,
            file,
            offset: u32_at(v, 0) as u64,
            width: u32_at(v, 1),
            height: u32_at(v, 2),
            pixel_type: u32_at(v, 3),
            compression: u32_at(v, 4),
            meta: u32_at(v, 5) as u64,
            meta_len: u32_at(v, 6),
        });
    }
    out
}

/// Summary metadata from the header of an NDTiff data file (the length
/// follows the summary header word at byte 16 in v2, 20 in v3, 32 in v1).
fn ndtiff_summary(path: &Path) -> Result<Value> {
    let tiff = TiffFile::from_file(File::open(path)?, 1)?;
    let head = tiff.read_bytes(0, 40)?;
    let at = [16usize, 20, 32].into_iter().find(|&at| tiff.u32(&head[at..]) == SUMMARY_HEADER).ok_or_else(|| Error::Corrupt("NDTiff file without summary".into()))?;
    let len = tiff.u32(&head[at + 4..]) as u64;
    if len > MAX_JSON {
        return Err(Error::Corrupt("NDTiff summary too large".into()));
    }
    let bytes = tiff.read_bytes(at as u64 + 8, len as usize)?;
    parse_json(&bytes).map_err(|e| Error::Corrupt(format!("NDTiff summary: {e}")))
}

impl Backend for MicroManager {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        if self.kind == Kind::Separate {
            return Ok(self.series[series].plane_meta[i].clone());
        }
        Ok(plane_meta(&self.image_json(series, i)?))
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let Some(loc) = self.series[series].planes[i] else {
            // Images missing from the dataset read as zeros.
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let check = |img: &TiffImage| {
            if (img.width as u64, img.height as u64, img.pixel_type, img.samples_out) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type, m.samples_per_pixel) {
                return Err(Error::Corrupt(format!("Micro-Manager image {i} of series {series} differs from the first image")));
            }
            Ok(if img.samples_out == 1 { Samples::One(0) } else { Samples::All })
        };
        match self.kind {
            Kind::Stack | Kind::Separate => self.with_file(loc.file, |tiff| {
                let img = if self.kind == Kind::Stack {
                    TiffImage::new(tiff, &tiff.read_ifd(loc.offset)?.0)?
                } else {
                    TiffImage::new(tiff, &tiff.ifds[0])?
                };
                let samples = check(&img)?;
                img.read_native(tiff, region, samples, sink)
            }),
            Kind::NdTiff => self.with_file(loc.file, |tiff| {
                let bytes = m.pixel_type.bytes();
                let row = m.levels[0].size_x * bytes as u64;
                let w = region.width as usize;
                let little = cfg!(target_endian = "little");
                for r in 0..region.height as usize {
                    let at = loc.offset + (region.y + r as u64) * row + region.x * bytes as u64;
                    if at + (w * bytes) as u64 > tiff.len {
                        return Err(Error::Corrupt(format!("NDTiff image {i} of series {series} beyond the end of its file")));
                    }
                    sink.write_with(r, 0, w, |d| {
                        read_exact_at(&tiff.file, d, at)?;
                        // NDTiff pixels are little-endian.
                        if bytes > 1 && !little {
                            for s in d.chunks_exact_mut(bytes) {
                                s.reverse();
                            }
                        }
                        Ok(())
                    })?;
                }
                Ok(())
            }),
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_become_iso_8601() {
        assert_eq!(mm_date("2016-01-31 08:01:46 +0100").as_deref(), Some("2016-01-31T08:01:46+01:00"));
        assert_eq!(mm_date("2022-09-09 08:56:21.190 -0700").as_deref(), Some("2022-09-09T08:56:21.190-07:00"));
        assert_eq!(mm_date("2016-01-31").as_deref(), None);
    }

    #[test]
    fn java_colours_are_argb() {
        assert_eq!(java_color(&Value::from(-13369549)), Some([0x33, 0xFF, 0x33, 255]));
        assert_eq!(java_color(&Value::from(-1)), Some([255, 255, 255, 255]));
    }

    #[test]
    fn index_entries_stop_at_truncation() {
        let mut b = Vec::new();
        for t in 0..2 {
            let axes = format!("{{\"time\":{t},\"channel\":\"GFP\"}}");
            b.extend((axes.len() as u32).to_le_bytes());
            b.extend(axes.as_bytes());
            b.extend(4u32.to_le_bytes());
            b.extend(b"a.tf");
            for v in [100u32, 2, 3, 1, 0, 200, 10, 0] {
                b.extend(v.to_le_bytes());
            }
        }
        let full = parse_ndtiff_index(&b);
        assert_eq!(full.len(), 2);
        assert_eq!(full[1].axes["channel"], AxisValue::Str("GFP".into()));
        assert_eq!(parse_ndtiff_index(&b[..b.len() - 1]).len(), 1);
    }
}

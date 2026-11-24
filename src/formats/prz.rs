//! # prz
//!
//! CEOS Panta Rhei reader (`.prz`): electron microscopy images saved by
//! the Panta Rhei acquisition software.
//!
//! A `.prz` file is a ZIP archive of NumPy `.npy` arrays:
//!
//! - `data.npy`: the image, a numeric array of shape `(height, width)` or
//!   `(frames, height, width)`; frames are returned as time points.
//! - `meta_data_json.npy`: a 0-d `U` (UTF-32) string holding JSON, either
//!   one object or a list of objects (one per frame); keys are dotted
//!   paths such as `camera.start_time`. `device.calib` is a list with one
//!   entry per image axis (x first) of `{name, value, unit, pixel_factor}`;
//!   the pixel size is `value * pixel_factor` (`value` is the unbinned
//!   pixel in the specimen plane, `pixel_factor` the binning).
//!   Per frame: time since frame 0 from `camera.start_time`, exposure from
//!   `camera.exposure_time`, stage position from `stage.x/y/z` (metres).
//!   `source.name` is the camera (detector model), `repo_id` the
//!   acquisition session (dataset).
//! - `data_model_json.npy`: JSON of the same kind (kept as a document).
//! - `file_format_version.npy`: a 0-d `<i8`.
//! - `meta_data.npy`, `data_model.npy`: Python pickles of the same
//!   content; never read (pickles are code, the JSON copies suffice).
//!
//! Stored entries are read in place, deflated ones are decoded into
//! memory on first read. ZIP64 archives are supported.
//!
//! Provenance: PKWARE ZIP APPNOTE 6.3 (local and central headers, ZIP64);
//! NumPy `.npy` format description (numpy.lib.format documentation and
//! NEP 1, BSD-3 project, prose only); own reverse engineering of a
//! sample file (field meanings, pixel size from `device.calib` checked
//! against detector pixel, binning and magnification). Bio-Formats 8.5.0
//! and bioformats-rs 0.1.8 do not read this format. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use serde_json::Value;

use super::iso::iso_seconds;
use super::strided::{Data, Strided};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, DetectorMeta, Error, FormatId, FormatInfo, LevelMeta, MicroscopeMeta, OriginalMeta, PhysicalSize, PixelType, Plane, PlaneMeta,
    Position, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::PantaRhei, "CEOS Panta Rhei", "1").extensions(&["prz"]);

/// Entries of a Panta Rhei archive; one of them first identifies a file
/// without the `.prz` extension.
const KNOWN: &[&str] = &["data_model.npy", "meta_data.npy", "data_model_json.npy", "meta_data_json.npy", "data.npy", "file_format_version.npy"];
/// Largest metadata entry read (decoded).
const MAX_META: u64 = 64 << 20;
/// Largest deflated image decoded into memory.
const MAX_DECODED: u64 = 4 << 30;

/// Returns `true` if `head` starts a ZIP archive whose first entry is a
/// Panta Rhei array, or one with a `.prz` extension holding a `.npy` entry.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    if head.len() < 30 || &head[..4] != b"PK\x03\x04" {
        return false;
    }
    let n = u16::from_le_bytes([head[26], head[27]]) as usize;
    let Some(name) = head.get(30..30 + n) else { return false };
    let ext = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("prz"));
    KNOWN.iter().any(|k| k.as_bytes() == name) || (ext && name.ends_with(b".npy"))
}

// --- ZIP --------------------------------------------------------------------

/// One archive entry.
#[derive(Debug, Clone)]
struct Entry {
    name: String,
    /// 0 stored, 8 deflated.
    method: u16,
    compressed: u64,
    size: u64,
    /// Offset of the entry data.
    data_at: u64,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap_or_default())
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap_or_default())
}

fn corrupt(what: &str) -> Error {
    Error::Corrupt(format!("Panta Rhei {what}"))
}

/// Reads the central directory of the archive.
fn entries(file: &File, len: u64) -> Result<Vec<Entry>> {
    // End of central directory record: 22 bytes plus a comment of up to 64 KiB.
    let tail_len = len.min(22 + 0xffff + 20);
    let mut tail = vec![0u8; tail_len as usize];
    read_exact_at(file, &mut tail, len - tail_len)?;
    let eocd = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| tail[i..i + 4] == *b"PK\x05\x06")
        .ok_or_else(|| corrupt("archive without central directory"))?;
    let mut count = u16_at(&tail, eocd + 10) as u64;
    let mut cd_len = u32_at(&tail, eocd + 12) as u64;
    let mut cd_at = u32_at(&tail, eocd + 16) as u64;
    // ZIP64: a locator just before the record points to the ZIP64 record.
    if (count == 0xffff || cd_len == 0xffff_ffff || cd_at == 0xffff_ffff) && eocd >= 20 && tail[eocd - 20..eocd - 16] == *b"PK\x06\x07" {
        let at = u64_at(&tail, eocd - 12);
        let mut rec = [0u8; 56];
        if at.checked_add(56).is_none_or(|e| e > len) {
            return Err(corrupt("ZIP64 record beyond the file"));
        }
        read_exact_at(file, &mut rec, at)?;
        if rec[..4] != *b"PK\x06\x06" {
            return Err(corrupt("invalid ZIP64 record"));
        }
        count = u64_at(&rec, 32);
        cd_len = u64_at(&rec, 40);
        cd_at = u64_at(&rec, 48);
    }
    if cd_at.checked_add(cd_len).is_none_or(|e| e > len) || cd_len > MAX_META || count > cd_len / 46 {
        return Err(corrupt("central directory beyond the file"));
    }
    let mut cd = vec![0u8; cd_len as usize];
    read_exact_at(file, &mut cd, cd_at)?;
    let mut out = Vec::with_capacity(count as usize);
    let mut p = 0usize;
    for _ in 0..count {
        let h = cd.get(p..p + 46).ok_or_else(|| corrupt("truncated central directory"))?;
        if h[..4] != *b"PK\x01\x02" {
            return Err(corrupt("invalid central directory entry"));
        }
        let (flags, method) = (u16_at(h, 8), u16_at(h, 10));
        let mut compressed = u32_at(h, 20) as u64;
        let mut size = u32_at(h, 24) as u64;
        let (n, e, c) = (u16_at(h, 28) as usize, u16_at(h, 30) as usize, u16_at(h, 32) as usize);
        let mut local = u32_at(h, 42) as u64;
        let name = cd.get(p + 46..p + 46 + n).ok_or_else(|| corrupt("truncated central directory"))?;
        let name = String::from_utf8_lossy(name).into_owned();
        let extra = cd.get(p + 46 + n..p + 46 + n + e).ok_or_else(|| corrupt("truncated central directory"))?;
        // ZIP64 extended information: the 0xffffffff fields in order.
        let mut x = 0;
        while x + 4 <= extra.len() {
            let (id, l) = (u16_at(extra, x), u16_at(extra, x + 2) as usize);
            let body = extra.get(x + 4..x + 4 + l).unwrap_or_default();
            if id == 1 {
                let mut vals = body.chunks_exact(8).map(|v| u64_at(v, 0));
                for f in [&mut size, &mut compressed, &mut local] {
                    if *f == 0xffff_ffff {
                        *f = vals.next().ok_or_else(|| corrupt("short ZIP64 extra field"))?;
                    }
                }
            }
            x += 4 + l;
        }
        p += 46 + n + e + c;
        if flags & 1 != 0 {
            return Err(Error::UnsupportedVariant("encrypted Panta Rhei archive".into()));
        }
        // The local header repeats name and extra field with their own lengths.
        let mut lh = [0u8; 30];
        if local.checked_add(30).is_none_or(|end| end > len) {
            return Err(corrupt("entry beyond the file"));
        }
        read_exact_at(file, &mut lh, local)?;
        if lh[..4] != *b"PK\x03\x04" {
            return Err(corrupt("invalid local header"));
        }
        let data_at = local + 30 + u16_at(&lh, 26) as u64 + u16_at(&lh, 28) as u64;
        if data_at.checked_add(compressed).is_none_or(|end| end > len) {
            return Err(corrupt("entry data beyond the file"));
        }
        out.push(Entry { name, method, compressed, size, data_at });
    }
    Ok(out)
}

/// Decoded bytes of an entry, at most `limit` of them (stored entries
/// are cut, deflated ones are decoded up to `limit`).
fn entry_bytes(file: &File, e: &Entry, limit: u64) -> Result<Vec<u8>> {
    match e.method {
        0 => {
            let mut b = vec![0u8; e.compressed.min(limit) as usize];
            read_exact_at(file, &mut b, e.data_at)?;
            Ok(b)
        }
        8 => {
            if e.compressed > MAX_DECODED {
                return Err(Error::UnsupportedVariant("deflated Panta Rhei entry above 4 GiB".into()));
            }
            let mut src = vec![0u8; e.compressed as usize];
            read_exact_at(file, &mut src, e.data_at)?;
            // Deflate expands by at most 1032:1.
            let n = e.size.min(limit).min(e.compressed.saturating_mul(1032) + 64);
            let mut out = vec![0u8; n as usize];
            let got = crate::codecs::deflate(&src, &mut out, false)?;
            out.truncate(got);
            Ok(out)
        }
        m => Err(Error::UnsupportedVariant(format!("Panta Rhei entry {} with ZIP method {m}", e.name))),
    }
}

// --- NPY --------------------------------------------------------------------

/// Header of a `.npy` array.
#[derive(Debug, Clone, PartialEq)]
struct Npy {
    /// `descr`, e.g. `<u2`.
    descr: String,
    fortran: bool,
    shape: Vec<u64>,
    /// Bytes before the array data.
    data_at: u64,
}

/// Parses the `.npy` header at the start of `b`.
fn parse_npy(b: &[u8]) -> Result<Npy> {
    if b.len() < 10 || &b[..6] != b"\x93NUMPY" {
        return Err(corrupt("entry is not a NumPy array"));
    }
    let (start, hlen) = match b[6] {
        1 => (10, u16_at(b, 8) as usize),
        2 | 3 if b.len() >= 12 => (12, u32_at(b, 8) as usize),
        v => return Err(Error::UnsupportedVariant(format!("NumPy format version {v}"))),
    };
    let dict = b.get(start..start + hlen).ok_or_else(|| corrupt("truncated NumPy header"))?;
    let dict = String::from_utf8_lossy(dict);
    let value = |key: &str| -> Option<&str> {
        let at = dict.find(&format!("'{key}'")).or_else(|| dict.find(&format!("\"{key}\"")))? + key.len() + 2;
        let rest = dict[at..].trim_start().strip_prefix(':')?.trim_start();
        Some(rest)
    };
    let descr = value("descr").and_then(|v| {
        let q = v.chars().next().filter(|&c| c == '\'' || c == '"')?;
        v[1..].split(q).next().map(String::from)
    });
    let descr = descr.ok_or_else(|| Error::UnsupportedVariant("NumPy array with a structured dtype".into()))?;
    let fortran = value("fortran_order").is_some_and(|v| v.starts_with("True"));
    let shape =
        value("shape").and_then(|v| v.strip_prefix('(')).and_then(|v| v.split(')').next()).ok_or_else(|| corrupt("NumPy header without shape"))?;
    let shape = shape
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('L').parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>();
    let shape = shape.map_err(|_| corrupt("NumPy shape not a tuple of integers"))?;
    Ok(Npy { descr, fortran, shape, data_at: (start + hlen) as u64 })
}

/// Pixel type and byte order (`true`: little endian) of a numeric `descr`.
fn dtype(descr: &str) -> Result<(PixelType, bool)> {
    let (order, kind) = match descr.as_bytes().first() {
        Some(b'<' | b'>' | b'|' | b'=') => (descr.as_bytes()[0], &descr[1..]),
        _ => (b'=', descr),
    };
    let little = order != b'>';
    let t = match kind {
        "u1" | "b1" | "B" => PixelType::U8,
        "i1" | "b" => PixelType::I8,
        "u2" => PixelType::U16,
        "i2" => PixelType::I16,
        "u4" => PixelType::U32,
        "i4" => PixelType::I32,
        "f4" => PixelType::F32,
        "f8" => PixelType::F64,
        _ => return Err(Error::UnsupportedVariant(format!("Panta Rhei samples of NumPy type {descr}"))),
    };
    Ok((t, little))
}

/// Text of a 0-d `U` array (UTF-32), or of a `S` byte string.
fn npy_text(b: &[u8]) -> Result<String> {
    let h = parse_npy(b)?;
    let data = b.get(h.data_at as usize..).unwrap_or_default();
    let (order, kind) = h.descr.split_at(h.descr.len().min(1));
    let text = if kind.starts_with('U') {
        let little = order != ">";
        data.chunks_exact(4)
            .map(|c| if little { u32::from_le_bytes([c[0], c[1], c[2], c[3]]) } else { u32::from_be_bytes([c[0], c[1], c[2], c[3]]) })
            .take_while(|&c| c != 0)
            .map(|c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    } else if kind.starts_with('S') {
        String::from_utf8_lossy(data.split(|&c| c == 0).next().unwrap_or_default()).into_owned()
    } else {
        return Err(corrupt("metadata entry is not a string"));
    };
    Ok(text)
}

// --- Metadata ---------------------------------------------------------------

/// Flattens `v` into `out` as dotted keys; arrays of scalars stay JSON.
fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&key, x, out);
            }
        }
        Value::Array(a) if a.iter().any(|x| x.is_object() || x.is_array()) => {
            for (i, x) in a.iter().enumerate() {
                flatten(&format!("{prefix}[{i}]"), x, out);
            }
        }
        Value::String(s) => out.push((prefix.to_string(), s.clone())),
        Value::Null => {}
        x => out.push((prefix.to_string(), x.to_string())),
    }
}

/// Pixel size in nm of calibration entry `i` (`value * pixel_factor`, in metres).
fn calib_nm(meta: &Value, i: usize) -> Option<f64> {
    let c = meta.get("device.calib")?.get(i)?;
    if c.get("name").and_then(Value::as_str).is_some_and(|n| !n.eq_ignore_ascii_case("length")) {
        return None;
    }
    let scale = match c.get("unit").and_then(Value::as_str).unwrap_or("m") {
        "m" => 1e9,
        "mm" => 1e6,
        "um" | "µm" => 1e3,
        "nm" => 1.0,
        _ => return None,
    };
    let v = c.get("value")?.as_f64()? * c.get("pixel_factor").and_then(Value::as_f64).unwrap_or(1.0) * scale;
    (v.is_finite() && v > 0.0).then_some(v)
}

/// Metadata of frame 0 (`meta_data_json` is an object or a list of them).
fn first(v: &Value) -> Option<&Value> {
    frame(v, 0)
}

/// Metadata of frame `i`: entry `i` of the list, or the single object for
/// frame 0 only.
fn frame(v: &Value, i: usize) -> Option<&Value> {
    match v {
        Value::Array(a) => a.get(i).filter(|x| x.is_object()),
        Value::Object(_) if i == 0 => Some(v),
        _ => None,
    }
}

fn number(m: &Value, k: &str) -> Option<f64> {
    m.get(k).and_then(Value::as_f64).filter(|v| v.is_finite())
}

/// Exposure (`camera.exposure_time`, else the requested one) in seconds.
fn exposure_s(m: &Value) -> Option<f64> {
    number(m, "camera.exposure_time").or_else(|| number(m, "camera.requested_exposure_time"))
}

/// Stage position from `stage.x/y/z` (metres); `None` without x and y.
fn stage(m: &Value) -> Option<Position> {
    let nm = |k: &str| number(m, k).map(|v| v * 1e9);
    let p = Position { x_nm: nm("stage.x"), y_nm: nm("stage.y"), z_nm: nm("stage.z") };
    (p.x_nm.is_some() || p.y_nm.is_some()).then_some(p)
}

/// Per-frame metadata: time since frame 0 (from `camera.start_time`),
/// exposure and stage position of each frame's metadata object.
fn plane_metas(v: Option<&Value>, frames: usize) -> Vec<PlaneMeta> {
    let start = |i: usize| v.and_then(|v| frame(v, i)).and_then(|m| m.get("camera.start_time")).and_then(Value::as_str).and_then(iso_seconds);
    let t0 = start(0);
    (0..frames)
        .map(|i| {
            let m = v.and_then(|v| frame(v, i));
            PlaneMeta { delta_t_s: t0.zip(start(i)).map(|(a, b)| b - a), exposure_s: m.and_then(exposure_s), position: m.and_then(stage) }
        })
        .collect()
}

/// Mean frame interval if all frames have a time and the intervals agree
/// within 1 % (or 1 ms).
fn time_increment(planes: &[PlaneMeta]) -> Option<f64> {
    let t: Vec<f64> = planes.iter().map(|p| p.delta_t_s).collect::<Option<_>>()?;
    if t.len() < 2 {
        return None;
    }
    let mean = (t[t.len() - 1] - t[0]) / (t.len() - 1) as f64;
    let even = t.windows(2).all(|w| ((w[1] - w[0]) - mean).abs() <= (0.01 * mean.abs()).max(1e-3));
    (mean > 0.0 && even).then_some(mean)
}

// --- Reader -----------------------------------------------------------------

/// Panta Rhei image.
#[derive(Debug)]
pub(super) struct PantaRhei {
    samples: Strided,
    metas: Vec<SeriesMeta>,
    /// Metadata of each frame.
    planes: Vec<PlaneMeta>,
    original: OriginalMeta,
}

impl PantaRhei {
    /// Opens a `.prz` archive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid archive or array and
    /// [`Error::UnsupportedVariant`] for encrypted entries, other ZIP
    /// methods, non-numeric samples and arrays that are not 2-D or 3-D.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let entries = entries(&file, len)?;
        let find = |n: &str| entries.iter().find(|e| e.name == n);
        let data = find("data.npy").ok_or_else(|| corrupt("archive without data.npy"))?;

        // Header of the image array: the first bytes of the entry.
        let head = entry_bytes(&file, data, 1 << 16)?;
        let npy = parse_npy(&head)?;
        let (pixel_type, little) = dtype(&npy.descr)?;
        let bytes = pixel_type.bytes() as u64;
        let (frames, ny, nx) = match npy.shape[..] {
            [h, w] => (1, h, w),
            [n, h, w] => (n, h, w),
            _ => return Err(Error::UnsupportedVariant(format!("Panta Rhei array of {} dimensions", npy.shape.len()))),
        };
        if frames == 0 || ny == 0 || nx == 0 || frames > u32::MAX as u64 {
            return Err(corrupt("empty image array"));
        }
        let total = frames
            .checked_mul(ny)
            .and_then(|n| n.checked_mul(nx))
            .and_then(|n| n.checked_mul(bytes))
            .ok_or_else(|| corrupt("image array too large"))?;
        let end = npy.data_at.checked_add(total).ok_or_else(|| corrupt("image array too large"))?;
        if end > data.size {
            return Err(corrupt("image array shorter than its shape"));
        }
        // Element strides of x, y and frames: C order has x fastest,
        // Fortran order the first axis (frames, or y for one frame).
        let (sx, sy, st) = if npy.fortran { (frames * ny, frames, 1) } else { (1, nx, nx * ny) };
        let samples_data = match data.method {
            0 => Data::file(path, data.data_at + npy.data_at, total, "Panta Rhei")?,
            _ => {
                if end > MAX_DECODED {
                    return Err(Error::UnsupportedVariant("deflated Panta Rhei image above 4 GiB".into()));
                }
                let (p, e, skip) = (path.to_path_buf(), data.clone(), npy.data_at as usize);
                Data::decoded(move || {
                    let mut b = entry_bytes(&File::open(&p)?, &e, end)?;
                    if (b.len() as u64) < end {
                        return Err(corrupt("deflated image shorter than its shape"));
                    }
                    b.drain(..skip);
                    Ok(b)
                })
            }
        };

        // Metadata.
        let mut original = OriginalMeta::default();
        let json = |name: &str| -> Option<(String, Value)> {
            let b = entry_bytes(&file, find(name)?, MAX_META).ok()?;
            let text = npy_text(&b).ok()?;
            let v = serde_json::from_str(&text).ok()?;
            Some((text, v))
        };
        let meta_json = json("meta_data_json.npy");
        if let Some(e) = find("file_format_version.npy")
            && let Ok(b) = entry_bytes(&file, e, 4096)
            && let Ok(h) = parse_npy(&b)
            && h.descr.ends_with("i8")
            && let Some(v) = b.get(h.data_at as usize..h.data_at as usize + 8)
        {
            let v = if h.descr.starts_with('>') {
                i64::from_be_bytes(v.try_into().unwrap_or_default())
            } else {
                i64::from_le_bytes(v.try_into().unwrap_or_default())
            };
            original.pairs.push(("file_format_version".into(), v.to_string()));
        }
        if let Some((_, v)) = &meta_json {
            match v {
                Value::Array(a) if a.len() > 1 => a.iter().enumerate().for_each(|(i, x)| flatten(&format!("[{i}]"), x, &mut original.pairs)),
                _ => flatten("", first(v).unwrap_or(v), &mut original.pairs),
            }
        }
        for name in ["meta_data_json", "data_model_json"] {
            if let Some((text, _)) = json(&format!("{name}.npy")) {
                original.documents.push((name.to_string(), text));
            }
        }

        let all = meta_json.as_ref().map(|(_, v)| v);
        let m = all.and_then(first);
        let text = |k: &str| m.and_then(|m| m.get(k)).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
        let planes = plane_metas(all, frames as usize);
        let detector = text("source.name").map(|model| DetectorMeta { model: Some(model), ..Default::default() });
        let channel = ChannelMeta { exposure_s: m.and_then(exposure_s), detector, ..Default::default() };
        let meta = SeriesMeta {
            name: path.file_stem().map(|n| n.to_string_lossy().into_owned()),
            acquisition_date: text("camera.start_time"),
            size_c: 1,
            size_z: 1,
            size_t: frames as u32,
            time_increment_s: time_increment(&planes),
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: nx, size_y: ny, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: m.and_then(|m| calib_nm(m, 0)), y_nm: m.and_then(|m| calib_nm(m, 1)), z_nm: None },
            channels: vec![channel],
            microscope: text("microscope.name").map(|model| MicroscopeMeta { model: Some(model), ..Default::default() }),
            stage_position: m.and_then(stage),
            // The acquisition session; the series name stays the file name.
            dataset: text("repo_id"),
            ..Default::default()
        };
        let samples = Strided {
            data: samples_data,
            sx,
            sy,
            sz: 0,
            c_axes: vec![(0, 1)],
            t_axes: vec![(st, frames)],
            bytes: bytes as usize,
            little,
            flip_y: false,
            height: ny,
            total,
            what: "Panta Rhei",
        };
        Ok(PantaRhei { samples, metas: vec![meta], planes, original })
    }
}

impl Backend for PantaRhei {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        Ok(self.planes.get(plane.t as usize).copied().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        self.samples.read_plane(plane, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_npy_headers() {
        let mut b = b"\x93NUMPY\x01\x00".to_vec();
        let h = b"{'descr': '<u2', 'fortran_order': False, 'shape': (3, 4), }     \n";
        b.extend_from_slice(&(h.len() as u16).to_le_bytes());
        b.extend_from_slice(h);
        let n = parse_npy(&b).unwrap();
        assert_eq!((n.descr.as_str(), n.fortran, n.shape.as_slice(), n.data_at), ("<u2", false, &[3, 4][..], 10 + h.len() as u64));
        assert_eq!(dtype("<u2").unwrap(), (PixelType::U16, true));
        assert_eq!(dtype(">f4").unwrap(), (PixelType::F32, false));
        assert!(dtype("<c8").is_err());
    }

    /// A stored archive of one entry with a ZIP64 central directory: the
    /// sizes and the local header offset in the ZIP64 extra field, a ZIP64
    /// end record and its locator.
    #[test]
    fn reads_zip64_central_directory() {
        let (name, body) = (b"data.npy", b"abcd");
        let mut z = Vec::new();
        z.extend_from_slice(b"PK\x03\x04\x2d\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
        z.extend_from_slice(&(name.len() as u16).to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z.extend_from_slice(name);
        z.extend_from_slice(body);
        let cd_at = z.len() as u64;
        z.extend_from_slice(b"PK\x01\x02\x2d\0\x2d\0\0\0\0\0\0\0\0\0\0\0\0\0");
        z.extend_from_slice(&[0xff; 8]); // compressed and uncompressed size
        z.extend_from_slice(&(name.len() as u16).to_le_bytes());
        z.extend_from_slice(&28u16.to_le_bytes()); // extra length
        z.extend_from_slice(&[0; 10]); // comment length, disk, attributes
        z.extend_from_slice(&[0xff; 4]); // local header offset
        z.extend_from_slice(name);
        z.extend_from_slice(&1u16.to_le_bytes());
        z.extend_from_slice(&24u16.to_le_bytes());
        for v in [body.len() as u64, body.len() as u64, 0] {
            z.extend_from_slice(&v.to_le_bytes());
        }
        let cd_len = z.len() as u64 - cd_at;
        let rec_at = z.len() as u64;
        z.extend_from_slice(b"PK\x06\x06");
        z.extend_from_slice(&44u64.to_le_bytes());
        z.extend_from_slice(&[0x2d, 0, 0x2d, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for v in [1u64, 1, cd_len, cd_at] {
            z.extend_from_slice(&v.to_le_bytes());
        }
        z.extend_from_slice(b"PK\x06\x07\0\0\0\0");
        z.extend_from_slice(&rec_at.to_le_bytes());
        z.extend_from_slice(&1u32.to_le_bytes());
        z.extend_from_slice(b"PK\x05\x06\0\0\0\0\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\0\0");
        let path = std::env::temp_dir().join(format!("scopeio-prz-zip64-{}.prz", std::process::id()));
        std::fs::write(&path, &z).unwrap();
        let file = File::open(&path).unwrap();
        let e = entries(&file, z.len() as u64).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].name.as_str(), e[0].method, e[0].size, e[0].data_at), ("data.npy", 0, 4, 30 + name.len() as u64));
        assert_eq!(entry_bytes(&file, &e[0], 16).unwrap(), body);
    }

    #[test]
    fn reads_calibration() {
        let v: Value =
            serde_json::from_str(r#"{"device.calib": [{"name": "Length", "value": 2.5e-10, "unit": "m", "pixel_factor": 2}, null]}"#).unwrap();
        assert!((calib_nm(&v, 0).unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(calib_nm(&v, 1), None);
    }
}

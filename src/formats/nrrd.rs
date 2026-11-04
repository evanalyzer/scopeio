//! # nrrd
//!
//! NRRD ("nearly raw raster data") reader: attached headers (`.nrrd`) and
//! detached headers (`.nhdr`) with one data file.
//!
//! The header is text: the magic `NRRD000<n>`, then `<field>: <value>`
//! lines, `<key>:=<value>` pairs and `#` comments, ended by a blank line
//! (after which the data of an attached header follows) or the end of a
//! detached header. Required fields: `type`, `dimension`, `sizes` (fastest
//! axis first) and `encoding` (`raw`, `gzip` / `gz`, `ascii` / `text` /
//! `txt`, `hex`); `endian` for multi-byte raw data; `data file` names the
//! data of a detached header, `line skip` / `byte skip` locate the samples
//! in it (`byte skip: -1`: raw samples end at the end of the file).
//!
//! Axis roles: by `kinds` (`space` / `domain` axes are x, y, z in order,
//! `time` the time points, others such as `list`, `vector` or `RGB-color`
//! channels); without kinds an axis whose `space directions` entry is
//! `none` or whose `spacings` / `axis mins` are NaN is a channel axis, the
//! others are x, y, z and then time. Pixel sizes from the lengths of the
//! space directions (in `space units`) or from `spacings` (in `units`);
//! a length without unit is taken as micrometres.
//!
//! Provenance: Teem NRRD file format definition (G. Kindlmann,
//! `doc/nrrd/`); own analysis of the NRRD samples on the OME sample server
//! (CC BY 4.0); checked against Bio-Formats and bioformats-rs as black
//! boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::{Path, PathBuf};

use super::strided::{Data, Strided};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Nrrd, "NRRD", "1").extensions(&["nrrd", "nhdr"]);

/// Largest header parsed.
const MAX_HEADER: usize = 1 << 20;
/// Largest data decoded into memory (compressed or text encodings).
const MAX_DECODED: u64 = 4 << 30;

/// Returns `true` for a NRRD header.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(b"NRRD000") || head.starts_with(b"NRRD00.01")
}

/// Header fields (lower-case names), key / value pairs and the byte
/// offset after the blank line ending the header (attached data).
struct Header {
    fields: Vec<(String, String)>,
    pairs: Vec<(String, String)>,
    data_at: Option<usize>,
}

fn parse_header(raw: &[u8]) -> Header {
    let mut h = Header { fields: Vec::new(), pairs: Vec::new(), data_at: None };
    let mut at = 0;
    for (n, line) in raw.split_inclusive(|&b| b == b'\n').enumerate() {
        at += line.len();
        let text = String::from_utf8_lossy(line);
        let text = text.trim_end_matches(['\n', '\r']);
        if n == 0 {
            continue;
        }
        if text.is_empty() {
            h.data_at = Some(at);
            break;
        }
        if text.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = text.split_once(":=") {
            h.pairs.push((k.to_string(), v.to_string()));
        } else if let Some((k, v)) = text.split_once(':') {
            h.fields.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    h
}

/// Sample type of a `type` value.
fn pixel_type(t: &str) -> Option<PixelType> {
    Some(match t.trim().to_ascii_lowercase().as_str() {
        "signed char" | "int8" | "int8_t" => PixelType::I8,
        "uchar" | "unsigned char" | "uint8" | "uint8_t" => PixelType::U8,
        "short" | "short int" | "signed short" | "signed short int" | "int16" | "int16_t" => PixelType::I16,
        "ushort" | "unsigned short" | "unsigned short int" | "uint16" | "uint16_t" => PixelType::U16,
        "int" | "signed int" | "int32" | "int32_t" => PixelType::I32,
        "uint" | "unsigned int" | "uint32" | "uint32_t" => PixelType::U32,
        "float" => PixelType::F32,
        "double" => PixelType::F64,
        _ => return None,
    })
}

/// Words of a field value, quoted strings as one word.
fn words(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = v.trim();
    while !rest.is_empty() {
        if let Some(q) = rest.strip_prefix('"') {
            let end = q.find('"').unwrap_or(q.len());
            out.push(q[..end].to_string());
            rest = q.get(end + 1..).unwrap_or("").trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest[..end].to_string());
            rest = rest[end..].trim_start();
        }
    }
    out
}

/// Vectors of `space directions`: `None` for `none`.
fn directions(v: &str) -> Vec<Option<Vec<f64>>> {
    let mut out = Vec::new();
    let mut rest = v.trim();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('(') {
            let end = r.find(')').unwrap_or(r.len());
            out.push(Some(r[..end].split(',').filter_map(|c| c.trim().parse().ok()).collect()));
            rest = r.get(end + 1..).unwrap_or("").trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(None);
            rest = rest[end..].trim_start();
        }
    }
    out
}

/// A length in nanometres; micrometres if the unit is missing.
fn length_nm(v: f64, unit: &str) -> Option<f64> {
    let f = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "micrometers" | "micrometer" | "microns" | "micron" | "um" | "µm" => 1e3,
        "nanometers" | "nanometer" | "nm" => 1.0,
        "millimeters" | "millimeter" | "mm" => 1e6,
        "centimeters" | "cm" => 1e7,
        "meters" | "meter" | "m" => 1e9,
        _ => return None,
    };
    Some(v * f).filter(|v| v.is_finite() && *v > 0.0)
}

fn time_s(v: f64, unit: &str) -> Option<f64> {
    let f = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "s" | "sec" | "second" | "seconds" => 1.0,
        "ms" | "msec" | "millisecond" | "milliseconds" => 1e-3,
        "us" | "microsecond" | "microseconds" => 1e-6,
        "min" | "minute" | "minutes" => 60.0,
        _ => return None,
    };
    Some(v * f).filter(|v| v.is_finite() && *v > 0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Space,
    Time,
    Channel,
    Unknown,
}

/// Offset of the byte after `lines` line ends from `start` in `path`.
fn skip_lines(path: &Path, start: u64, lines: u64) -> Result<u64> {
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let (mut at, mut left) = (start, lines);
    let mut buf = vec![0u8; 1 << 16];
    while left > 0 {
        let n = (len.saturating_sub(at)).min(buf.len() as u64) as usize;
        if n == 0 {
            return Err(Error::Corrupt("NRRD line skip beyond the data file".into()));
        }
        read_exact_at(&file, &mut buf[..n], at)?;
        let mut used = n;
        for (i, &b) in buf[..n].iter().enumerate() {
            if b == b'\n' {
                left -= 1;
                if left == 0 {
                    used = i + 1;
                    break;
                }
            }
        }
        at += used as u64;
    }
    Ok(at)
}

/// Text samples (`ascii`) as little-endian elements.
fn parse_text(text: &[u8], pixel_type: PixelType, count: u64) -> Result<Vec<u8>> {
    // At least one digit and one separator per sample.
    if count > (text.len() as u64 + 1) / 2 {
        return Err(Error::Corrupt("NRRD text data shorter than its sizes".into()));
    }
    let mut out = Vec::with_capacity((count as usize).saturating_mul(pixel_type.bytes()));
    let text = String::from_utf8_lossy(text);
    for word in text.split_ascii_whitespace().take(count as usize) {
        let bad = || Error::Corrupt(format!("NRRD text sample {word:?}"));
        let int = || word.parse::<i64>().or_else(|_| word.parse::<f64>().map(|v| v as i64)).map_err(|_| bad());
        match pixel_type {
            PixelType::U8 => out.push(int()? as u8),
            PixelType::I8 => out.push(int()? as i8 as u8),
            PixelType::U16 => out.extend((int()? as u16).to_le_bytes()),
            PixelType::I16 => out.extend((int()? as i16).to_le_bytes()),
            PixelType::U32 => out.extend((int()? as u32).to_le_bytes()),
            PixelType::I32 => out.extend((int()? as i32).to_le_bytes()),
            PixelType::F32 => out.extend(word.parse::<f32>().map_err(|_| bad())?.to_le_bytes()),
            PixelType::F64 => out.extend(word.parse::<f64>().map_err(|_| bad())?.to_le_bytes()),
        }
    }
    if out.len() as u64 != count * pixel_type.bytes() as u64 {
        return Err(Error::Corrupt("NRRD text data shorter than its sizes".into()));
    }
    Ok(out)
}

/// Hexadecimal samples, two digits per byte, white space ignored.
fn parse_hex(text: &[u8], len: u64) -> Result<Vec<u8>> {
    if len > text.len() as u64 / 2 {
        return Err(Error::Corrupt("NRRD hex data shorter than its sizes".into()));
    }
    let mut out = Vec::with_capacity(len as usize);
    let mut high = None;
    for &b in text {
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ if b.is_ascii_whitespace() || b == 0 => continue,
            _ => return Err(Error::Corrupt("NRRD hex data with a non-hex character".into())),
        };
        match high.take() {
            None => high = Some(v),
            Some(h) => {
                out.push(h << 4 | v);
                if out.len() as u64 == len {
                    return Ok(out);
                }
            }
        }
    }
    Err(Error::Corrupt("NRRD hex data shorter than its sizes".into()))
}

/// NRRD image.
#[derive(Debug)]
pub(super) struct Nrrd {
    samples: Strided,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Nrrd {
    /// Opens a NRRD header (`.nrrd` or `.nhdr`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header or short data and
    /// [`Error::UnsupportedVariant`] for block or 64-bit integer samples,
    /// bzip2 data and several data files.
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        let mut raw = vec![0u8; len.min(MAX_HEADER as u64) as usize];
        read_exact_at(&file, &mut raw, 0)?;
        let header = parse_header(&raw);
        let get = |k: &str| header.fields.iter().find(|(f, _)| f == k || f.replace(' ', "") == k.replace(' ', "")).map(|(_, v)| v.as_str());
        let type_name = get("type").ok_or_else(|| Error::Corrupt("NRRD header without type".into()))?;
        let pixel_type = pixel_type(type_name).ok_or_else(|| Error::UnsupportedVariant(format!("NRRD sample type {type_name}")))?;
        let bytes = pixel_type.bytes();
        let dim: usize = get("dimension").and_then(|v| v.trim().parse().ok()).filter(|&d| (1..=16).contains(&d)).ok_or_else(|| Error::Corrupt("NRRD header without valid dimension".into()))?;
        let sizes: Vec<u64> = get("sizes").map(|v| v.split_whitespace().filter_map(|s| s.parse().ok()).collect()).unwrap_or_default();
        if sizes.len() != dim || sizes.contains(&0) {
            return Err(Error::Corrupt("NRRD sizes do not match the dimension".into()));
        }
        let count = sizes.iter().try_fold(1u64, |a, &s| a.checked_mul(s)).ok_or_else(|| Error::Corrupt("NRRD sizes too large".into()))?;
        let total = count.checked_mul(bytes as u64).ok_or_else(|| Error::Corrupt("NRRD sizes too large".into()))?;
        let little = match get("endian").map(|e| e.trim().to_ascii_lowercase()) {
            Some(e) if e == "big" => false,
            Some(e) if e == "little" => true,
            None => true,
            Some(e) => return Err(Error::Corrupt(format!("NRRD endian {e}"))),
        };
        let encoding = get("encoding").map(|e| e.trim().to_ascii_lowercase()).ok_or_else(|| Error::Corrupt("NRRD header without encoding".into()))?;

        // Per-axis information.
        let per_axis = |k: &str| get(k).map(words).unwrap_or_default();
        let kinds = per_axis("kinds");
        // One direction per axis; some writers leave out the `none` of
        // non-spatial axes (fewer entries than axes): those are mapped onto
        // the spatial axes below.
        let all_dirs = get("space directions").map(directions).unwrap_or_default();
        let space_dirs = if all_dirs.len() == dim { all_dirs.clone() } else { Vec::new() };
        let spacings: Vec<f64> = per_axis("spacings").iter().map(|s| s.parse().unwrap_or(f64::NAN)).collect();
        let mins: Vec<f64> = per_axis("axis mins").iter().map(|s| s.parse().unwrap_or(f64::NAN)).collect();
        let units = per_axis("units");
        let space_units = per_axis("space units");
        let roles: Vec<Role> = (0..dim)
            .map(|i| {
                let kind = kinds.get(i).map(|k| k.to_ascii_lowercase()).unwrap_or_default();
                match kind.as_str() {
                    "space" | "domain" => Role::Space,
                    "time" => Role::Time,
                    "" | "none" | "???" => {
                        let non_spatial = space_dirs.get(i).is_some_and(|d| d.is_none())
                            || spacings.get(i).is_some_and(|v| v.is_nan()) && spacings.iter().any(|v| !v.is_nan())
                            || mins.get(i).is_some_and(|v| v.is_nan()) && mins.iter().any(|v| !v.is_nan());
                        if non_spatial { Role::Channel } else { Role::Unknown }
                    }
                    _ => Role::Channel,
                }
            })
            .collect();
        let space_dirs = if all_dirs.len() == dim || all_dirs.is_empty() {
            space_dirs
        } else {
            let mut given = all_dirs.into_iter().filter(Option::is_some);
            roles.iter().map(|r| if matches!(r, Role::Space | Role::Unknown) { given.next().flatten() } else { None }).collect()
        };
        // Spacing of axis `i` in nm (or s for time).
        let space_index: Vec<usize> = (0..dim).filter(|&i| space_dirs.get(i).is_some_and(|d| d.is_some())).collect();
        let length = |i: usize| -> Option<f64> {
            if let Some(Some(v)) = space_dirs.get(i) {
                let norm = v.iter().map(|c| c * c).sum::<f64>().sqrt();
                let k = space_index.iter().position(|&j| j == i).unwrap_or(0);
                return length_nm(norm, space_units.get(k).map_or("", String::as_str));
            }
            spacings.get(i).and_then(|&s| length_nm(s, units.get(i).map_or("", String::as_str)))
        };

        // Strides: x, y, z from the first spatial (or unknown) axes, then time.
        let mut stride = 1u64;
        let (mut space, mut c_axes, mut t_axes) = (Vec::new(), Vec::new(), Vec::new());
        let mut dt = None;
        for (i, (&size, &role)) in sizes.iter().zip(&roles).enumerate() {
            match role {
                Role::Space | Role::Unknown if space.len() < 3 => space.push((stride, size, i)),
                Role::Time | Role::Unknown => {
                    if t_axes.is_empty() {
                        dt = spacings.get(i).and_then(|&s| time_s(s, units.get(i).map_or("", String::as_str)));
                    }
                    t_axes.push((stride, size))
                }
                _ => c_axes.push((stride, size)),
            }
            stride *= size;
        }
        // Fewer than two spatial axes: the plane is made of the first axes.
        while space.len() < 2 {
            let pick = c_axes.first().copied().map(|(s, n)| (s, n, usize::MAX));
            match pick {
                Some(p) => {
                    c_axes.remove(0);
                    space.push(p);
                }
                None => space.push((stride, 1, usize::MAX)),
            }
            space.sort_by_key(|a| a.0);
        }
        let (sx, nx, ix) = space[0];
        let (sy, ny, iy) = space[1];
        let (sz, nz, iz) = space.get(2).copied().unwrap_or((0, 1, usize::MAX));
        let size_c: u64 = c_axes.iter().map(|a| a.1).product();
        let size_t: u64 = t_axes.iter().map(|a| a.1).product();
        if [nx, ny, nz, size_c, size_t].iter().any(|&v| v > u32::MAX as u64) {
            return Err(Error::Corrupt("NRRD sizes too large".into()));
        }

        // Data location.
        let dir = path.parent().unwrap_or(Path::new("."));
        let data_file = get("data file");
        let data_path: PathBuf = match data_file {
            None => {
                if header.data_at.is_none() {
                    return Err(Error::Corrupt("NRRD header without data".into()));
                }
                path.to_path_buf()
            }
            Some(f) => {
                let f = f.trim();
                if f.starts_with("LIST") || f.split_whitespace().count() > 1 {
                    return Err(Error::UnsupportedVariant("NRRD with several data files".into()));
                }
                let name = Path::new(&f.replace('\\', "/")).components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect::<PathBuf>();
                dir.join(name)
            }
        };
        let start = if data_file.is_none() { header.data_at.unwrap_or(0) as u64 } else { 0 };
        let line_skip: u64 = get("line skip").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        let byte_skip: i64 = get("byte skip").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        let start = if line_skip > 0 { skip_lines(&data_path, start, line_skip)? } else { start };
        let data = match encoding.as_str() {
            "raw" => {
                let file_len = std::fs::metadata(&data_path)?.len();
                let offset = if byte_skip == -1 { file_len.checked_sub(total).ok_or_else(|| Error::Corrupt("NRRD data shorter than its sizes".into()))? } else { start + byte_skip.max(0) as u64 };
                Data::file(&data_path, offset, total, "NRRD")?
            }
            "gzip" | "gz" => {
                if total > MAX_DECODED {
                    return Err(Error::UnsupportedVariant("compressed NRRD data above 4 GiB".into()));
                }
                Data::gzip(&data_path, start, byte_skip.max(0) as u64, total, "NRRD")
            }
            "ascii" | "text" | "txt" | "hex" => {
                if total > MAX_DECODED {
                    return Err(Error::UnsupportedVariant("NRRD text data above 4 GiB".into()));
                }
                let (p, hex) = (data_path.clone(), encoding == "hex");
                let skip = byte_skip.max(0) as u64;
                Data::decoded(move || {
                    let raw = std::fs::read(&p)?;
                    let text = raw.get((start + skip) as usize..).ok_or_else(|| Error::Corrupt("NRRD data offset beyond the file".into()))?;
                    if hex { parse_hex(text, total) } else { parse_text(text, pixel_type, count) }
                })
            }
            e => return Err(Error::UnsupportedVariant(format!("NRRD encoding {e}"))),
        };
        // Text samples are decoded little endian.
        let little = little || matches!(encoding.as_str(), "ascii" | "text" | "txt");

        let at = |i: usize| if i == usize::MAX { None } else { length(i) };
        let meta = SeriesMeta {
            name: get("content").filter(|c| !c.is_empty()).map(String::from).or_else(|| path.file_stem().map(|n| n.to_string_lossy().into_owned())),
            size_c: size_c as u32,
            size_z: nz as u32,
            size_t: size_t as u32,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: nx, size_y: ny, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: at(ix), y_nm: at(iy), z_nm: at(iz).filter(|_| nz > 1) },
            time_increment_s: dt.filter(|_| size_t > 1),
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        original.pairs.extend(header.fields.iter().cloned());
        original.pairs.extend(header.pairs.iter().cloned());
        let samples = Strided { data, sx, sy, sz, c_axes, t_axes, bytes, little, flip_y: false, height: ny, total, what: "NRRD" };
        Ok(Nrrd { samples, metas: vec![meta], original })
    }
}

impl Backend for Nrrd {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
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
    fn parses_header_values() {
        let h = parse_header(b"NRRD0004\n# c\ntype: float\nkey:=v:1\nsizes: 2 3\n\n\x01");
        assert_eq!(h.fields, [("type".to_string(), "float".to_string()), ("sizes".into(), "2 3".into())]);
        assert_eq!(h.pairs, [("key".to_string(), "v:1".to_string())]);
        assert_eq!(h.data_at, Some(46));
        assert_eq!(words("\"\" \"mm\" x"), ["", "mm", "x"]);
        assert_eq!(directions("none (1,0,0) (0,0.5,0)"), [None, Some(vec![1.0, 0.0, 0.0]), Some(vec![0.0, 0.5, 0.0])]);
        assert_eq!(parse_text(b" 1 -2\n3 ", PixelType::I16, 3).unwrap(), [1, 0, 0xfe, 0xff, 3, 0]);
        assert_eq!(parse_hex(b"0a F0\n", 2).unwrap(), [0x0a, 0xf0]);
    }
}

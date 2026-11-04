//! # gatan
//!
//! Gatan DigitalMicrograph reader (`.dm3`, `.dm4`).
//!
//! A DM file is a header (version 3 or 4, root length, byte order of the
//! values) and a tree of tag groups and data tags; structure words are big
//! endian, values in the header's byte order, and DM4 widens counts and
//! info words to 64 bits and adds a byte length to every tag. Images are the
//! entries of `ImageList` not named in `Thumbnails[*].ImageIndex`, each with
//! `ImageData` (`DataType`, `Dimensions` fastest first, `Calibrations`
//! with `Scale` / `Origin` / `Units` per dimension, and the `Data` array)
//! and `ImageTags`. Large arrays are skipped by offset while parsing, so
//! the pixels are never read at open.
//!
//! Each image is a series. Dimensions 0 and 1 are x and y (a 1-D spectrum
//! is one row); a third dimension holds channels when its unit is an energy
//! (`eV`, `keV`, `meV`: spectrum images), z planes when it is a length and
//! the image is not a sequence (`ImageTags.Meta Data.IsSequence`), else
//! time points; further dimensions are time points too. Planes are
//! contiguous. Pixel sizes from length calibrations (`nm`, `µm`, `Å`,
//! `pm`, `mm`, `m`). Complex, packed complex and RGB (type 8) images are not
//! supported.
//!
//! Provenance: C. Boothroyd's DM2/DM3/DM4 format description and G.
//! Jefferis' DM3 notes (`doc/gatan-dm/`, both reverse-engineered
//! descriptions); OpenReadout DM notes (MIT OR Apache-2.0, `doc/gatan-dm/`):
//! data type table, RGBA byte order, axis rules; own analysis of the DM4
//! samples on the OME sample server (CC BY 4.0); checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Gatan, "Gatan DigitalMicrograph", "1").extensions(&["dm3", "dm4"]);

/// Arrays up to this many elements are read while parsing (names, units).
const INLINE_ARRAY: u64 = 4096;
/// Deepest group nesting followed.
const MAX_DEPTH: usize = 64;
/// Most tags parsed, against corrupt counts.
const MAX_TAGS: usize = 1 << 20;
/// Bytes read at once while parsing.
const WINDOW: usize = 1 << 20;
/// Most original metadata pairs kept.
const MAX_PAIRS: usize = 4096;

/// Returns `true` for a DM3 or DM4 file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    head.len() >= 16 && matches!(ext.as_str(), "dm3" | "dm4") && matches!(&head[..4], [0, 0, 0, 3] | [0, 0, 0, 4])
}

fn corrupt(what: &str) -> Error {
    Error::Corrupt(format!("DM: {what}"))
}

/// A parsed tag value.
#[derive(Debug, Clone)]
enum Value {
    Num(f64),
    Text(String),
    /// An array: file offset and byte length.
    Array { offset: u64, len: u64 },
    Other,
}

/// A tag tree node.
#[derive(Debug, Clone)]
enum Node {
    Group(Vec<(String, Node)>),
    Data(Value),
}

impl Node {
    fn get(&self, name: &str) -> Option<&Node> {
        match self {
            Node::Group(entries) => entries.iter().find(|(n, _)| n == name).map(|(_, v)| v),
            Node::Data(_) => None,
        }
    }

    /// Node at a `.`-separated path.
    fn path(&self, path: &str) -> Option<&Node> {
        path.split('.').try_fold(self, |n, p| n.get(p))
    }

    fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Group(entries) => entries,
            Node::Data(_) => &[],
        }
    }

    fn num(&self) -> Option<f64> {
        match self {
            Node::Data(Value::Num(v)) => Some(*v),
            _ => None,
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Node::Data(Value::Text(t)) => Some(t),
            _ => None,
        }
    }
}

/// Size in bytes of a scalar tag type.
fn type_size(t: u64) -> Option<u64> {
    Some(match t {
        2 | 4 => 2,
        3 | 5 | 6 => 4,
        7 | 11 | 12 => 8,
        8..=10 => 1,
        _ => return None,
    })
}

/// Tag tree parser over a window of the file.
struct Parser<'a> {
    file: &'a File,
    len: u64,
    dm4: bool,
    little: bool,
    at: u64,
    buf: Vec<u8>,
    buf_at: u64,
    tags: usize,
}

impl Parser<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        if self.at.checked_add(n as u64).is_none_or(|e| e > self.len) {
            return Err(corrupt("tag directory beyond the end of the file"));
        }
        let start = (self.at - self.buf_at.min(self.at)) as usize;
        if self.at < self.buf_at || start + n > self.buf.len() {
            let take = (WINDOW.max(n) as u64).min(self.len - self.at) as usize;
            self.buf.resize(take, 0);
            read_exact_at(self.file, &mut self.buf, self.at)?;
            self.buf_at = self.at;
        }
        let start = (self.at - self.buf_at) as usize;
        self.at += n as u64;
        Ok(&self.buf[start..start + n])
    }

    fn be(&mut self, n: usize) -> Result<u64> {
        Ok(self.bytes(n)?.iter().fold(0u64, |a, &b| (a << 8) | b as u64))
    }

    /// A count or info word: 4 bytes in DM3, 8 in DM4.
    fn word(&mut self) -> Result<u64> {
        self.be(if self.dm4 { 8 } else { 4 })
    }

    /// A scalar value of type `t` in the file's value byte order.
    fn scalar(&mut self, t: u64) -> Result<f64> {
        let n = type_size(t).ok_or_else(|| corrupt("unknown scalar type"))? as usize;
        let little = self.little;
        let b = self.bytes(n)?;
        let mut raw = [0u8; 8];
        raw[..n].copy_from_slice(b);
        if !little {
            raw[..n].reverse();
        }
        let u = u64::from_le_bytes(raw);
        Ok(match t {
            2 => u as u16 as i16 as f64,
            3 => u as u32 as i32 as f64,
            6 => f32::from_bits(u as u32) as f64,
            7 => f64::from_bits(u),
            11 => u as i64 as f64,
            _ => u as f64,
        })
    }

    fn group(&mut self, depth: usize) -> Result<Node> {
        if depth > MAX_DEPTH {
            return Err(corrupt("tag groups nested too deeply"));
        }
        let _sorted = self.be(1)?;
        let _open = self.be(1)?;
        let n = self.word()?;
        let mut entries = Vec::new();
        for _ in 0..n {
            self.tags += 1;
            if self.tags > MAX_TAGS {
                return Err(corrupt("too many tags"));
            }
            let kind = self.be(1)?;
            let name_len = self.be(2)? as usize;
            let name = String::from_utf8_lossy(self.bytes(name_len)?).into_owned();
            let tag_len = if self.dm4 { Some(self.be(8)?) } else { None };
            let start = self.at;
            let node = match kind {
                20 => self.group(depth + 1)?,
                21 => Node::Data(self.data(name == "Data")?),
                0 => break,
                _ => return Err(corrupt("unknown tag kind")),
            };
            // DM4 tag lengths resynchronise after unparsed contents.
            if let Some(l) = tag_len
                && self.at != start + l
                && start + l <= self.len
            {
                self.at = start + l;
            }
            entries.push((name, node));
        }
        Ok(Node::Group(entries))
    }

    /// A data tag; `pixels` marks the `Data` array of an image, never text.
    fn data(&mut self, pixels: bool) -> Result<Value> {
        if self.bytes(4)? != b"%%%%" {
            return Err(corrupt("tag without %%%% marker"));
        }
        let n = self.word()?;
        if n == 0 || n > 1 << 16 {
            return Err(corrupt("tag info count"));
        }
        let info: Vec<u64> = (0..n).map(|_| self.word()).collect::<Result<_>>()?;
        Ok(match info[0] {
            t if type_size(t).is_some() => Value::Num(self.scalar(t)?),
            // DM3 strings: the byte length follows the type.
            18 => {
                let l = info.get(1).copied().unwrap_or(0) as usize;
                if l > 1 << 20 {
                    return Err(corrupt("string tag too long"));
                }
                Value::Text(String::from_utf8_lossy(self.bytes(l)?).into_owned())
            }
            // Struct: name length, field count, (name length, type) pairs.
            15 => {
                let fields = info.get(2).copied().unwrap_or(0);
                let mut size = 0u64;
                for k in 0..fields {
                    let t = info.get(4 + 2 * k as usize).copied().unwrap_or(0);
                    size += type_size(t).ok_or_else(|| corrupt("struct field type"))?;
                }
                self.at += size;
                Value::Other
            }
            20 => {
                let elem = info.get(1).copied().unwrap_or(0);
                let count = *info.last().expect("info not empty");
                let elem_size = match type_size(elem) {
                    Some(s) => s,
                    // Array of structs: field types at info[5], info[7], ...
                    None if elem == 15 => {
                        let fields = info.get(3).copied().unwrap_or(0);
                        (0..fields).try_fold(0u64, |a, k| Some(a + type_size(*info.get(5 + 2 * k as usize)?)?)).ok_or_else(|| corrupt("struct array field type"))?
                    }
                    None => return Err(corrupt("array element type")),
                };
                let len = count.checked_mul(elem_size).ok_or_else(|| corrupt("array too large"))?;
                let offset = self.at;
                if offset.checked_add(len).is_none_or(|e| e > self.len) {
                    return Err(corrupt("array beyond the end of the file"));
                }
                // Short 16-bit arrays are text (names, units).
                if elem == 4 && count <= INLINE_ARRAY && !pixels {
                    let little = self.little;
                    let b = self.bytes(len as usize)?;
                    let units: Vec<u16> = b.chunks_exact(2).map(|c| if little { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
                    match String::from_utf16(&units) {
                        Ok(s) => Value::Text(s.trim_end_matches('\0').to_string()),
                        Err(_) => Value::Array { offset, len },
                    }
                } else {
                    self.at += len;
                    Value::Array { offset, len }
                }
            }
            _ => Value::Other,
        })
    }
}

/// Collects scalar and text tags as `path = value` pairs.
fn flatten(node: &Node, prefix: &str, out: &mut Vec<(String, String)>) {
    for (i, (name, child)) in node.entries().iter().enumerate() {
        if out.len() >= MAX_PAIRS {
            return;
        }
        let key = if name.is_empty() { format!("{prefix}[{i}]") } else if prefix.is_empty() { name.clone() } else { format!("{prefix}.{name}") };
        match child {
            Node::Group(_) => flatten(child, &key, out),
            Node::Data(Value::Num(v)) => out.push((key, v.to_string())),
            Node::Data(Value::Text(t)) => out.push((key, t.clone())),
            _ => {}
        }
    }
}

/// Nanometres per unit of a length calibration.
fn length_nm(unit: &str) -> Option<f64> {
    Some(match unit.trim() {
        "nm" => 1.0,
        "µm" | "um" | "\u{3bc}m" | "micron" => 1e3,
        "Å" | "A" => 0.1,
        "pm" => 1e-3,
        "mm" => 1e6,
        "m" => 1e9,
        _ => return None,
    })
}

#[derive(Debug)]
struct Image {
    offset: u64,
    plane_bytes: u64,
}

/// DigitalMicrograph file.
#[derive(Debug)]
pub(super) struct Gatan {
    file: File,
    little: bool,
    images: Vec<Image>,
    metas: Vec<SeriesMeta>,
    original: Vec<OriginalMeta>,
}

impl Gatan {
    /// Opens a DM3 or DM4 file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid tag tree and
    /// [`Error::UnsupportedVariant`] for complex or RGB data.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut head = [0u8; 16];
        read_exact_at(&file, &mut head, 0)?;
        let version = u32::from_be_bytes(head[..4].try_into().expect("4 bytes"));
        let dm4 = match version {
            3 => false,
            4 => true,
            v => return Err(Error::UnsupportedVariant(format!("DM version {v}"))),
        };
        let (order_at, root_at) = if dm4 { (12, 16) } else { (8, 12) };
        let little = u32::from_be_bytes(head[order_at..order_at + 4].try_into().expect("4 bytes")) == 1;
        let mut p = Parser { file: &file, len, dm4, little, at: root_at, buf: Vec::new(), buf_at: 0, tags: 0 };
        let root = p.group(0)?;
        let list = root.get("ImageList").ok_or_else(|| corrupt("no ImageList"))?;
        let thumbs: Vec<usize> = root.get("Thumbnails").map(|t| t.entries().iter().filter_map(|(_, e)| e.get("ImageIndex")?.num()).map(|v| v as usize).collect()).unwrap_or_default();
        let entries = list.entries();
        let mut indices: Vec<usize> = (0..entries.len()).filter(|i| !thumbs.contains(i)).collect();
        // A file holding only thumbnails exposes them.
        if indices.is_empty() {
            indices = (0..entries.len()).collect();
        }
        let mut images = Vec::new();
        let mut metas = Vec::new();
        let mut originals = Vec::new();
        let mut first_error = None;
        for i in indices {
            let entry = &entries[i].1;
            match image(entry, len) {
                Ok((img, meta)) => {
                    images.push(img);
                    metas.push(meta);
                    let mut pairs = Vec::new();
                    if let Some(tags) = entry.get("ImageTags") {
                        flatten(tags, "ImageTags", &mut pairs);
                    }
                    originals.push(OriginalMeta { pairs, documents: Vec::new() });
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        if images.is_empty() {
            return Err(first_error.unwrap_or_else(|| corrupt("no image in ImageList")));
        }
        Ok(Gatan { file, little, images, metas, original: originals })
    }
}

/// Geometry and metadata of one `ImageList` entry.
fn image(entry: &Node, file_len: u64) -> Result<(Image, SeriesMeta)> {
    let data = entry.get("ImageData").ok_or_else(|| corrupt("image without ImageData"))?;
    let data_type = data.get("DataType").and_then(Node::num).ok_or_else(|| corrupt("image without DataType"))? as u32;
    let (pixel_type, spp) = match data_type {
        1 => (PixelType::I16, 1),
        2 => (PixelType::F32, 1),
        6 | 14 => (PixelType::U8, 1),
        7 => (PixelType::I32, 1),
        9 => (PixelType::I8, 1),
        10 => (PixelType::U16, 1),
        11 => (PixelType::U32, 1),
        12 => (PixelType::F64, 1),
        23 => (PixelType::U8, 4),
        t => return Err(Error::UnsupportedVariant(format!("DM data type {t}"))),
    };
    let dims: Vec<u64> = data.get("Dimensions").map(|d| d.entries().iter().filter_map(|(_, v)| v.num()).map(|v| v.max(1.0) as u64).collect()).unwrap_or_default();
    if dims.is_empty() || dims.len() > 8 {
        return Err(corrupt("image dimensions"));
    }
    let Some(Node::Data(Value::Array { offset, len, .. })) = data.get("Data") else {
        return Err(corrupt("image without Data array"));
    };
    let px = (pixel_type.bytes() * spp) as u64;
    let total = dims.iter().try_fold(px, |a, &d| a.checked_mul(d)).ok_or_else(|| corrupt("image too large"))?;
    if total > *len || offset + total > file_len {
        return Err(corrupt("Data array shorter than its dimensions"));
    }
    let cal: Vec<(f64, String)> = data
        .path("Calibrations.Dimension")
        .map(|d| d.entries().iter().map(|(_, c)| (c.get("Scale").and_then(Node::num).unwrap_or(0.0), c.get("Units").and_then(Node::text).unwrap_or_default().to_string())).collect())
        .unwrap_or_default();
    let unit = |i: usize| cal.get(i).map(|c| c.1.as_str()).unwrap_or("");
    let spacing = |i: usize| cal.get(i).and_then(|(s, u)| length_nm(u).map(|f| s.abs() * f)).filter(|v| v.is_finite() && *v > 0.0);
    let (w, h) = (dims[0], dims.get(1).copied().unwrap_or(1));
    let rest: u64 = dims.iter().skip(2).product();
    let tags = entry.get("ImageTags");
    let sequence = tags.and_then(|t| t.path("Meta Data.IsSequence")).and_then(Node::num).is_some_and(|v| v != 0.0);
    let third_unit = unit(2);
    let (size_c, size_z, size_t) = if dims.len() == 3 && matches!(third_unit.trim(), "eV" | "keV" | "meV") {
        (rest, 1, 1)
    } else if dims.len() == 3 && length_nm(third_unit).is_some() && !sequence {
        (1, rest, 1)
    } else {
        (1, 1, rest)
    };
    if [w, h, size_c, size_z, size_t].iter().any(|&v| v > u32::MAX as u64) {
        return Err(corrupt("image dimensions too large"));
    }
    let name = entry.get("Name").and_then(Node::text).filter(|n| !n.is_empty()).map(String::from);
    let meta = SeriesMeta {
        name,
        size_c: size_c as u32,
        size_z: size_z as u32,
        size_t: size_t as u32,
        pixel_type,
        samples_per_pixel: spp as u8,
        levels: vec![LevelMeta { size_x: w, size_y: h, tile: None, downsample: 1.0 }],
        physical_size: PhysicalSize { x_nm: spacing(0), y_nm: if dims.len() > 1 { spacing(1) } else { None }, z_nm: if size_z > 1 { spacing(2) } else { None } },
        channels: vec![ChannelMeta::default(); size_c as usize],
        ..Default::default()
    };
    Ok((Image { offset: *offset, plane_bytes: w * h * px }, meta))
}

impl Backend for Gatan {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        Ok(self.original[series].clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let img = &self.images[series];
        // The third and further dimensions follow the plane: c, z or t.
        let index = plane.c as u64 + m.size_c as u64 * (plane.z as u64 + m.size_z as u64 * plane.t as u64);
        let bytes = m.pixel_type.bytes();
        let px = bytes * m.samples_per_pixel as usize;
        let width = m.levels[0].size_x;
        let w = region.width as usize;
        let swap = bytes > 1 && self.little != cfg!(target_endian = "little");
        for r in 0..region.height as usize {
            let at = img.offset + index * img.plane_bytes + ((region.y + r as u64) * width + region.x) * px as u64;
            sink.write_with(r, 0, w, |d| {
                read_exact_at(&self.file, d, at)?;
                if swap {
                    d.chunks_exact_mut(bytes).for_each(<[u8]>::reverse);
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}

//! # lif
//!
//! Leica LIF reader (LAS X / LAS AF project files).
//!
//! A LIF file is a UTF-16 XML document describing a tree of elements,
//! followed by memory blocks holding raw, uncompressed samples. Each image
//! element is one series; the tiles of a tile scan (dimension 10) and the
//! indices of other non-spatial dimensions (6, 9, 11 and unknown ids) are
//! separate series. A lambda dimension (5) multiplies the channels; XT and
//! T slices (7, 8) fold into T. RGB images (channel tags 1-3 interleaved
//! in a pixel) are one colour channel. Every sample is addressed with the
//! `BytesInc` strides of its channel and dimensions.
//!
//! Not supported: FLIM raw data (`SingleMoleculeDetection`), LOF, XLIF and
//! the other XML containers of the family.
//!
//! Provenance: OpenReadout LIF format notes (MIT OR Apache-2.0; derived from
//! public samples and liffile's BSD-3 documentation) in `doc/leica-lif/`;
//! own analysis of sample files. No GPL code consulted (readlif is GPL and
//! was not used).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use roxmltree::{Document, Node};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta, PhysicalSize,
    PixelType, Plane, PlaneMeta, Position, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Lif, "Leica LIF", "1");

const MARKER: u32 = 0x70;
const TEXT: u8 = 0x2A;
/// Largest XML read, against corrupt files.
const MAX_XML_UNITS: usize = 512 << 20;

/// Returns `true` if `head` starts with a LIF header block.
pub(super) fn matches(head: &[u8]) -> bool {
    head.len() >= 14
        && u32::from_le_bytes(head[..4].try_into().expect("4 bytes")) == MARKER
        && head[8] == TEXT
        && head[13] == b'<'
}

/// Stride of one non-spatial index: (count, byte increment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Axis {
    count: u64,
    inc: u64,
}

impl Axis {
    const NONE: Axis = Axis { count: 1, inc: 0 };
}

/// One series: an image element, or one tile / split index of it.
#[derive(Debug, Clone)]
struct Image {
    /// Byte offset of the image's memory block plus the offset of this
    /// series' split index.
    base: u64,
    block_len: u64,
    width: u64,
    height: u64,
    x_inc: u64,
    y_inc: u64,
    /// Byte offset of each channel (lambda expanded), first sample.
    channels: Vec<u64>,
    /// Byte offsets of the samples of a colour channel, in R, G, B order.
    rgb: Option<[u64; 3]>,
    z: Axis,
    /// T axes, innermost first (T, then XT slices, then T slices).
    t: Vec<Axis>,
    bytes: usize,
    /// Half floats, widened to f32 on read.
    half: bool,
}

/// Leica LIF file.
#[derive(Debug)]
pub(super) struct Lif {
    file: File,
    xml_at: u64,
    xml_units: usize,
    images: Vec<Image>,
    metas: Vec<SeriesMeta>,
    /// Frame times of each series in seconds since its first frame.
    times: Vec<Vec<f64>>,
}

impl Lif {
    /// Opens a LIF file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header, XML or block
    /// structure and [`Error::UnsupportedVariant`] for files without images.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut head = [0u8; 13];
        if len < 13 {
            return Err(Error::Corrupt("LIF file too short".into()));
        }
        read_exact_at(&file, &mut head, 0)?;
        let units = u32::from_le_bytes(head[9..13].try_into().expect("4 bytes")) as usize;
        if !matches(&[&head[..], b"<"].concat()) || units > MAX_XML_UNITS || 13 + 2 * units as u64 > len {
            return Err(Error::Corrupt("invalid LIF header".into()));
        }
        let xml = read_xml(&file, 13, units)?;
        let blocks = memory_blocks(&file, len, 13 + 2 * units as u64);
        let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
        let doc = Document::parse_with_options(&xml, options).map_err(|e| Error::Corrupt(format!("LIF XML: {e}")))?;
        let version = doc.root_element().attribute("Version").and_then(|v| v.parse::<u32>().ok()).unwrap_or(2);
        let _ = version;
        let mut lif = Lif { file, xml_at: 13, xml_units: units, images: Vec::new(), metas: Vec::new(), times: Vec::new() };
        let root = doc.root_element().children().find(|c| c.has_tag_name("Element"));
        if let Some(root) = root {
            lif.walk(root, "", &blocks, true, 0)?;
        }
        if lif.images.is_empty() {
            return Err(Error::UnsupportedVariant("LIF without readable images".into()));
        }
        Ok(lif)
    }

    /// Adds the images of `element` and its children, in document order.
    fn walk(&mut self, element: Node, prefix: &str, blocks: &HashMap<String, (u64, u64)>, top: bool, depth: usize) -> Result<()> {
        if depth > 64 {
            return Err(Error::Corrupt("LIF elements nested too deeply".into()));
        }
        let name = element.attribute("Name").unwrap_or("");
        // The top element is the project; its name is not part of image names.
        let path = if top { String::new() } else if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
        if let Some(image) = child(element, "Data").and_then(|d| child(d, "Image"))
            && let Some(desc) = child(image, "ImageDescription") {
                let memory = child(element, "Memory");
                let id = memory.and_then(|m| m.attribute("MemoryBlockID")).unwrap_or("");
                let size: u64 = memory.and_then(|m| m.attribute("Size")).and_then(|s| s.parse().ok()).unwrap_or(0);
                if let Some(&(offset, len)) = blocks.get(id).filter(|_| size > 0) {
                    self.add_image(element, image, desc, &path, offset, len.min(size))?;
                }
            }
        if let Some(children) = child(element, "Children") {
            for c in children.children().filter(|c| c.has_tag_name("Element")) {
                self.walk(c, &path, blocks, false, depth + 1)?;
            }
        }
        Ok(())
    }

    fn add_image(&mut self, element: Node, image: Node, desc: Node, name: &str, offset: u64, block_len: u64) -> Result<()> {
        let num = |n: Node, a: &str| n.attribute(a).and_then(|v| v.trim().parse::<f64>().ok());
        let int = |n: Node, a: &str| n.attribute(a).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
        let chans: Vec<Node> = child(desc, "Channels").map(|c| c.children().filter(|c| c.has_tag_name("ChannelDescription")).collect()).unwrap_or_default();
        let dims: Vec<Node> = child(desc, "Dimensions").map(|c| c.children().filter(|c| c.has_tag_name("DimensionDescription")).collect()).unwrap_or_default();
        if chans.is_empty() || dims.is_empty() {
            return Ok(());
        }
        let dim = |id: u64| dims.iter().find(|d| int(**d, "DimID") == id);
        let axis = |d: Option<&Node>| d.map(|d| Axis { count: int(*d, "NumberOfElements").max(1), inc: int(*d, "BytesInc") }).unwrap_or(Axis::NONE);
        let (Some(dx), dy) = (dim(1), dim(2)) else { return Ok(()) };
        let (x, y) = (axis(Some(dx)), axis(dy));
        // Sample type: all channels must agree.
        let res = int(chans[0], "Resolution");
        let float = int(chans[0], "DataType") == 1;
        if chans.iter().any(|c| int(*c, "Resolution") != res || (int(*c, "DataType") == 1) != float) {
            return Err(Error::UnsupportedVariant(format!("LIF image {name} with mixed channel types")));
        }
        let (pixel_type, half) = match (res, float) {
            (1..=8, false) => (PixelType::U8, false),
            (9..=16, false) => (PixelType::U16, false),
            (17..=32, false) => (PixelType::U32, false),
            (16, true) => (PixelType::F32, true),
            (32, true) => (PixelType::F32, false),
            (64, true) => (PixelType::F64, false),
            (r, f) => return Err(Error::UnsupportedVariant(format!("LIF {r}-bit {} samples", if f { "float" } else { "integer" }))),
        };
        let bytes = if half { 2 } else { pixel_type.bytes() };
        // Channels in storage order; RGB when tags 1-3 lie within one pixel.
        let mut order: Vec<Node> = chans.clone();
        order.sort_by_key(|c| int(*c, "BytesInc"));
        let tags: Vec<u64> = order.iter().map(|c| int(*c, "ChannelTag")).collect();
        let rgb = (order.len() == 3 && tags.iter().all(|t| (1..=3).contains(t)) && int(order[2], "BytesInc") < x.inc.max(1)).then(|| {
            let at = |tag: u64| order.iter().find(|c| int(**c, "ChannelTag") == tag).map(|c| int(*c, "BytesInc")).unwrap_or(0);
            [at(1), at(2), at(3)]
        });
        let lambda = axis(dim(5));
        let channel_offsets: Vec<u64> = if rgb.is_some() {
            vec![0]
        } else {
            order.iter().flat_map(|c| (0..lambda.count).map(move |l| int(*c, "BytesInc") + l * lambda.inc)).collect()
        };
        let z = axis(dim(3));
        let t: Vec<Axis> = [4, 7, 8].iter().map(|&id| axis(dim(id))).filter(|a| a.count > 1).collect();
        // Split axes: tiles and other non-spatial dimensions, one series per index.
        let splits: Vec<(u64, Axis)> = dims
            .iter()
            .map(|d| (int(*d, "DimID"), axis(Some(d))))
            .filter(|(id, a)| !matches!(id, 1..=5 | 7 | 8) && a.count > 1)
            .collect();
        let size_t = t.iter().map(|a| a.count).product::<u64>().max(1);

        // Metadata shared by all split series.
        let step_nm = |d: Option<&Node>| -> Option<f64> {
            let d = d?;
            let n = int(*d, "NumberOfElements");
            let len = num(*d, "Length")?.abs();
            let step = len / (n.max(2) - 1) as f64;
            // Pixel indices labelled metres, or no extent recorded.
            (n > 1 && step > 0.0 && step < 1e-2 && d.attribute("Unit").is_none_or(|u| u == "m" || u.is_empty())).then_some(step * 1e9)
        };
        let t_inc = dim(4).and_then(|d| {
            let n = int(*d, "NumberOfElements");
            let len = num(*d, "Length")?;
            (n > 1 && len > 0.0).then(|| len / (n - 1) as f64)
        });
        let lambda_nm: Vec<Option<f64>> = (0..lambda.count)
            .map(|i| {
                let d = dim(5)?;
                let (origin, len) = (num(*d, "Origin")?, num(*d, "Length").unwrap_or(0.0));
                let n = lambda.count.max(2) - 1;
                Some((origin + i as f64 * len / n as f64) * 1e9)
            })
            .collect();
        let channels: Vec<ChannelMeta> = if rgb.is_some() {
            vec![ChannelMeta::default()]
        } else {
            order
                .iter()
                .flat_map(|c| {
                    let lut = c.attribute("LUTName").map(str::trim).filter(|s| !s.is_empty()).map(String::from);
                    let lambda_nm = &lambda_nm;
                    (0..lambda.count as usize).map(move |l| ChannelMeta {
                        name: match (&lut, lambda_nm.get(l).copied().flatten().filter(|_| lambda_nm.len() > 1)) {
                            (Some(n), Some(w)) => Some(format!("{n} {w:.0} nm")),
                            (n, _) => n.clone(),
                        },
                        color: lut.as_deref().and_then(lut_color),
                        emission_wavelength_nm: lambda_nm.get(l).copied().flatten().filter(|_| lambda_nm.len() > 1),
                        ..Default::default()
                    })
                })
                .collect()
        };
        let objective = element.descendants().find(|n| n.attribute("ObjectiveName").is_some()).map(|o| ObjectiveMeta {
            model: o.attribute("ObjectiveName").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            magnification: num(o, "Magnification").filter(|m| *m > 0.0),
            numerical_aperture: num(o, "NumericalAperture").filter(|m| *m > 0.0),
            immersion: o.attribute("Immersion").map(|i| match i {
                "DRY" => "Air".to_string(),
                "OIL" => "Oil".to_string(),
                "WATER" => "Water".to_string(),
                "GLYCERIN" | "GLYC" => "Glycerol".to_string(),
                other => other.to_string(),
            }),
            ..Default::default()
        });
        let stamps = time_stamps(image);
        let date = stamps.first().and_then(|&ft| filetime_iso(ft));
        let times: Vec<f64> = stamps.iter().map(|&ft| (ft as f64 - stamps[0] as f64) / 1e7).collect();
        let tiles = tile_positions(image);
        let significant = (res as usize) < pixel_type.bytes() * 8 && !float;

        // One series per split index; the tile index is the first split
        // axis that is the mosaic.
        let split_count: u64 = splits.iter().map(|(_, a)| a.count).product::<u64>().max(1);
        let base_name = name.to_string();
        for k in 0..split_count {
            let mut rem = k;
            let mut base = offset;
            let mut labels = Vec::new();
            let mut tile = None;
            for (id, a) in &splits {
                let i = rem % a.count;
                rem /= a.count;
                base += i * a.inc;
                if *id == 10 {
                    tile = Some(i as usize);
                }
                labels.push(match id {
                    10 => format!("tile {}", i + 1),
                    6 => format!("rotation {i}"),
                    _ => format!("dim{id} {i}"),
                });
            }
            let img = Image {
                base,
                block_len,
                width: x.count,
                height: y.count,
                x_inc: x.inc,
                y_inc: y.inc,
                channels: channel_offsets.clone(),
                rgb,
                z,
                t: t.clone(),
                bytes,
                half,
            };
            img.check(size_t, offset)?;
            let position = tile.and_then(|i| tiles.get(i)).map(|p| Position { x_nm: Some(p[0] * 1e9), y_nm: Some(p[1] * 1e9), z_nm: Some(p[2] * 1e9) });
            self.images.push(img);
            self.times.push(times.clone());
            self.metas.push(SeriesMeta {
                name: Some(if labels.is_empty() { base_name.clone() } else { format!("{base_name} [{}]", labels.join(", ")) }),
                acquisition_date: date.clone(),
                size_c: channel_offsets.len() as u32,
                size_z: z.count as u32,
                size_t: size_t as u32,
                pixel_type,
                significant_bits: significant.then_some(res as u8),
                samples_per_pixel: if rgb.is_some() { 3 } else { 1 },
                levels: vec![LevelMeta { size_x: x.count, size_y: y.count, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: step_nm(Some(dx)), y_nm: step_nm(dy), z_nm: step_nm(dim(3)) },
                time_increment_s: t_inc,
                channels: channels.clone(),
                objective: objective.clone(),
                stage_position: position,
                ..Default::default()
            });
        }
        Ok(())
    }
}

impl Image {
    /// Byte offset of the first sample of plane (c, z, t), relative to the file.
    fn plane_offset(&self, plane: Plane) -> u64 {
        let mut t = plane.t as u64;
        let mut off = self.base + self.channels[plane.c as usize] + plane.z as u64 * self.z.inc;
        for a in &self.t {
            off += (t % a.count) * a.inc;
            t /= a.count;
        }
        off
    }

    fn pixel_bytes(&self) -> usize {
        self.bytes * if self.rgb.is_some() { 3 } else { 1 }
    }

    /// Checks that the last sample of the image lies within its block.
    fn check(&self, size_t: u64, block: u64) -> Result<()> {
        let last = Plane::new(self.channels.len() as u32 - 1, (self.z.count - 1) as u32, (size_t - 1) as u32);
        let max_c = self.channels.iter().max().copied().unwrap_or(0);
        let mut end = self.plane_offset(last) - self.channels[last.c as usize] + max_c;
        // Last pixel, its last sample.
        end += (self.width - 1) * self.x_inc + (self.height - 1) * self.y_inc + self.bytes as u64;
        if let Some(rgb) = self.rgb {
            end += rgb.iter().max().copied().unwrap_or(0);
        }
        if end > block + self.block_len {
            return Err(Error::Corrupt(format!("LIF image extends beyond its memory block ({end} > {})", block + self.block_len)));
        }
        Ok(())
    }
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(name))
}

/// Reads `units` UTF-16 code units at `at`.
fn read_xml(file: &File, at: u64, units: usize) -> Result<String> {
    let mut raw = vec![0u8; units * 2];
    read_exact_at(file, &mut raw, at)?;
    let u: Vec<u16> = raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
    Ok(String::from_utf16_lossy(&u))
}

/// Memory blocks by id: payload offset and length. Stops at the first
/// block that does not parse.
fn memory_blocks(file: &File, len: u64, mut at: u64) -> HashMap<String, (u64, u64)> {
    let mut map = HashMap::new();
    let mut head = [0u8; 22];
    while at + 22 <= len {
        if read_exact_at(file, &mut head, at).is_err() || u32::from_le_bytes(head[..4].try_into().expect("4 bytes")) != MARKER || head[8] != TEXT {
            break;
        }
        // Version 2: u64 data length; version 1: u32.
        let (data_len, id_at) = if head[17] == TEXT {
            (u64::from_le_bytes(head[9..17].try_into().expect("8 bytes")), 18)
        } else if head[13] == TEXT {
            (u32::from_le_bytes(head[9..13].try_into().expect("4 bytes")) as u64, 14)
        } else {
            break;
        };
        let id_units = u32::from_le_bytes(head[id_at..id_at + 4].try_into().expect("4 bytes")) as u64;
        if id_units > 1024 {
            break;
        }
        let mut id = vec![0u8; id_units as usize * 2];
        if read_exact_at(file, &mut id, at + id_at as u64 + 4).is_err() {
            break;
        }
        let units: Vec<u16> = id.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        let start = at + id_at as u64 + 4 + id_units * 2;
        let data_len = data_len.min(len.saturating_sub(start));
        map.insert(String::from_utf16_lossy(&units), (start, data_len));
        at = start + data_len;
    }
    map
}

/// Display colour of a LUT name.
fn lut_color(name: &str) -> Option<[u8; 4]> {
    Some(match name.to_ascii_lowercase().as_str() {
        "red" => [255, 0, 0, 255],
        "green" => [0, 255, 0, 255],
        "blue" => [0, 0, 255, 255],
        "cyan" => [0, 255, 255, 255],
        "magenta" => [255, 0, 255, 255],
        "yellow" => [255, 255, 0, 255],
        "gray" | "grey" => [255, 255, 255, 255],
        _ => return None,
    })
}

/// FILETIMEs of the `TimeStampList`: hex text (LAS X) or `TimeStamp`
/// children with high and low halves (LAS AF).
fn time_stamps(image: Node) -> Vec<u64> {
    let Some(list) = child(image, "TimeStampList") else { return Vec::new() };
    let text: Vec<u64> = list.text().unwrap_or("").split_whitespace().filter_map(|h| u64::from_str_radix(h, 16).ok()).collect();
    if !text.is_empty() {
        return text;
    }
    list.children()
        .filter(|c| c.has_tag_name("TimeStamp"))
        .filter_map(|c| {
            let hi: u64 = c.attribute("HighInteger")?.parse().ok()?;
            let lo: u64 = c.attribute("LowInteger")?.parse().ok()?;
            Some(hi << 32 | lo)
        })
        .collect()
}

/// ISO 8601 of a Windows FILETIME (100 ns since 1601), seconds precision.
fn filetime_iso(ft: u64) -> Option<String> {
    let secs = (ft / 10_000_000) as i64 - 11_644_473_600;
    if !(0..=4_102_444_800).contains(&secs) {
        return None;
    }
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    Some(format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60))
}

/// Stage positions (metres) of the tiles of a tile scan, in tile order.
fn tile_positions(image: Node) -> Vec<[f64; 3]> {
    let Some(info) = image.children().find(|c| c.has_tag_name("Attachment") && c.attribute("Name") == Some("TileScanInfo")) else {
        return Vec::new();
    };
    let num = |n: Node, a: &str| n.attribute(a).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    info.children().filter(|c| c.has_tag_name("Tile")).map(|t| [num(t, "PosX"), num(t, "PosY"), num(t, "PosZ")]).collect()
}

/// IEEE half float bits as f32.
fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) as u32) << 31;
    let exp = ((h >> 10) & 0x1F) as u32;
    let frac = (h & 0x3FF) as u32;
    let bits = match (exp, frac) {
        (0, 0) => sign,
        (0, f) => {
            // Subnormal: normalise.
            let shift = f.leading_zeros() - 21;
            sign | ((113 - shift) << 23) | ((f << (shift + 1)) & 0x3FF) << 13
        }
        (31, 0) => sign | 0x7F80_0000,
        (31, f) => sign | 0x7FC0_0000 | f << 13,
        (e, f) => sign | (e + 112) << 23 | f << 13,
    };
    f32::from_bits(bits)
}

impl Backend for Lif {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        // Time stamps are per frame: c fastest... recorded per acquired
        // plane in storage order; use the frame of (z, t) for channel 0.
        let frame = (plane.t as usize * m.size_z as usize + plane.z as usize) * m.size_c as usize;
        Ok(PlaneMeta { delta_t_s: self.times[series].get(frame).copied(), ..Default::default() })
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        if let Some(name) = &self.metas[series].name {
            meta.pairs.push(("Image name".into(), name.clone()));
        }
        meta.documents.push(("LIF-XML".into(), read_xml(&self.file, self.xml_at, self.xml_units)?));
        Ok(meta)
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let img = &self.images[series];
        let base = img.plane_offset(plane);
        let (x0, y0, w, h) = (region.x, region.y, region.width as usize, region.height as usize);
        let px = img.pixel_bytes();
        let n = img.bytes;
        // Contiguous gray rows go straight into the destination.
        if img.rgb.is_none() && !img.half && img.x_inc == n as u64 {
            for r in 0..h {
                let at = base + (y0 + r as u64) * img.y_inc + x0 * img.x_inc;
                sink.write_with(r, 0, w, |dst| read_exact_at(&self.file, dst, at))?;
            }
            return Ok(());
        }
        // Otherwise read the row span and gather samples.
        let extra = img.rgb.map(|o| o.iter().max().copied().unwrap_or(0)).unwrap_or(0) as usize;
        let span = (w - 1) * img.x_inc as usize + extra + n;
        scratch::with(span, |row| {
            for r in 0..h {
                let at = base + (y0 + r as u64) * img.y_inc + x0 * img.x_inc;
                read_exact_at(&self.file, row, at)?;
                sink.write_with(r, 0, w, |dst| {
                    let out = if img.half { 4 } else { n };
                    for (i, d) in dst.chunks_exact_mut(px / n * out).enumerate() {
                        let p = i * img.x_inc as usize;
                        let offsets: &[u64] = match &img.rgb {
                            Some(o) => o,
                            None => &[0],
                        };
                        for (k, &o) in offsets.iter().enumerate() {
                            let s = &row[p + o as usize..p + o as usize + n];
                            if img.half {
                                d[k * 4..k * 4 + 4].copy_from_slice(&half_to_f32(u16::from_le_bytes([s[0], s[1]])).to_ne_bytes());
                            } else {
                                d[k * n..(k + 1) * n].copy_from_slice(s);
                            }
                        }
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats() {
        assert_eq!(half_to_f32(0x3C00), 1.0);
        assert_eq!(half_to_f32(0xC000), -2.0);
        assert_eq!(half_to_f32(0x7BFF), 65504.0);
        assert_eq!(half_to_f32(0x0001), 2f32.powi(-24));
        assert!(half_to_f32(0x7E00).is_nan());
        assert_eq!(half_to_f32(0xFC00), f32::NEG_INFINITY);
    }

    #[test]
    fn filetimes() {
        // 2019-10-25T10:00:00Z
        assert_eq!(filetime_iso(132_164_712_000_000_000).as_deref(), Some("2019-10-25T10:00:00Z"));
        assert_eq!(filetime_iso(0), None);
    }
}

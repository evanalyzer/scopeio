//! # oir
//!
//! Olympus/Evident OIR reader (FluoView FV3000/FV4000).
//!
//! An OIR file is a chain of blocks listed in an index: XML document blocks
//! (the last one holds the complete metadata), one frame-properties block
//! per frame, a thumbnail and pixel chunks. Each plane (one channel at one
//! t, λ, z) is stored as chunks, each a tag block naming the plane
//! (`l002z001t003_0_1_<channel uuid>_<n>`) with the chunk's offset in the
//! plane, followed by a pixel block. Large acquisitions continue in files
//! `<stem>_00001`, `<stem>_00002`, … next to the `.oir`. Planes are read
//! by positioned reads of the chunks a region needs.
//!
//! Lambda bands are channels (channel-major); reference images (`REF_`
//! chunks) and the thumbnail are not read. Pixel sizes from
//! `imageProperties`, the z step from the ZSTACK axis, the time increment
//! from the frames' TIMELAPSE positions.
//!
//! Provenance: OpenReadout OIR notes (MIT OR Apache-2.0) in
//! `doc/olympus-oir/`; `oirfile` (BSD-3-Clause, Christoph Gohlke, see
//! THIRD_PARTY_NOTICES.md) read as documentation; own analysis of the OME
//! sample files (CC BY 4.0); checked against Bio-Formats as a black box. No
//! GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::path::Path;

use roxmltree::{Document, Node};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, PixelType, Plane,
    Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Oir, "Olympus OIR", "1");

const MAGIC: &[u8] = b"OLYMPUSRAWFORMAT";
const FIRST_BLOCK: u64 = 96;
const INDEX_MARKER: u32 = 0xFFFF_FFFF;
const DOCUMENTS: u32 = 0;
const FRAME_PROPERTIES: u32 = 1;
const CHUNK_TAG: u32 = 3;
const PIXELS: u32 = 4;
/// Largest XML block parsed.
const MAX_XML: u64 = 64 << 20;
/// Most blocks indexed per file, against corrupt counts.
const MAX_BLOCKS: u64 = 1 << 24;

/// Returns `true` if `head` starts with the OIR signature.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(MAGIC)
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

/// One pixel chunk of a plane.
#[derive(Debug, Clone, Copy)]
struct Chunk {
    /// File (0 = the `.oir`, k = continuation `_0000k`).
    part: u16,
    /// Byte offset in the plane.
    plane_offset: u64,
    /// Position and length of the pixel data in the file.
    at: u64,
    len: u64,
}

/// Blocks of one OIR file.
#[derive(Default)]
struct Blocks {
    /// Plane name and chunk, in file order.
    chunks: Vec<(String, Chunk)>,
    /// Payload position and length of frame-properties blocks.
    frames: Vec<(u64, u64)>,
    /// The last documents block.
    documents: Option<(u64, u64)>,
}

/// Indexes the blocks of `file` (part `part`).
fn blocks(file: &File, part: u16) -> Result<Blocks> {
    let len = file.metadata()?.len();
    let mut h = [0u8; 96];
    if len < 96 {
        return Err(Error::Corrupt("OIR file too short".into()));
    }
    read_exact_at(file, &mut h, 0)?;
    if !matches(&h) {
        return Err(Error::Corrupt("not an OIR file".into()));
    }
    let declared = u64_at(&h, 32).min(len);
    let index = u64_at(&h, 40);
    // Block offsets from the index, else by walking the chain.
    let mut offsets = Vec::new();
    let mut marker = [0u8; 4];
    if index >= FIRST_BLOCK && index + 4 <= declared && read_exact_at(file, &mut marker, index).is_ok() && u32::from_le_bytes(marker) == INDEX_MARKER {
        let n = ((declared - index - 4) / 8).min(MAX_BLOCKS) as usize;
        let mut raw = vec![0u8; n * 8];
        read_exact_at(file, &mut raw, index + 4)?;
        offsets.extend(raw.chunks_exact(8).map(|b| u64_at(b, 0)).take_while(|&o| o >= FIRST_BLOCK && o < declared));
    } else {
        let mut pos = FIRST_BLOCK;
        let mut head = [0u8; 8];
        while pos + 8 <= declared && (offsets.len() as u64) < MAX_BLOCKS {
            read_exact_at(file, &mut head, pos)?;
            offsets.push(pos);
            pos += 8 + u32_at(&head, 0) as u64;
        }
    }
    let mut out = Blocks::default();
    let mut i = 0;
    let mut head = [0u8; 20];
    while i < offsets.len() {
        let at = offsets[i];
        let n = (len - at).min(20) as usize;
        if n < 8 {
            break;
        }
        read_exact_at(file, &mut head[..n], at)?;
        let (payload, kind) = (u32_at(&head, 0) as u64, u32_at(&head, 4));
        if at + 8 + payload > len {
            return Err(Error::Corrupt(format!("OIR block at {at} beyond the end")));
        }
        match kind {
            CHUNK_TAG if n == 20 && payload >= 12 => {
                let (plane_offset, name_len) = (u32_at(&head, 8) as u64, u32_at(&head, 16) as usize);
                if name_len as u64 > payload - 12 || name_len > 4096 {
                    return Err(Error::Corrupt(format!("OIR chunk tag at {at} with bad name")));
                }
                let mut name = vec![0u8; name_len];
                read_exact_at(file, &mut name, at + 20)?;
                // The pixel block follows the tag.
                if let Some(&next) = offsets.get(i + 1) {
                    let mut p = [0u8; 8];
                    read_exact_at(file, &mut p, next)?;
                    if u32_at(&p, 4) == PIXELS {
                        let plen = u32_at(&p, 0) as u64;
                        if next + 8 + plen > len {
                            return Err(Error::Corrupt(format!("OIR pixel block at {next} beyond the end")));
                        }
                        let chunk = Chunk { part, plane_offset, at: next + 8, len: plen };
                        out.chunks.push((String::from_utf8_lossy(&name).into_owned(), chunk));
                        i += 2;
                        continue;
                    }
                }
            }
            FRAME_PROPERTIES => out.frames.push((at + 8, payload)),
            DOCUMENTS => out.documents = Some((at + 8, payload)),
            _ => {}
        }
        i += 1;
    }
    Ok(out)
}

/// Reads `len` bytes at `at`, bounded against corrupt sizes.
fn read_bytes(file: &File, at: u64, len: u64) -> Result<Vec<u8>> {
    if len > MAX_XML {
        return Err(Error::Corrupt("OIR metadata block too large".into()));
    }
    let mut b = vec![0u8; len as usize];
    read_exact_at(file, &mut b, at)?;
    Ok(b)
}

/// The XML documents of a documents or frame-properties payload.
fn documents(payload: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut starts: Vec<usize> = payload.windows(5).enumerate().filter(|(_, w)| *w == b"<?xml").map(|(i, _)| i).collect();
    starts.push(payload.len());
    for w in starts.windows(2) {
        let doc = &payload[w[0]..w[1]];
        let end = doc.iter().rposition(|&b| b == b'>').map_or(0, |e| e + 1);
        out.push(String::from_utf8_lossy(&doc[..end]).into_owned());
    }
    out
}

/// The start of an XML document, where its root element is named.
fn head(doc: &str) -> &str {
    let mut end = doc.len().min(300);
    while !doc.is_char_boundary(end) {
        end -= 1;
    }
    &doc[..end]
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

/// A plane's (t, λ, z) indices (absent axes 0) and channel uuid.
type PlaneKey = ((u32, u32, u32), String);

/// Plane indices of a chunk name.
fn chunk_name(name: &str) -> Option<PlaneKey> {
    if name.starts_with("REF_") {
        return None;
    }
    // `<axes>_<a>_<b>_<uuid>_<n>`: the uuid is the second to last part.
    let mut parts: Vec<&str> = name.rsplitn(3, '_').collect();
    parts.reverse();
    let [head, uuid, n] = parts[..] else { return None };
    if n.parse::<u32>().is_err() || uuid.len() < 8 {
        return None;
    }
    // Planes without t, λ or z have no axis letters.
    let axes = head.split('_').next().filter(|a| a.starts_with(|c: char| c.is_ascii_alphabetic())).unwrap_or("");
    let (mut t, mut l, mut z) = (0, 0, 0);
    let mut chars = axes.chars().peekable();
    while let Some(c) = chars.next() {
        let mut digits = String::new();
        while let Some(d) = chars.next_if(|d| d.is_ascii_digit()) {
            digits.push(d);
        }
        let v: u32 = digits.parse().ok()?;
        match c {
            't' => t = v,
            'l' => l = v,
            'z' => z = v,
            _ => return None,
        }
    }
    Some(((t, l, z), uuid.to_string()))
}

/// Nanometres per unit of an OIR pixel unit.
fn unit_nm(unit: &str) -> Option<f64> {
    match unit {
        "NANO_METER" => Some(1.0),
        "MICRO_METER" => Some(1e3),
        "MILLI_METER" => Some(1e6),
        _ => None,
    }
}

/// Olympus OIR acquisition.
#[derive(Debug)]
pub(super) struct Oir {
    parts: Vec<File>,
    /// Chunks of each plane (index `c + size_c * (z + size_z * t)`), sorted
    /// by plane offset.
    planes: Vec<Vec<Chunk>>,
    bytes: usize,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
}

impl Oir {
    /// Opens an OIR file and its continuation files.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid block structures and
    /// [`Error::UnsupportedVariant`] for files without image planes (overview
    /// maps) or colour planes.
    pub fn open(path: &Path) -> Result<Self> {
        let main = File::open(path)?;
        let first = blocks(&main, 0)?;
        let mut parts = vec![main];
        let mut all_chunks = first.chunks;
        let mut frames: Vec<(u16, u64, u64)> = first.frames.iter().map(|&(a, l)| (0, a, l)).collect();
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("oir")) {
            let stem = path.with_extension("");
            for k in 1..u16::MAX {
                let Ok(f) = File::open(format!("{}_{k:05}", stem.display())) else { break };
                let b = blocks(&f, k)?;
                all_chunks.extend(b.chunks);
                frames.extend(b.frames.iter().map(|&(a, l)| (k, a, l)));
                parts.push(f);
            }
        }
        let (doc_at, doc_len) = first.documents.ok_or_else(|| Error::Corrupt("OIR without metadata".into()))?;
        let docs = documents(&read_bytes(&parts[0], doc_at, doc_len)?);
        let doc = |root: &str| docs.iter().find(|d| head(d).contains(&format!("<{root}")));
        let props_xml = doc("lsmimage:imageProperties").ok_or_else(|| Error::Corrupt("OIR without image properties".into()))?;
        let props = Document::parse(props_xml).map_err(|e| Error::Corrupt(format!("OIR image properties: {e}")))?;
        let root = props.root_element();

        // Planes by (t, λ, z, channel).
        let mut by_plane: BTreeMap<PlaneKey, Vec<Chunk>> = BTreeMap::new();
        for (name, chunk) in all_chunks {
            if let Some(key) = chunk_name(&name) {
                by_plane.entry(key).or_default().push(chunk);
            }
        }
        if by_plane.is_empty() {
            return Err(Error::UnsupportedVariant("OIR without image planes (overview map)".into()));
        }
        let distinct = |f: fn(&(u32, u32, u32)) -> u32| -> Vec<u32> { by_plane.keys().map(|(k, _)| f(k)).collect::<BTreeSet<_>>().into_iter().collect() };
        let (ts, ls, zs) = (distinct(|k| k.0), distinct(|k| k.1), distinct(|k| k.2));
        // Channels: imageInfo order, then channels without description by uuid.
        let with_pixels: BTreeSet<&str> = by_plane.keys().map(|(_, c)| c.as_str()).collect();
        let info = root.descendants().find(|n| local(*n) == "imageInfo");
        let mut described: Vec<(i64, String, Node)> = Vec::new();
        for n in info.into_iter().flat_map(|i| i.descendants()).filter(|n| local(*n) == "channel") {
            if let Some(id) = n.attribute("id")
                && with_pixels.contains(id)
                && !described.iter().any(|(_, d, _)| d == id)
            {
                described.push((n.attribute("order").and_then(|o| o.parse().ok()).unwrap_or(0), id.to_string(), n));
            }
        }
        described.sort_by_key(|(o, _, _)| *o);
        let mut channel_ids: Vec<String> = described.iter().map(|(_, id, _)| id.clone()).collect();
        channel_ids.extend(with_pixels.iter().filter(|c| !described.iter().any(|(_, d, _)| d == *c)).map(|c| c.to_string()));
        let (size_c, size_z, size_t) = (channel_ids.len() * ls.len(), zs.len(), ts.len());
        let mut planes = vec![Vec::new(); size_c * size_z * size_t];
        for (((t, l, z), ch), mut chunks) in by_plane {
            let (Some(ti), Some(li), Some(zi), Some(ci)) = (ts.iter().position(|&v| v == t), ls.iter().position(|&v| v == l), zs.iter().position(|&v| v == z), channel_ids.iter().position(|c| *c == ch)) else {
                continue;
            };
            chunks.sort_by_key(|c| c.plane_offset);
            planes[ci * ls.len() + li + size_c * (zi + size_z * ti)] = chunks;
        }

        // Geometry from the first frame.
        let &(fpart, fat, flen) = frames.first().ok_or_else(|| Error::Corrupt("OIR without frame properties".into()))?;
        let frame_doc = |k: usize| -> Option<String> {
            let &(p, a, l) = frames.get(k)?;
            read_bytes(&parts[p as usize], a, l).ok().and_then(|b| documents(&b).into_iter().next())
        };
        let frame0 = documents(&read_bytes(&parts[fpart as usize], fat, flen)?).into_iter().next().ok_or_else(|| Error::Corrupt("OIR frame properties without XML".into()))?;
        let f0 = Document::parse(&frame0).map_err(|e| Error::Corrupt(format!("OIR frame properties: {e}")))?;
        let def = f0.descendants().find(|n| local(*n) == "imageDefinition").ok_or_else(|| Error::Corrupt("OIR frame without image definition".into()))?;
        let width = num(def, "width").unwrap_or(0.0) as u64;
        let height = num(def, "height").unwrap_or(0.0) as u64;
        let depth = num(def, "depth").unwrap_or(0.0) as usize;
        let pixel_type = match depth {
            1 => PixelType::U8,
            2 => PixelType::U16,
            4 => PixelType::F32,
            d => return Err(Error::UnsupportedVariant(format!("OIR with {d} bytes per sample"))),
        };
        if text(def, "colorType").is_some_and(|c| c == "RGB") {
            return Err(Error::UnsupportedVariant("OIR colour planes".into()));
        }
        if width == 0 || height == 0 || width > 1 << 20 || height > 1 << 20 {
            return Err(Error::Corrupt(format!("OIR plane of {width} x {height}")));
        }
        // Line scans store more rows than the frame height.
        let rows = planes.iter().map(|p| p.iter().map(|c| c.plane_offset + c.len).max().unwrap_or(0) / (width * depth as u64)).max().unwrap_or(0);
        let height = height.max(rows);
        let bits = num(def, "bitCounts").map(|b| b as u8).filter(|&b| b > 0 && (b as usize) < depth * 8);

        // Physical sizes and axes.
        let length = root.descendants().find(|n| local(*n) == "length");
        let unit = root.descendants().find(|n| local(*n) == "pixelUnit");
        let size = |axis: &str| -> Option<f64> {
            let v = num(length?, axis)?;
            // Older files have no unit: micrometres.
            let f = match unit {
                Some(u) => unit_nm(&text(u, axis)?)?,
                None => 1e3,
            };
            (v > 0.0).then_some(v * f)
        };
        let axis = |name: &str| -> Option<Node> {
            info?.descendants().filter(|n| local(*n) == "axis").find(|n| text(*n, "axis").as_deref() == Some(name))
        };
        let z_nm = axis("ZSTACK").and_then(|a| num(a, "step")).map(f64::abs).filter(|s| *s > 0.0 && size_z > 1).map(|s| s * 1e3);
        let frame_time = |k: usize| -> Option<f64> {
            let xml = frame_doc(k)?;
            let d = Document::parse(&xml).ok()?;
            let v = d.descendants().filter(|n| local(*n) == "axisValue").find(|n| text(*n, "axisType").as_deref() == Some("TIMELAPSE"))?;
            num(v, "position")
        };
        let time_increment_s = (size_t > 1)
            .then(|| match (frame_time(0), frame_time(ls.len() * size_z)) {
                (Some(a), Some(b)) if b > a => Some((b - a) / 1e3),
                _ => axis("TIMELAPSE").and_then(|a| num(a, "step")).filter(|s| *s > 0.0),
            })
            .flatten();

        // Channel descriptions.
        let lambda = axis("LAMBDA").and_then(|a| Some((num(a, "startPosition")?, num(a, "step")?)));
        let lasers: HashMap<String, f64> = root
            .descendants()
            .filter(|n| local(*n) == "imagingMainLaser" && n.attribute("enable") == Some("true"))
            .filter_map(|n| Some((n.attribute("id")?.to_string(), num(n, "wavelength")?)))
            .collect();
        let lsm_channels: Vec<Document> = docs.iter().filter(|d| head(d).contains("<lsmimage:lsmChannel")).filter_map(|d| Document::parse(d).ok()).collect();
        let mut channels = Vec::new();
        for id in &channel_ids {
            let desc = described.iter().find(|(_, d, _)| d == id).map(|(_, _, n)| *n);
            let name = desc.and_then(|n| text(n, "name"));
            let ex: BTreeSet<u64> = desc
                .into_iter()
                .flat_map(|n| n.descendants())
                .filter(|n| local(*n) == "laserDataId")
                .filter_map(|n| lasers.get(n.text()?.trim()).map(|w| w.to_bits()))
                .collect();
            let excitation = (ex.len() == 1).then(|| f64::from_bits(*ex.first().expect("one")));
            let settings = lsm_channels.iter().map(|d| d.root_element()).find(|r| r.attribute("id") == Some(id));
            let dye = settings.and_then(|r| r.descendants().find(|n| local(*n) == "dyeName")).and_then(|n| n.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
            let dye_emission = settings.and_then(|r| r.descendants().find(|n| local(*n) == "emissionWavelength")).and_then(|n| n.text()?.trim().parse::<f64>().ok()).filter(|v| *v > 0.0);
            for &l in &ls {
                let band = (ls.len() > 1).then(|| lambda.map(|(start, step)| start + (l.max(1) - 1) as f64 * step)).flatten();
                channels.push(ChannelMeta {
                    name: match (&name, band) {
                        (Some(n), Some(b)) => Some(format!("{n} {b} nm")),
                        (n, _) => n.clone(),
                    },
                    fluor: dye.clone(),
                    excitation_wavelength_nm: excitation,
                    emission_wavelength_nm: band.or(dye_emission),
                    ..Default::default()
                });
            }
        }
        let objective = root.descendants().find(|n| local(*n) == "objectiveLens").map(|o| ObjectiveMeta {
            model: text(o, "name"),
            magnification: num(o, "magnification").filter(|v| *v > 0.0),
            numerical_aperture: num(o, "naValue").filter(|v| *v > 0.0),
            immersion: text(o, "immersion"),
            ..Default::default()
        });
        let date = root.descendants().find(|n| local(*n) == "creationDateTime").and_then(|n| n.text()).map(|t| t.trim().to_string());
        let meta = SeriesMeta {
            acquisition_date: date,
            size_c: size_c as u32,
            size_z: size_z as u32,
            size_t: size_t as u32,
            pixel_type,
            significant_bits: bits,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: size("x"), y_nm: size("y"), z_nm },
            time_increment_s,
            channels,
            objective,
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        for (i, d) in docs.iter().enumerate() {
            let root = head(d).split('<').nth(2).and_then(|r| r.split_whitespace().next()).unwrap_or("document");
            original.documents.push((format!("{i}:{root}"), d.clone()));
        }
        Ok(Oir { parts, planes, bytes: depth, metas: [meta], original })
    }
}

impl Backend for Oir {
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
        let m = &self.metas[0];
        let chunks = &self.planes[(plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize];
        let px = self.bytes as u64;
        let row_bytes = m.levels[0].size_x * px;
        let w = region.width as usize;
        scratch::with(w * self.bytes, |row| {
            for r in 0..region.height as usize {
                let start = (region.y + r as u64) * row_bytes + region.x * px;
                let end = start + region.width * px;
                row.fill(0);
                // Chunks overlapping the row; uncovered bytes stay zero.
                let first = chunks.partition_point(|c| c.plane_offset + c.len <= start);
                for c in chunks[first..].iter().take_while(|c| c.plane_offset < end) {
                    let (a, b) = (start.max(c.plane_offset), end.min(c.plane_offset + c.len));
                    if a < b {
                        let dst = &mut row[(a - start) as usize..(b - start) as usize];
                        read_exact_at(&self.parts[c.part as usize], dst, c.at + (a - c.plane_offset))?;
                    }
                }
                sink.write_with(r, 0, w, |d| {
                    d.copy_from_slice(row);
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
    fn parses_chunk_names() {
        let id = "93e4632f-0342-4d98-bdc1-4ce305b92525";
        assert_eq!(chunk_name(&format!("l005z004t003_0_1_{id}_44")), Some(((3, 5, 4), id.to_string())));
        assert_eq!(chunk_name(&format!("z067_0_1_{id}_3")), Some(((0, 0, 67), id.to_string())));
        assert_eq!(chunk_name(&format!("REF_LSM0_{id}_0")), None);
        assert_eq!(chunk_name(&format!("0_1_{id}_0")), Some(((0, 0, 0), id.to_string())));
        assert_eq!(chunk_name("garbage"), None);
    }
}

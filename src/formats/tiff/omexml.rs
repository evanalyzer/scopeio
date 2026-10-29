//! # omexml
//!
//! OME-XML files (`.ome`, `.ome.xml`) holding their pixels inline.
//!
//! Each `Pixels` element carries one `BinData` element per plane, in the
//! order of `DimensionOrder`: base64 text, `Compression` `none` or `zlib`
//! (`bzip2` is not supported) and `BigEndian` for the byte order. Type
//! `bit` packs eight samples per byte, most significant bit first, and is
//! returned as 0 / 1 bytes. Images without `BinData` (`MetadataOnly`) and
//! empty `BinData` elements read as zeros. The metadata is the OME-TIFF reader's (see [`super::ome`]).
//! Planes are decoded on open and held in memory, as the whole document is
//! read anyway.
//!
//! Provenance: OME-XML schema 2016-06 (`BinData`, `Compression`,
//! `BigEndian`) and the OME data model documentation (`doc/ome-xml/`); the
//! OME-XML sample files on the OME sample server; checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::ome::{self, children};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, Plane, PlaneMeta, PlateMeta, Region, Result, SeriesMeta};

pub(in crate::formats) static FORMAT: FormatInfo = FormatInfo::new(FormatId::OmeXml, "OME-XML", "1").extensions(&["ome", "ome.xml"]);

/// Largest OME-XML document read.
const MAX_XML: u64 = 2 << 30;

/// Returns `true` for an OME-XML document with inline pixels or none (no
/// `TiffData`, which makes it an OME-TIFF companion).
pub(in crate::formats) fn matches(path: &Path, head: &[u8]) -> bool {
    if !ome::is_ome_xml(head) {
        return false;
    }
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if meta.len() > MAX_XML {
        return false;
    }
    std::fs::read(path).is_ok_and(|b| !contains(&b, b"TiffData") && !contains(&b, b"BinaryOnly"))
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Decodes base64, ignoring whitespace; `None` for other characters.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for b in text.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// One image: its planes by `c + size_c * (z + size_z * t)`, native byte order.
#[derive(Debug)]
struct Image {
    planes: Vec<Option<Vec<u8>>>,
    plane_meta: Vec<PlaneMeta>,
}

/// OME-XML document with inline pixels.
#[derive(Debug)]
pub(in crate::formats) struct OmeXml {
    images: Vec<Image>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    xml: String,
}

impl OmeXml {
    /// Opens an OME-XML file and decodes its planes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or planes of the wrong
    /// size and [`Error::UnsupportedVariant`] for bzip2-compressed planes.
    pub fn open(path: &Path) -> Result<Self> {
        if std::fs::metadata(path)?.len() > MAX_XML {
            return Err(Error::Corrupt("OME-XML file too large".into()));
        }
        let raw = std::fs::read(path)?;
        let xml = String::from_utf8_lossy(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw)).into_owned();
        let doc = ome::parse(&xml)?;
        let root = doc.root_element();
        let (objectives, rois) = ome::document_maps(root);
        let mut images = Vec::new();
        let mut metas = Vec::new();
        let mut image_ids = std::collections::HashMap::new();
        for (i, image) in children(root, "Image").enumerate() {
            if let Some(id) = image.attribute("ID") {
                image_ids.insert(id.to_string(), i);
            }
            let pixels = ome::pixels_of(image)?;
            let ((sx, sy, sz, size_c, st), _, samples) = ome::sizes_of(pixels);
            let pixel_type = ome::pixel_type_of(pixels)?;
            let bit = pixels.attribute("Type").or(pixels.attribute("PixelType")) == Some("bit");
            let total = (size_c as usize).checked_mul(sz as usize).and_then(|v| v.checked_mul(st as usize)).filter(|&v| v <= 1 << 24);
            let total = total.ok_or_else(|| Error::Corrupt("OME-XML with too many planes".into()))?;
            let plane_len = (sx as u64 * sy as u64 * samples as u64 * pixel_type.bytes() as u64) as usize;
            let order = pixels.attribute("DimensionOrder").unwrap_or("XYZCT");
            let mut planes = vec![None; total];
            for (k, bin) in children(pixels, "BinData").enumerate().take(total) {
                let data = base64(bin.text().unwrap_or_default()).ok_or_else(|| Error::Corrupt("OME-XML BinData is not base64".into()))?;
                // An empty BinData is a placeholder: the plane reads as zeros.
                if data.is_empty() {
                    continue;
                }
                let mut data = match bin.attribute("Compression").unwrap_or("none") {
                    "none" => data,
                    "zlib" => {
                        // Planes of packed bits are shorter than `plane_len`.
                        let mut out = vec![0u8; plane_len];
                        let n = crate::codecs::deflate(&data, &mut out, true)?;
                        out.truncate(n);
                        out
                    }
                    c => return Err(Error::UnsupportedVariant(format!("OME-XML BinData compression {c}"))),
                };
                if bit {
                    let n = sx as usize * sy as usize;
                    data = (0..n).map(|i| data.get(i / 8).map_or(0, |b| (b >> (7 - i % 8)) & 1)).collect();
                }
                if data.len() < plane_len {
                    return Err(Error::Corrupt(format!("OME-XML plane {k} of {} bytes, {plane_len} expected", data.len())));
                }
                data.truncate(plane_len);
                let size = pixel_type.bytes();
                let big = bin.attribute("BigEndian").is_some_and(|v| v.eq_ignore_ascii_case("true"));
                if size > 1 && big == cfg!(target_endian = "little") {
                    data.chunks_exact_mut(size).for_each(<[u8]>::reverse);
                }
                let (z, c, t) = ome::unrasterize(order, k, (sz as usize, size_c as usize, st as usize));
                planes[c + size_c as usize * (z + sz as usize * t)] = Some(data);
            }
            let levels = vec![LevelMeta { size_x: sx as u64, size_y: sy as u64, tile: None, downsample: 1.0 }];
            let (meta, plane_meta) = ome::describe(image, pixels, (size_c, sz, st), pixel_type, samples, levels, &objectives, &rois);
            metas.push(meta);
            images.push(Image { planes, plane_meta });
        }
        if images.is_empty() {
            return Err(Error::Corrupt("OME-XML without Image".into()));
        }
        let plates = children(root, "Plate").map(|p| ome::plate(p, &image_ids)).collect();
        Ok(OmeXml { images, metas, plates, xml })
    }
}

impl Backend for OmeXml {
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
        Ok(self.images[series].plane_meta.get(i).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(OriginalMeta { pairs: Vec::new(), documents: vec![("OME-XML".into(), self.xml.clone())] })
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let pixel = m.pixel_type.bytes() * m.samples_per_pixel as usize;
        let width = m.levels[0].size_x as usize;
        let w = region.width as usize;
        let data = self.images[series].planes[i].as_deref();
        for r in 0..region.height as usize {
            let at = ((region.y as usize + r) * width + region.x as usize) * pixel;
            sink.write_with(r, 0, w, |d| {
                match data {
                    Some(p) => d.copy_from_slice(&p[at..at + w * pixel]),
                    // Planes without BinData read as zeros.
                    None => d.fill(0),
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(base64("AP+r\n zg==").unwrap(), [0x00, 0xFF, 0xAB, 0xCE]);
        assert_eq!(base64("TWFu").unwrap(), b"Man");
        assert!(base64("A*").is_none());
    }
}

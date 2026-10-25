//! # plain
//!
//! Plain image files: PNG, JPEG (JFIF) and JPEG 2000 (JP2 file or raw
//! codestream). One series and one plane.
//!
//! PNG palette images are expanded to RGB (transparency to alpha); gray
//! images below 8 bits return their values unscaled (0..=1 for 1 bit), as
//! 8-bit samples; a gray image with alpha is two gray channels.
//! Non-interlaced PNG rows are decoded only up to the last row a read needs;
//! interlaced PNG, JPEG and JPEG 2000 images are decoded as a whole for
//! every read. The resolution levels of a JPEG 2000 codestream are the
//! pyramid levels, each decoded at its own size. Pixel sizes come from the PNG `pHYs` chunk,
//! the JFIF density and the JP2 capture (or display) resolution box. Only
//! the first frame of an animated PNG is read.
//!
//! Provenance: PNG specification (W3C, 3rd edition); JFIF 1.02 (ITU-T
//! T.871); JPEG 2000 Part 1 (ITU-T T.800, Annex I: JP2 boxes); decoding via
//! the `png` crate (MIT OR Apache-2.0), `zune-jpeg` and `hayro-jpeg2000`.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::{Path, PathBuf};

use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region,
    Result, SeriesMeta, scratch,
};

#[cfg(feature = "png")]
pub(super) static PNG_FORMAT: FormatInfo = FormatInfo::new(FormatId::Png, "PNG", "1").extensions(&["png"]);
#[cfg(feature = "jpg")]
pub(super) static JPEG_FORMAT: FormatInfo = FormatInfo::new(FormatId::Jpeg, "JPEG", "1").extensions(&["jpg", "jpeg"]);
#[cfg(feature = "jp2")]
pub(super) static JP2_FORMAT: FormatInfo = FormatInfo::new(FormatId::Jpeg2000, "JPEG 2000", "1").extensions(&["jp2", "j2k", "j2c"]);

#[cfg(feature = "png")]
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
#[cfg(feature = "jp2")]
const JP2_MAGIC: &[u8] = b"\0\0\0\x0cjP  \r\n\x87\n";
#[cfg(feature = "jp2")]
const J2K_MAGIC: &[u8] = b"\xff\x4f\xff\x51";
/// Largest image decoded as a whole, in bytes.
const MAX_DECODED: usize = 1 << 30;

/// The plain image format of `head`, if any.
pub(super) fn probe(head: &[u8]) -> Option<FormatId> {
    #[cfg(feature = "png")]
    if head.starts_with(PNG_MAGIC) {
        return Some(FormatId::Png);
    }
    #[cfg(feature = "jpg")]
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(FormatId::Jpeg);
    }
    #[cfg(feature = "jp2")]
    if head.starts_with(JP2_MAGIC) || head.starts_with(J2K_MAGIC) {
        return Some(FormatId::Jpeg2000);
    }
    let _ = head;
    None
}

/// How the pixels are decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Interlaced images are decoded as a whole; `packed` is the bit depth
    /// of gray images below 8 bits, returned unscaled (0 if expanded).
    #[cfg(feature = "png")]
    Png { interlaced: bool, packed: u8 },
    #[cfg(feature = "jpg")]
    Jpeg,
    /// Resolution levels are the pyramid levels.
    #[cfg(feature = "jp2")]
    Jp2,
}

/// A PNG, JPEG or JPEG 2000 file.
#[derive(Debug)]
pub(super) struct Plain {
    path: PathBuf,
    format: &'static FormatInfo,
    kind: Kind,
    /// Decoded samples per pixel (after PNG expansion).
    samples: usize,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
}

/// Series metadata of a `width` x `height` image with `samples` decoded
/// samples per pixel: 3 or 4 are colour, otherwise one channel each.
fn series(width: usize, height: usize, samples: usize, pixel_type: PixelType, nm: (Option<f64>, Option<f64>)) -> SeriesMeta {
    let colour = samples >= 3;
    let size_c = if colour { 1 } else { samples as u32 };
    SeriesMeta {
        size_c,
        size_z: 1,
        size_t: 1,
        pixel_type,
        samples_per_pixel: if colour { samples as u8 } else { 1 },
        levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
        physical_size: PhysicalSize { x_nm: nm.0, y_nm: nm.1, z_nm: None },
        channels: vec![ChannelMeta::default(); size_c as usize],
        ..Default::default()
    }
}

/// Nanometres per pixel from pixels per metre.
fn nm_per_pixel(per_metre: f64) -> Option<f64> {
    (per_metre.is_finite() && per_metre > 0.0).then(|| 1e9 / per_metre)
}

#[cfg(feature = "jpg")]
/// Pixel size from a JFIF APP0 segment: units 1 (per inch) or 2 (per cm).
fn jfif_size(data: &[u8]) -> (Option<f64>, Option<f64>) {
    let mut pos = 2;
    while pos + 4 <= data.len() && data[pos] == 0xFF {
        let marker = data[pos + 1];
        let len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        let seg = data.get(pos + 4..pos + 2 + len).unwrap_or_default();
        if marker == 0xE0 && seg.len() >= 12 && seg.starts_with(b"JFIF\0") {
            let (x, y) = (u16::from_be_bytes([seg[8], seg[9]]) as f64, u16::from_be_bytes([seg[10], seg[11]]) as f64);
            let metre = match seg[7] {
                1 => 1.0 / 0.0254,
                2 => 100.0,
                _ => return (None, None),
            };
            return (nm_per_pixel(x * metre), nm_per_pixel(y * metre));
        }
        if marker == 0xDA || len < 2 {
            break;
        }
        pos += 2 + len;
    }
    (None, None)
}

#[cfg(feature = "jp2")]
/// Boxes `(type, payload)` in `data`.
fn boxes(data: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let len = u32::from_be_bytes(data[pos..pos + 4].try_into().expect("4 bytes")) as usize;
        let typ = &data[pos + 4..pos + 8];
        let (start, end) = match len {
            0 => (pos + 8, data.len()),
            1 if pos + 16 <= data.len() => {
                let l = u64::from_be_bytes(data[pos + 8..pos + 16].try_into().expect("8 bytes"));
                (pos + 16, pos.saturating_add(usize::try_from(l).unwrap_or(usize::MAX)))
            }
            l if l >= 8 => (pos + 8, pos.saturating_add(l)),
            _ => break,
        };
        if end > data.len() || start > end {
            break;
        }
        out.push((typ, &data[start..end]));
        pos = end;
    }
    out
}

#[cfg(feature = "jp2")]
/// Pixel size from the JP2 `res ` box: capture resolution, else display
/// resolution, in grid points per metre.
fn jp2_size(data: &[u8]) -> (Option<f64>, Option<f64>) {
    let Some((_, header)) = boxes(data).into_iter().find(|(t, _)| *t == b"jp2h") else { return (None, None) };
    let Some((_, res)) = boxes(header).into_iter().find(|(t, _)| *t == b"res ") else { return (None, None) };
    let res = boxes(res);
    let Some((_, r)) = res.iter().find(|(t, _)| *t == b"resc").or_else(|| res.iter().find(|(t, _)| *t == b"resd")) else {
        return (None, None);
    };
    if r.len() < 10 {
        return (None, None);
    }
    let u = |i: usize| u16::from_be_bytes([r[i], r[i + 1]]) as f64;
    let per_metre = |n: f64, d: f64, e: i8| if d > 0.0 { n / d * 10f64.powi(e as i32) } else { 0.0 };
    let vertical = per_metre(u(0), u(2), r[8] as i8);
    let horizontal = per_metre(u(4), u(6), r[9] as i8);
    (nm_per_pixel(horizontal), nm_per_pixel(vertical))
}

impl Plain {
    /// Opens a plain image file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers and
    /// [`Error::UnsupportedVariant`] for sample layouts not handled.
    pub fn open(path: &Path, id: FormatId) -> Result<Self> {
        match id {
            #[cfg(feature = "png")]
            FormatId::Png => Self::open_png(path),
            #[cfg(feature = "jpg")]
            FormatId::Jpeg => Self::open_jpeg(path),
            #[cfg(feature = "jp2")]
            FormatId::Jpeg2000 => Self::open_jp2(path),
            _ => Err(Error::UnsupportedFormat(path.display().to_string())),
        }
    }

    #[cfg(feature = "png")]
    fn png_reader(path: &Path, expand: bool) -> Result<png::Reader<std::io::BufReader<std::fs::File>>> {
        let mut dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?));
        dec.set_transformations(if expand { png::Transformations::EXPAND } else { png::Transformations::IDENTITY });
        dec.read_info().map_err(|e| Error::Corrupt(format!("PNG: {e}")))
    }

    #[cfg(feature = "png")]
    fn open_png(path: &Path) -> Result<Self> {
        let reader = Self::png_reader(path, true)?;
        let info = reader.info();
        let (color, depth) = reader.output_color_type();
        let samples = color.samples();
        // Low-bit gray (without transparency) keeps its values.
        let bits = info.bit_depth as u8;
        let packed = if info.color_type == png::ColorType::Grayscale && bits < 8 && info.trns.is_none() { bits } else { 0 };
        let pixel_type = if depth == png::BitDepth::Sixteen { PixelType::U16 } else { PixelType::U8 };
        let nm = match info.pixel_dims {
            Some(d) if d.unit == png::Unit::Meter => (nm_per_pixel(d.xppu as f64), nm_per_pixel(d.yppu as f64)),
            _ => (None, None),
        };
        let mut original = OriginalMeta::default();
        original.pairs.push(("png.ColorType".into(), format!("{:?}", info.color_type)));
        original.pairs.push(("png.BitDepth".into(), (info.bit_depth as u8).to_string()));
        original.pairs.push(("png.Interlaced".into(), info.interlaced.to_string()));
        for t in &info.uncompressed_latin1_text {
            original.pairs.push((format!("png.{}", t.keyword), t.text.clone()));
        }
        for t in &info.utf8_text {
            if let Ok(text) = t.get_text() {
                original.pairs.push((format!("png.{}", t.keyword), text));
            }
        }
        let meta = series(info.width as usize, info.height as usize, samples, pixel_type, nm);
        let kind = Kind::Png { interlaced: info.interlaced, packed };
        Ok(Plain { path: path.to_path_buf(), format: &PNG_FORMAT, kind, samples, metas: [meta], original })
    }

    #[cfg(feature = "jpg")]
    fn open_jpeg(path: &Path) -> Result<Self> {
        use zune_jpeg::JpegDecoder;
        use zune_jpeg::zune_core::bytestream::ZCursor;
        let data = std::fs::read(path)?;
        let mut dec = JpegDecoder::new(ZCursor::new(&data[..]));
        dec.decode_headers().map_err(|e| Error::Corrupt(format!("JPEG: {e}")))?;
        let info = dec.info().ok_or_else(|| Error::Corrupt("JPEG without frame header".into()))?;
        let samples = info.components as usize;
        if samples != 1 && samples != 3 {
            return Err(Error::UnsupportedVariant(format!("JPEG with {samples} components")));
        }
        let meta = series(info.width as usize, info.height as usize, samples, PixelType::U8, jfif_size(&data));
        let original = OriginalMeta { pairs: vec![("jpeg.Components".into(), samples.to_string())], ..Default::default() };
        Ok(Plain { path: path.to_path_buf(), format: &JPEG_FORMAT, kind: Kind::Jpeg, samples, metas: [meta], original })
    }

    #[cfg(feature = "jp2")]
    fn open_jp2(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        let (info, depth) = crate::codecs::jpeg2000::info(&data)?;
        if depth > 16 || info.components == 0 {
            return Err(Error::UnsupportedVariant(format!("JPEG 2000 with {} components of {depth} bits", info.components)));
        }
        let pixel_type = if depth > 8 { PixelType::U16 } else { PixelType::U8 };
        let nm = if data.starts_with(JP2_MAGIC) { jp2_size(&data) } else { (None, None) };
        let mut meta = series(info.width, info.height, info.components, pixel_type, nm);
        // Resolution levels: halve until the codestream has no lower one.
        for k in 1..32u8 {
            let (lv, _) = crate::codecs::jpeg2000::info_at(&data, k)?;
            let prev = meta.levels.last().expect("level 0");
            if (lv.width as u64, lv.height as u64) == (prev.size_x, prev.size_y) {
                break;
            }
            meta.levels.push(LevelMeta { size_x: lv.width as u64, size_y: lv.height as u64, tile: None, downsample: (1u64 << k) as f64 });
        }
        let original = OriginalMeta {
            pairs: vec![("jpeg2000.Components".into(), info.components.to_string()), ("jpeg2000.BitDepth".into(), depth.to_string())],
            ..Default::default()
        };
        Ok(Plain { path: path.to_path_buf(), format: &JP2_FORMAT, kind: Kind::Jp2, samples: info.components, metas: [meta], original })
    }

    /// Decodes the whole image into `out` (`samples` native samples per
    /// pixel), for formats without partial decoding.
    fn decode_all(&self, level: usize, out: &mut [u8]) -> Result<()> {
        match self.kind {
            #[cfg(feature = "png")]
            Kind::Png { packed, .. } => {
                let mut reader = Self::png_reader(&self.path, packed == 0)?;
                if packed > 0 {
                    // Packed rows of the whole image, then one byte per pixel.
                    let width = self.metas[0].levels[0].size_x as usize;
                    let line = (width * packed as usize).div_ceil(8);
                    let size = reader.output_buffer_size().ok_or_else(|| Error::Corrupt("PNG too large".into()))?;
                    return scratch::with(size, |buf| {
                        reader.next_frame(buf).map_err(|e| Error::Corrupt(format!("PNG: {e}")))?;
                        for (src, dst) in buf.chunks_exact(line).zip(out.chunks_exact_mut(width)) {
                            unpack(src, packed, 0, dst);
                        }
                        Ok(())
                    });
                }
                reader.next_frame(out).map_err(|e| Error::Corrupt(format!("PNG: {e}")))?;
                if self.metas[0].pixel_type == PixelType::U16 {
                    big_endian_to_native(out);
                }
                Ok(())
            }
            #[cfg(feature = "jpg")]
            Kind::Jpeg => {
                let data = std::fs::read(&self.path)?;
                crate::codecs::jpeg::decode(&data, None, crate::codecs::jpeg::Color::YcbcrToRgb, out).map(|_| ())
            }
            #[cfg(feature = "jp2")]
            Kind::Jp2 => {
                let data = std::fs::read(&self.path)?;
                crate::codecs::jpeg2000::decode_at(&data, self.metas[0].pixel_type, out, level as u8).map(|_| ())
            }
        }
    }

    /// Streams the rows of a non-interlaced PNG up to the region's end.
    #[cfg(feature = "png")]
    fn read_png_rows(&self, samples: usize, channel: Option<usize>, packed: u8, region: Region, sink: &mut Sink) -> Result<()> {
        let mut reader = Self::png_reader(&self.path, packed == 0)?;
        let mut unpacked = vec![0u8; if packed > 0 { region.width as usize } else { 0 }];
        let bytes = self.metas[0].pixel_type.bytes();
        let px = samples * bytes;
        let (x0, y0) = (region.x as usize, region.y as usize);
        let (w, h) = (region.width as usize, region.height as usize);
        for y in 0..y0 + h {
            let row = reader.next_row().map_err(|e| Error::Corrupt(format!("PNG: {e}")))?;
            let Some(row) = row else { return Err(Error::Corrupt("PNG ends early".into())) };
            if y < y0 {
                continue;
            }
            if packed > 0 {
                if row.data().len() * 8 < (x0 + w) * packed as usize {
                    return Err(Error::Corrupt("PNG row too short".into()));
                }
                unpack(row.data(), packed, x0, &mut unpacked);
                put_row(sink, y - y0, &unpacked, 1, 1, None, false)?;
                continue;
            }
            let src = row.data().get(x0 * px..(x0 + w) * px).ok_or_else(|| Error::Corrupt("PNG row too short".into()))?;
            put_row(sink, y - y0, src, samples, bytes, channel, true)?;
        }
        Ok(())
    }
}

/// Unpacks `dst.len()` samples of `bits` bits (MSB first) starting at
/// sample `x0` of a packed row.
#[cfg(feature = "png")]
fn unpack(src: &[u8], bits: u8, x0: usize, dst: &mut [u8]) {
    let bits = bits as usize;
    let mask = (1u8 << bits) - 1;
    for (i, d) in dst.iter_mut().enumerate() {
        let at = (x0 + i) * bits;
        *d = (src[at / 8] >> (8 - bits - at % 8)) & mask;
    }
}

/// Swaps 16-bit big-endian samples to native order.
fn big_endian_to_native(b: &mut [u8]) {
    for s in b.chunks_exact_mut(2) {
        let v = u16::from_be_bytes([s[0], s[1]]);
        s.copy_from_slice(&v.to_ne_bytes());
    }
}

/// Writes one region row: all samples (colour) or sample `channel`.
fn put_row(sink: &mut Sink, y: usize, src: &[u8], samples: usize, bytes: usize, channel: Option<usize>, big_endian: bool) -> Result<()> {
    let px = samples * bytes;
    let w = src.len() / px;
    sink.write_with(y, 0, w, |dst| {
        match channel {
            None => dst.copy_from_slice(src),
            Some(c) => {
                for (d, s) in dst.chunks_exact_mut(bytes).zip(src.chunks_exact(px)) {
                    d.copy_from_slice(&s[c * bytes..(c + 1) * bytes]);
                }
            }
        }
        if big_endian && bytes == 2 {
            big_endian_to_native(dst);
        }
        Ok(())
    })
}

impl Backend for Plain {
    fn format(&self) -> &'static FormatInfo {
        self.format
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[0];
        let samples = self.samples;
        // Colour images return all samples, others one channel.
        let channel = (m.samples_per_pixel == 1 && samples > 1).then_some(plane.c as usize);
        #[cfg(feature = "png")]
        if let Kind::Png { interlaced: false, packed } = self.kind {
            return self.read_png_rows(samples, channel, packed, region, sink);
        }
        let bytes = m.pixel_type.bytes();
        let (width, height) = (m.levels[level].size_x as usize, m.levels[level].size_y as usize);
        let len = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(samples * bytes))
            .filter(|&n| n <= MAX_DECODED)
            .ok_or_else(|| Error::UnsupportedVariant(format!("{} image of {width} x {height} too large to decode at once", self.format.name)))?;
        scratch::with(len, |buf| {
            self.decode_all(level, buf)?;
            let px = samples * bytes;
            let (x0, y0) = (region.x as usize, region.y as usize);
            for r in 0..region.height as usize {
                let start = ((y0 + r) * width + x0) * px;
                put_row(sink, r, &buf[start..start + region.width as usize * px], samples, bytes, channel, false)?;
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
    fn detects_signatures() {
        #[cfg(feature = "png")]
        assert_eq!(probe(PNG_MAGIC), Some(FormatId::Png));
        #[cfg(feature = "jpg")]
        assert_eq!(probe(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(FormatId::Jpeg));
        #[cfg(feature = "jp2")]
        assert_eq!(probe(J2K_MAGIC), Some(FormatId::Jpeg2000));
        assert_eq!(probe(b"II*\0"), None);
    }

    #[test]
    fn reads_resolution_boxes() {
        // JFIF at 254 dpi: 100 um per pixel.
        let jfif = [0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 2, 1, 0, 254, 0, 254, 0, 0];
        let (x, y) = jfif_size(&jfif);
        assert!((x.unwrap() - 100_000.0).abs() < 1e-6 && (y.unwrap() - 100_000.0).abs() < 1e-6);
        // JP2 capture resolution 4000 grid points per metre: 250 um.
        let resc = [0, 4, 0, 1, 0, 4, 0, 1, 3, 3];
        let mut res = (8u32 + 10).to_be_bytes().to_vec();
        res.extend_from_slice(b"resc");
        res.extend_from_slice(&resc);
        let mut res_box = ((8 + res.len()) as u32).to_be_bytes().to_vec();
        res_box.extend_from_slice(b"res ");
        res_box.extend_from_slice(&res);
        let mut file = JP2_MAGIC.to_vec();
        file.extend_from_slice(&((8 + res_box.len()) as u32).to_be_bytes());
        file.extend_from_slice(b"jp2h");
        file.extend_from_slice(&res_box);
        assert_eq!(jp2_size(&file), (Some(250_000.0), Some(250_000.0)));
    }
}

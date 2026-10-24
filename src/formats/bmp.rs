//! # bmp
//!
//! Windows bitmap (BMP/DIB) reader.
//!
//! Supports the core, info and V2-V5 headers with 1, 2, 4, 8, 16, 24 and
//! 32 bits per pixel, uncompressed, bit-field and RLE4/RLE8 compressed data.
//! Palette images with a gray palette are returned as gray, other palette
//! images are expanded to RGB. Packed 16- and
//! 32-bit pixels are scaled to 8 bits per sample.
//!
//! Provenance: Microsoft "Bitmap Storage", "Bitmap Compression" and the
//! `BITMAP*HEADER` structure pages (see `doc/bmp/`); own analysis of sample
//! files. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
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
use crate::scratch;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region,
    Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Bmp, "Windows Bitmap", "1");

/// Returns `true` if `header` starts like a Windows bitmap file.
pub(super) fn matches(header: &[u8]) -> bool {
    header.len() >= 18 && &header[..2] == b"BM" && matches!(u32_at(header, 14), 12 | 40 | 52 | 56 | 64 | 108 | 124)
}

const FILE_HEADER_SIZE: u64 = 14;
const BI_RGB: u32 = 0;
const BI_RLE8: u32 = 1;
const BI_RLE4: u32 = 2;
const BI_BITFIELDS: u32 = 3;
const BI_ALPHABITFIELDS: u32 = 6;
/// Upper bound of the scratch buffer used per read call.
const CHUNK_BYTES: usize = 1 << 20;

/// Reader for Windows bitmap files.
#[derive(Debug)]
pub(super) struct Bmp {
    file: File,
    series: [SeriesMeta; 1],
    pixels: Pixels,
    data_offset: u64,
    data_len: u64,
    stride: usize,
    top_down: bool,
    /// RLE-compressed data, decoded on every read.
    rle: bool,
}

/// Pixel encoding of the bitmap data.
#[derive(Debug)]
enum Pixels {
    /// Palette indices of `bits` bits.
    Indexed { bits: u8, lut: Lut },
    /// 3 bytes per pixel, blue first.
    Bgr24,
    /// 2 or 4 bytes per pixel, little-endian, split by bit masks.
    Packed { bytes: u8, masks: Vec<Mask> },
}

/// Palette padded to 256 entries; missing entries are black.
#[derive(Debug)]
struct Lut {
    rgb: Box<[[u8; 3]; 256]>,
    /// All entries have equal red, green and blue.
    gray: bool,
    /// Gray palette mapping each index to itself.
    identity: bool,
}

/// One sample within a packed pixel.
#[derive(Debug, Clone, Copy)]
struct Mask {
    shift: u32,
    max: u32,
}

impl Mask {
    fn new(mask: u32) -> Result<Self> {
        if mask == 0 {
            return Err(Error::Corrupt("empty colour mask".into()));
        }
        let shift = mask.trailing_zeros();
        let max = mask >> shift;
        if max & (max + 1) != 0 {
            return Err(Error::Corrupt(format!("colour mask {mask:#x} is not contiguous")));
        }
        if max > 0xFF {
            return Err(Error::UnsupportedVariant(format!("colour mask {mask:#x} wider than 8 bits")));
        }
        Ok(Self { shift, max })
    }

    fn sample(self, pixel: u32) -> u8 {
        let v = (pixel >> self.shift) & self.max;
        if self.max == 0xFF { v as u8 } else { ((v * 255 + self.max / 2) / self.max) as u8 }
    }
}

/// Fields of the bitmap information header.
struct Header {
    size: u32,
    width: i32,
    height: i32,
    bits: u16,
    compression: u32,
    image_size: u32,
    x_ppm: i32,
    y_ppm: i32,
    colors_used: u32,
    colors_important: u32,
    masks: [u32; 4],
}

impl Bmp {
    /// Opens a bitmap file and parses its headers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read, [`Error::UnsupportedVariant`]
    /// for JPEG/PNG payloads, OS/2 Huffman data and masks wider than 8 bits,
    /// and [`Error::Corrupt`] for invalid headers or truncated data.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let file_len = file.metadata()?.len();
        let mut head = [0u8; 14 + 124 + 16];
        let head_len = head.len().min(file_len as usize);
        read_exact_at(&file, &mut head[..head_len], 0)?;
        if !matches(&head[..head_len]) {
            return Err(Error::Corrupt("not a BMP file".into()));
        }
        let data_offset = u32_at(&head, 10) as u64;
        let h = parse_header(&head[14..head_len])?;

        if h.width <= 0 || h.height == 0 || h.height == i32::MIN {
            return Err(Error::Corrupt(format!("invalid size {}x{}", h.width, h.height)));
        }
        let width = h.width as u32;
        let height = h.height.unsigned_abs();
        let top_down = h.height < 0;
        let bits = h.bits;

        let rle = matches!(h.compression, BI_RLE8 | BI_RLE4);
        if rle && top_down {
            return Err(Error::Corrupt("compressed bitmaps must be bottom-up".into()));
        }
        // Colour masks follow a 40-byte header; larger headers contain them.
        let mut masks = h.masks;
        let mut table_offset = FILE_HEADER_SIZE + h.size as u64;
        if h.size == 40 && matches!(h.compression, BI_BITFIELDS | BI_ALPHABITFIELDS) {
            let n = if h.compression == BI_ALPHABITFIELDS { 4 } else { 3 };
            let mut buf = [0u8; 16];
            read_exact_at(&file, &mut buf[..n * 4], table_offset)?;
            for (i, m) in masks.iter_mut().enumerate().take(n) {
                *m = u32_at(&buf, i * 4);
            }
            table_offset += n as u64 * 4;
        }

        let pixels = match (bits, h.compression) {
            // OS/2 2.x headers reuse codes 3 and 4 for Huffman and RLE24.
            (_, c @ (BI_BITFIELDS | 4)) if h.size == 64 => {
                return Err(Error::UnsupportedVariant(format!("OS/2 bitmap compression {c}")));
            }
            (1 | 2 | 4 | 8, BI_RGB) | (8, BI_RLE8) | (4, BI_RLE4) => {
                let entry = if h.size == 12 { 3 } else { 4 };
                // Some writers leave the colour count at 0 with a shorter table;
                // never read the table into the pixel data.
                let room = data_offset.saturating_sub(table_offset) / entry as u64;
                let lut = read_palette(&file, table_offset, entry, bits, h.colors_used, room)?;
                Pixels::Indexed { bits: bits as u8, lut }
            }
            (24, BI_RGB) => Pixels::Bgr24,
            (16 | 32, BI_RGB) => {
                let rgb = if bits == 16 { [0x7C00, 0x03E0, 0x001F] } else { [0xFF_0000, 0xFF00, 0xFF] };
                Pixels::Packed { bytes: (bits / 8) as u8, masks: packed_masks(&rgb, 0)? }
            }
            (16 | 32, BI_BITFIELDS | BI_ALPHABITFIELDS) => {
                let alpha = if h.size >= 56 || h.compression == BI_ALPHABITFIELDS { masks[3] } else { 0 };
                Pixels::Packed { bytes: (bits / 8) as u8, masks: packed_masks(&masks[..3], alpha)? }
            }
            (_, 4 | 5) => {
                return Err(Error::UnsupportedVariant("BMP with embedded JPEG or PNG data".into()));
            }
            (b, c) => {
                return Err(Error::Corrupt(format!("invalid bit count {b} for compression {c}")));
            }
        };

        let stride = (width as usize * bits as usize).div_ceil(32) * 4;
        let data_len = if rle {
            let available = file_len.saturating_sub(data_offset);
            if h.image_size > 0 { available.min(h.image_size as u64) } else { available }
        } else {
            let needed = stride as u64 * height as u64;
            if data_offset + needed > file_len {
                return Err(Error::Corrupt(format!(
                    "pixel data needs {needed} bytes at offset {data_offset}, file has {file_len}"
                )));
            }
            needed
        };

        let samples_per_pixel = match &pixels {
            Pixels::Indexed { lut, .. } if lut.gray => 1,
            Pixels::Indexed { .. } | Pixels::Bgr24 => 3,
            Pixels::Packed { masks, .. } => masks.len() as u8,
        };
        let nm_per_pixel = |ppm: i32| (ppm > 0).then(|| 1e9 / ppm as f64);
        let meta = SeriesMeta {
            size_c: 1,
            size_z: 1,
            size_t: 1,
            pixel_type: PixelType::U8,
            samples_per_pixel,
            levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize {
                x_nm: nm_per_pixel(h.x_ppm),
                y_nm: nm_per_pixel(h.y_ppm),
                z_nm: None,
            },
            channels: vec![ChannelMeta::default()],
            ..Default::default()
        };

        Ok(Self {
            file,
            series: [meta],
            pixels,
            data_offset,
            data_len,
            stride,
            top_down,
            rle,
        })
    }

    /// Converts one stored row into output samples.
    ///
    /// `src` holds the stored row from its first byte, `dst` the output row
    /// of `width` pixels starting at pixel `x`.
    fn convert_row(&self, src: &[u8], x: usize, dst: &mut [u8]) {
        match &self.pixels {
            Pixels::Indexed { bits: 8, lut } if lut.identity => {
                dst.copy_from_slice(&src[x..x + dst.len()]);
            }
            Pixels::Indexed { bits: 8, lut } if lut.gray => {
                for (out, &i) in dst.iter_mut().zip(&src[x..]) {
                    *out = lut.rgb[i as usize][0];
                }
            }
            Pixels::Indexed { bits: 8, lut } => {
                for (out, &i) in dst.chunks_exact_mut(3).zip(&src[x..]) {
                    out.copy_from_slice(&lut.rgb[i as usize]);
                }
            }
            Pixels::Indexed { bits, lut } => {
                let bits = *bits as usize;
                let mask = (1usize << bits) - 1;
                let samples = if lut.gray { 1 } else { 3 };
                for (i, out) in dst.chunks_exact_mut(samples).enumerate() {
                    let bit = (x + i) * bits;
                    let index = (src[bit / 8] as usize >> (8 - bits - bit % 8)) & mask;
                    out.copy_from_slice(&lut.rgb[index][..samples]);
                }
            }
            Pixels::Bgr24 => {
                let src = &src[x * 3..];
                for (out, bgr) in dst.chunks_exact_mut(3).zip(src.chunks_exact(3)) {
                    out.copy_from_slice(&[bgr[2], bgr[1], bgr[0]]);
                }
            }
            Pixels::Packed { bytes, masks } => {
                let n = *bytes as usize;
                let src = &src[x * n..];
                for (out, px) in dst.chunks_exact_mut(masks.len()).zip(src.chunks_exact(n)) {
                    let pixel = if n == 2 {
                        u16::from_le_bytes([px[0], px[1]]) as u32
                    } else {
                        u32::from_le_bytes([px[0], px[1], px[2], px[3]])
                    };
                    for (o, m) in out.iter_mut().zip(masks) {
                        *o = m.sample(pixel);
                    }
                }
            }
        }
    }

    /// Decodes the palette indices of an RLE file, top row first.
    fn rle_indices(&self) -> Result<Vec<u8>> {
        let Pixels::Indexed { bits, .. } = self.pixels else {
            return Err(Error::Corrupt("RLE data without palette".into()));
        };
        let level = &self.series[0].levels[0];
        scratch::with(self.data_len as usize, |data| {
            read_exact_at(&self.file, data, self.data_offset)?;
            Ok(decode_rle(data, bits, level.size_x as usize, level.size_y as usize))
        })
    }
}

impl Backend for Bmp {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.series
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut head = [0u8; 124];
        let len = head.len().min(self.file.metadata()?.len().saturating_sub(14) as usize);
        read_exact_at(&self.file, &mut head[..len], 14)?;
        let h = parse_header(&head[..len])?;
        Ok(OriginalMeta { pairs: original_meta(&h, self.top_down), ..Default::default() })
    }

    fn read_native(&self, _series: usize, _level: usize, _plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let width = self.series[0].levels[0].size_x as usize;
        let height = self.series[0].levels[0].size_y as usize;
        let (x, y0, w, rows) = (region.x as usize, region.y as usize, region.width as usize, region.height as usize);

        if self.rle {
            let indices = self.rle_indices()?;
            let Pixels::Indexed { lut, .. } = &self.pixels else { unreachable!("checked in rle_indices") };
            let samples = if lut.gray { 1 } else { 3 };
            for r in 0..rows {
                let src = &indices[(y0 + r) * width + x..][..w];
                sink.write_with(r, 0, w, |dst| {
                    for (out, &i) in dst.chunks_exact_mut(samples).zip(src) {
                        out.copy_from_slice(&lut.rgb[i as usize][..samples]);
                    }
                    Ok(())
                })?;
            }
            return Ok(());
        }

        // Rows of the region are contiguous in the file; read them in chunks.
        let chunk_rows = (CHUNK_BYTES / self.stride).clamp(1, rows);
        scratch::with(chunk_rows * self.stride, |scratch| {
            let mut done = 0;
            while done < rows {
                let n = chunk_rows.min(rows - done);
                let first = y0 + done;
                // Index of the lowest stored row in this chunk.
                let stored = if self.top_down { first } else { height - first - n };
                let block = &mut scratch[..n * self.stride];
                read_exact_at(&self.file, block, self.data_offset + (stored * self.stride) as u64)?;
                for r in 0..n {
                    let s = if self.top_down { r } else { n - 1 - r };
                    let src = &block[s * self.stride..(s + 1) * self.stride];
                    sink.write_with(done + r, 0, w, |dst| {
                        self.convert_row(src, x, dst);
                        Ok(())
                    })?;
                }
                done += n;
            }
            Ok(())
        })
    }
}

/// Parses the information header following the file header.
fn parse_header(b: &[u8]) -> Result<Header> {
    let size = u32_at(b, 0);
    if b.len() < size as usize {
        return Err(Error::Corrupt("file shorter than its header".into()));
    }
    if size == 12 {
        return Ok(Header {
            size,
            width: u16_at(b, 4) as i32,
            height: u16_at(b, 6) as i32,
            bits: u16_at(b, 10),
            compression: BI_RGB,
            image_size: 0,
            x_ppm: 0,
            y_ppm: 0,
            colors_used: 0,
            colors_important: 0,
            masks: [0; 4],
        });
    }
    let mut masks = [0; 4];
    if size >= 52 {
        for (i, m) in masks.iter_mut().enumerate().take(if size >= 56 { 4 } else { 3 }) {
            *m = u32_at(b, 40 + i * 4);
        }
    }
    Ok(Header {
        size,
        width: u32_at(b, 4) as i32,
        height: u32_at(b, 8) as i32,
        bits: u16_at(b, 14),
        compression: u32_at(b, 16),
        image_size: u32_at(b, 20),
        x_ppm: u32_at(b, 24) as i32,
        y_ppm: u32_at(b, 28) as i32,
        colors_used: u32_at(b, 32),
        colors_important: u32_at(b, 36),
        masks,
    })
}

/// Reads the colour table and pads it to 256 entries.
///
/// At most `room` entries are read if the table would otherwise overlap
/// the pixel data.
fn read_palette(
    file: &File,
    offset: u64,
    entry: usize,
    bits: u16,
    used: u32,
    room: u64,
) -> Result<Lut> {
    let max = 1usize << bits;
    let count = if used == 0 || used as usize > max { max } else { used as usize };
    let count = if room > 0 { count.min(room as usize) } else { count };
    let mut raw = vec![0u8; count * entry];
    read_exact_at(file, &mut raw, offset)?;
    let mut rgb = Box::new([[0u8; 3]; 256]);
    for (dst, e) in rgb.iter_mut().zip(raw.chunks_exact(entry)) {
        *dst = [e[2], e[1], e[0]];
    }
    let gray = rgb[..max].iter().all(|c| c[0] == c[1] && c[1] == c[2]);
    let identity = gray && bits == 8 && rgb.iter().enumerate().all(|(i, c)| c[0] as usize == i);
    Ok(Lut { rgb, gray, identity })
}

/// Builds the sample masks of packed pixels, alpha last if present.
fn packed_masks(rgb: &[u32], alpha: u32) -> Result<Vec<Mask>> {
    let mut masks = rgb.iter().map(|&m| Mask::new(m)).collect::<Result<Vec<_>>>()?;
    if alpha != 0 {
        masks.push(Mask::new(alpha)?);
    }
    Ok(masks)
}

/// Decodes RLE4 or RLE8 data into palette indices, top row first.
///
/// Pixels not written by the stream (skipped by deltas or early line ends)
/// stay at index 0. Data beyond the image bounds is ignored.
fn decode_rle(data: &[u8], bits: u8, width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; width * height];
    let (mut x, mut y) = (0usize, 0usize);
    let mut put = |x: usize, y: usize, index: u8| {
        if x < width && y < height {
            out[(height - 1 - y) * width + x] = index;
        }
    };
    let mut i = 0;
    while i + 1 < data.len() && y < height {
        let (count, value) = (data[i] as usize, data[i + 1]);
        i += 2;
        match (count, value) {
            (0, 0) => {
                x = 0;
                y += 1;
            }
            (0, 1) => break,
            (0, 2) => {
                let (Some(&dx), Some(&dy)) = (data.get(i), data.get(i + 1)) else { break };
                x += dx as usize;
                y += dy as usize;
                i += 2;
            }
            (0, n) => {
                let n = n as usize;
                let len = if bits == 8 { n } else { n.div_ceil(2) };
                let run = &data[i..data.len().min(i + len)];
                for k in 0..n.min(if bits == 8 { run.len() } else { run.len() * 2 }) {
                    let index = if bits == 8 { run[k] } else { nibble(run[k / 2], k) };
                    put(x, y, index);
                    x += 1;
                }
                i += len + len % 2;
            }
            (n, v) => {
                for k in 0..n {
                    put(x, y, if bits == 8 { v } else { nibble(v, k) });
                    x += 1;
                }
            }
        }
    }
    out
}

/// High nibble for even `k`, low nibble for odd `k`.
fn nibble(byte: u8, k: usize) -> u8 {
    if k % 2 == 0 { byte >> 4 } else { byte & 0x0F }
}

/// Header fields as key-value pairs for [`SeriesMeta::original`].
fn original_meta(h: &Header, top_down: bool) -> Vec<(String, String)> {
    let mut v = vec![
        ("Header size".to_string(), h.size.to_string()),
        ("Width".to_string(), h.width.to_string()),
        ("Height".to_string(), h.height.unsigned_abs().to_string()),
        ("Bits per pixel".to_string(), h.bits.to_string()),
        ("Compression".to_string(), h.compression.to_string()),
        ("Top-down".to_string(), top_down.to_string()),
    ];
    if h.size > 12 {
        v.extend([
            ("Image size".to_string(), h.image_size.to_string()),
            ("X pixels per metre".to_string(), h.x_ppm.to_string()),
            ("Y pixels per metre".to_string(), h.y_ppm.to_string()),
            ("Colours used".to_string(), h.colors_used.to_string()),
            ("Colours important".to_string(), h.colors_important.to_string()),
        ]);
    }
    v
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_scales_to_8_bits() {
        let m = Mask::new(0x7C00).unwrap();
        assert_eq!((m.sample(0x7C00), m.sample(0), m.sample(1 << 10)), (255, 0, 8));
        assert!(Mask::new(0x0F0F).is_err());
        assert!(matches!(Mask::new(0x3FF), Err(Error::UnsupportedVariant(_))));
    }

    #[test]
    fn rle8_follows_documented_example() {
        // Example from doc/bmp/ms-bitmap-compression.html, without the delta.
        let data = [3, 4, 5, 6, 0, 3, 0x45, 0x56, 0x67, 0, 2, 0x78, 0, 0, 9, 0x1E, 0, 1];
        let out = decode_rle(&data, 8, 13, 2);
        // Bottom row first in the stream, top row first in the output.
        assert_eq!(&out[13..], &[4, 4, 4, 6, 6, 6, 6, 6, 0x45, 0x56, 0x67, 0x78, 0x78]);
        assert_eq!(&out[..13], &[0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0, 0, 0, 0]);
    }

    #[test]
    fn rle4_follows_documented_example() {
        let data = [3, 4, 5, 6, 0, 6, 0x45, 0x56, 0x67, 0, 4, 0x78, 0, 0, 9, 0x1E, 0, 1];
        let out = decode_rle(&data, 4, 18, 2);
        assert_eq!(&out[18..], &[0, 4, 0, 0, 6, 0, 6, 0, 4, 5, 5, 6, 6, 7, 7, 8, 7, 8]);
        assert_eq!(&out[..9], &[1, 0xE, 1, 0xE, 1, 0xE, 1, 0xE, 1]);
    }

    #[test]
    fn rle_delta_skips_pixels_and_ignores_overflow() {
        let data = [2, 1, 0, 2, 1, 1, 5, 3, 0, 1];
        let out = decode_rle(&data, 8, 4, 2);
        // Bottom row: 1 1 0 0; delta to (3, 1); top row: 0 0 0 3, rest clipped.
        assert_eq!(out, vec![0, 0, 0, 3, 1, 1, 0, 0]);
    }
}

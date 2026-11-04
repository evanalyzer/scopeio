//! # dib
//!
//! Device-independent bitmap pieces shared by the BMP and AVI readers: the
//! `BITMAPINFOHEADER` family, colour tables, bit masks, row conversion to
//! gray / RGB(A) samples and RLE4 / RLE8 decoding.
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

use crate::{Error, Result};

pub(super) const BI_RGB: u32 = 0;
pub(super) const BI_RLE8: u32 = 1;
pub(super) const BI_RLE4: u32 = 2;
pub(super) const BI_BITFIELDS: u32 = 3;
pub(super) const BI_ALPHABITFIELDS: u32 = 6;
/// Pixel encoding of the bitmap data.
#[derive(Debug)]
pub(super) enum Pixels {
    /// Palette indices of `bits` bits.
    Indexed { bits: u8, lut: Lut },
    /// 3 bytes per pixel, blue first.
    Bgr24,
    /// 2 or 4 bytes per pixel, little-endian, split by bit masks.
    Packed { bytes: u8, masks: Vec<Mask> },
}

/// Palette padded to 256 entries; missing entries are black.
#[derive(Debug)]
pub(super) struct Lut {
    pub rgb: Box<[[u8; 3]; 256]>,
    /// All entries have equal red, green and blue.
    pub gray: bool,
    /// Gray palette mapping each index to itself.
    pub identity: bool,
}

/// One sample within a packed pixel.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mask {
    pub shift: u32,
    pub max: u32,
}

impl Mask {
    pub fn new(mask: u32) -> Result<Self> {
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

    pub fn sample(self, pixel: u32) -> u8 {
        let v = (pixel >> self.shift) & self.max;
        if self.max == 0xFF { v as u8 } else { ((v * 255 + self.max / 2) / self.max) as u8 }
    }
}

/// Fields of the bitmap information header.
pub(super) struct Header {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub bits: u16,
    pub compression: u32,
    pub image_size: u32,
    pub x_ppm: i32,
    pub y_ppm: i32,
    pub colors_used: u32,
    pub colors_important: u32,
    pub masks: [u32; 4],
}

impl Pixels {
    /// Converts one stored row into output samples.
    ///
    /// `src` holds the stored row from its first byte, `dst` the output row
    /// of `width` pixels starting at pixel `x`.
    pub fn convert_row(&self, src: &[u8], x: usize, dst: &mut [u8]) {
        match self {
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
                    let pixel = if n == 2 { u16::from_le_bytes([px[0], px[1]]) as u32 } else { u32::from_le_bytes([px[0], px[1], px[2], px[3]]) };
                    for (o, m) in out.iter_mut().zip(masks) {
                        *o = m.sample(pixel);
                    }
                }
            }
        }
    }
}

/// Parses the information header following the file header.
pub(super) fn parse_header(b: &[u8]) -> Result<Header> {
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

/// Palette from `raw` colour table entries of `entry` bytes (blue, green,
/// red[, reserved]), padded to 256 entries.
pub(super) fn palette(raw: &[u8], entry: usize, bits: u16) -> Lut {
    let max = 1usize << bits.min(8);
    let mut rgb = Box::new([[0u8; 3]; 256]);
    for (dst, e) in rgb.iter_mut().zip(raw.chunks_exact(entry.max(3))) {
        *dst = [e[2], e[1], e[0]];
    }
    let gray = rgb[..max].iter().all(|c| c[0] == c[1] && c[1] == c[2]);
    let identity = gray && bits == 8 && rgb.iter().enumerate().all(|(i, c)| c[0] as usize == i);
    Lut { rgb, gray, identity }
}

/// Samples per output pixel: 1 for gray palettes, else 3 or the masks.
pub(super) fn samples_per_pixel(pixels: &Pixels) -> u8 {
    match pixels {
        Pixels::Indexed { lut, .. } if lut.gray => 1,
        Pixels::Indexed { .. } | Pixels::Bgr24 => 3,
        Pixels::Packed { masks, .. } => masks.len() as u8,
    }
}

/// Builds the sample masks of packed pixels, alpha last if present.
pub(super) fn packed_masks(rgb: &[u32], alpha: u32) -> Result<Vec<Mask>> {
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
pub(super) fn decode_rle(data: &[u8], bits: u8, width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; width * height];
    decode_rle_into(data, bits, width, height, &mut out);
    out
}

/// Decodes RLE4 or RLE8 data onto `out` (palette indices, top row first,
/// `width * height`): pixels the stream skips keep their value, as needed
/// for the delta frames of RLE video.
pub(super) fn decode_rle_into(data: &[u8], bits: u8, width: usize, height: usize, out: &mut [u8]) {
    if out.len() < width.saturating_mul(height) {
        return;
    }
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
}

/// High nibble for even `k`, low nibble for odd `k`.
pub(super) fn nibble(byte: u8, k: usize) -> u8 {
    if k.is_multiple_of(2) { byte >> 4 } else { byte & 0x0F }
}

/// Header fields as key-value pairs for [`SeriesMeta::original`].
pub(super) fn original_meta(h: &Header, top_down: bool) -> Vec<(String, String)> {
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

pub(super) fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(super) fn u32_at(b: &[u8], at: usize) -> u32 {
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

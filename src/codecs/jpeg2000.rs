//! # jpeg2000
//!
//! JPEG 2000 codestream and JP2 decoding via `hayro-jpeg2000`, at the
//! native bit depth.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};

use crate::{Error, PixelType, Result};

/// Size of a decoded JPEG 2000 image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Info {
    pub width: usize,
    pub height: usize,
    pub components: usize,
}

/// Size, components and bit depth of a JPEG 2000 codestream or JP2 file,
/// from its headers.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid headers.
pub(crate) fn info(data: &[u8]) -> Result<(Info, u8)> {
    info_at(data, 0)
}

/// Settings decoding `reduce` resolution levels below the full image.
fn settings(data: &[u8], reduce: u8) -> DecodeSettings {
    let mut s = DecodeSettings::default();
    if reduce > 0
        && let Ok(full) = Image::new(data, &s)
    {
        s.target_resolution = Some(((full.width() >> reduce).max(1), (full.height() >> reduce).max(1)));
    }
    s
}

/// Like [`info`], for the image reduced by `reduce` resolution levels; the
/// size stops shrinking below the codestream's lowest level.
pub(crate) fn info_at(data: &[u8], reduce: u8) -> Result<(Info, u8)> {
    let image = Image::new(data, &settings(data, reduce)).map_err(|e| Error::Corrupt(format!("JPEG 2000: {e:?}")))?;
    let components = image.color_space().num_channels() as usize + usize::from(image.has_alpha());
    Ok((Info { width: image.width() as usize, height: image.height() as usize, components }, image.original_bit_depth()))
}

/// Returns `true` if `data` is a raw codestream (no JP2 boxes) whose decoded
/// samples come back as RGB although they are stored as YCbCr: three
/// components with the last two subsampled alike and more coarsely than
/// the first. hayro-jpeg2000 (since 0.5) takes those for sYCC and converts
/// them; other raw codestreams are returned as stored.
///
/// Reads the SIZ marker segment (ITU-T T.800 A.5.1), which follows SOC.
pub(crate) fn converts_ycc(data: &[u8]) -> bool {
    // SOC, SIZ, Lsiz, Rsiz, 8 x 32-bit sizes, Csiz: 42 bytes, then 3 bytes per component.
    if data.len() < 42 + 9 || data[..4] != [0xFF, 0x4F, 0xFF, 0x51] || u16::from_be_bytes([data[40], data[41]]) != 3 {
        return false;
    }
    let sub = |c: usize| (data[42 + 3 * c + 1], data[42 + 3 * c + 2]);
    let (y, cb, cr) = (sub(0), sub(1), sub(2));
    cb == cr && cb != y && cb.0 >= y.0 && cb.1 >= y.1
}

/// Decodes a JPEG 2000 codestream (or JP2 file) into `out`: interleaved
/// components as native samples of `pixel_type` (`U8` or `U16`).
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid streams or if `out` is too small,
/// [`Error::UnsupportedVariant`] for other output types.
pub(crate) fn decode(data: &[u8], pixel_type: PixelType, out: &mut [u8]) -> Result<Info> {
    decode_at(data, pixel_type, out, 0)
}

/// Like [`decode`], reduced by `reduce` resolution levels (see [`info_at`]).
pub(crate) fn decode_at(data: &[u8], pixel_type: PixelType, out: &mut [u8], reduce: u8) -> Result<Info> {
    let err = |e: hayro_jpeg2000::DecodeError| Error::Corrupt(format!("JPEG 2000: {e:?}"));
    let image = Image::new(data, &settings(data, reduce)).map_err(err)?;
    let mut context = DecoderContext::default();
    let decoded = image.decode(&mut context).map_err(err)?;
    let components = decoded.components();
    let width = image.width() as usize;
    let height = image.height() as usize;
    let n = components.len();
    let bytes = pixel_type.bytes();
    if width * height * n * bytes > out.len() {
        return Err(Error::Corrupt(format!("JPEG 2000 image {width}x{height}x{n} does not fit the tile")));
    }
    for (k, c) in components.iter().enumerate() {
        let max = ((1u64 << c.bit_depth().min(16)) - 1) as f32;
        let samples = c.samples();
        match pixel_type {
            PixelType::U8 => {
                for (i, v) in samples.iter().take(width * height).enumerate() {
                    out[i * n + k] = v.round().clamp(0.0, max) as u8;
                }
            }
            PixelType::U16 => {
                for (i, v) in samples.iter().take(width * height).enumerate() {
                    let at = (i * n + k) * 2;
                    out[at..at + 2].copy_from_slice(&(v.round().clamp(0.0, max) as u16).to_ne_bytes());
                }
            }
            t => return Err(Error::UnsupportedVariant(format!("JPEG 2000 into {t:?}"))),
        }
    }
    Ok(Info { width, height, components: n })
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// SOC and a SIZ segment with three components of the given subsampling.
    fn siz(sub: [(u8, u8); 3]) -> Vec<u8> {
        let mut d = vec![0xFF, 0x4F, 0xFF, 0x51, 0, 47, 0, 0];
        d.extend([0u8; 32]);
        d.extend(3u16.to_be_bytes());
        for (x, y) in sub {
            d.extend([7, x, y]);
        }
        d
    }

    #[test]
    fn detects_subsampled_chroma() {
        assert!(converts_ycc(&siz([(1, 1), (2, 2), (2, 2)])));
        assert!(converts_ycc(&siz([(1, 1), (2, 1), (2, 1)])));
        assert!(!converts_ycc(&siz([(1, 1), (1, 1), (1, 1)])));
        assert!(!converts_ycc(&siz([(1, 1), (2, 2), (1, 1)])));
        assert!(!converts_ycc(&[0xFF, 0x4F]));
    }
}

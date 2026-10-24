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

//! # jpeg
//!
//! Baseline and progressive 8-bit JPEG decoding via `zune-jpeg`, including
//! the abbreviated streams with shared tables used by TIFF (TIFF Technical
//! Note 2).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::borrow::Cow;

use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::{Error, Result};

/// How decoded colour components are returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Color {
    /// Convert YCbCr to RGB (JFIF, TIFF photometric YCbCr).
    YcbcrToRgb,
    /// Return the components as stored (TIFF photometric RGB or gray).
    AsStored,
}

/// Size of a decoded JPEG image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Info {
    pub width: usize,
    pub height: usize,
    pub components: usize,
}

/// Joins shared tables (a tables-only stream) and an abbreviated image stream.
fn with_tables<'a>(data: &'a [u8], tables: Option<&[u8]>) -> Cow<'a, [u8]> {
    match tables {
        Some(t) if t.len() > 4 && data.len() > 2 && data[..2] == [0xFF, 0xD8] => {
            // Tables without their EOI, image without its SOI.
            let t = if t.ends_with(&[0xFF, 0xD9]) { &t[..t.len() - 2] } else { t };
            let mut joined = Vec::with_capacity(t.len() + data.len() - 2);
            joined.extend_from_slice(t);
            joined.extend_from_slice(&data[2..]);
            Cow::Owned(joined)
        }
        _ => Cow::Borrowed(data),
    }
}

fn options(out: ColorSpace) -> DecoderOptions {
    DecoderOptions::new_fast().jpeg_set_out_colorspace(out).set_strict_mode(false)
}

/// Decodes a JPEG stream into `out`, interleaved, 8 bits per sample.
///
/// # Arguments
///
/// * `data` - JPEG stream, possibly abbreviated (without tables).
/// * `tables` - Shared tables-only stream, e.g. TIFF `JPEGTables`.
/// * `color` - Colour conversion to apply.
/// * `out` - Output buffer; the decoded image must fit.
///
/// # Returns
///
/// The size of the decoded image.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] for precisions other than 8 bits and
/// [`Error::Corrupt`] for invalid streams or if `out` is too small.
pub(crate) fn decode(data: &[u8], tables: Option<&[u8]>, color: Color, out: &mut [u8]) -> Result<Info> {
    let data = with_tables(data, tables);
    let err = |e: zune_jpeg::errors::DecodeErrors| {
        let msg = e.to_string();
        if msg.contains("precision") { Error::UnsupportedVariant(format!("JPEG: {msg}")) } else { Error::Corrupt(format!("JPEG: {msg}")) }
    };
    let mut dec = JpegDecoder::new_with_options(ZCursor::new(&*data), options(ColorSpace::RGB));
    dec.decode_headers().map_err(err)?;
    let info = dec.info().ok_or_else(|| Error::Corrupt("JPEG without frame header".into()))?;
    let components = info.components as usize;
    if components != 1 && components != 3 {
        return Err(Error::UnsupportedVariant(format!("JPEG with {components} components")));
    }
    let input = dec.input_colorspace().unwrap_or(ColorSpace::YCbCr);
    let space = match (components, color) {
        (1, _) => ColorSpace::Luma,
        // Requesting the input colour space disables conversion.
        (_, Color::AsStored) => input,
        (_, Color::YcbcrToRgb) => ColorSpace::RGB,
    };
    dec.set_options(options(space));
    let size = dec.output_buffer_size().ok_or_else(|| Error::Corrupt("JPEG without frame header".into()))?;
    if size > out.len() {
        return Err(Error::Corrupt(format!("JPEG decodes to {size} bytes, expected at most {}", out.len())));
    }
    dec.decode_into(&mut out[..size]).map_err(err)?;
    Ok(Info { width: info.width as usize, height: info.height as usize, components })
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_tables_and_abbreviated_stream() {
        let tables = [0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x02, 0xFF, 0xD9];
        let image = [0xFF, 0xD8, 0xFF, 0xDA, 0xFF, 0xD9];
        assert_eq!(&*with_tables(&image, Some(&tables)), &[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x02, 0xFF, 0xDA, 0xFF, 0xD9]);
        assert_eq!(&*with_tables(&image, None), &image);
    }
}

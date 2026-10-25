//! # nef
//!
//! Nikon NEF camera raw files: a TIFF whose first IFD is a small RGB
//! thumbnail; its SubIFDs hold a JPEG preview and the raw sensor image
//! (photometric CFA, 32803) with the colour filter pattern in tags 33421
//! (repeat size) and 33422 (pattern, 0 red, 1 green, 2 blue). The raw image
//! is returned as one channel of sensor values (not demosaiced); the
//! pattern is in the original metadata. Uncompressed raw data (12 or 14
//! bits, packed) is read with the TIFF core; Nikon's compressed encoding
//! (compression 34713) is not supported.
//!
//! Provenance: TIFF 6.0 and TIFF/EP (CFA photometric and pattern tags);
//! tifffile (BSD-3-Clause, see THIRD_PARTY_NOTICES.md) for the compression
//! code 34713; own analysis of public NEF samples (raw.pixls.us, CC0);
//! compared with Bio-Formats as a black box. No GPL or LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use super::ifd::{Ifd, TiffFile, tag};
use super::image::TiffImage;
use super::{Series, make_series};
use crate::{Error, Result};

const PHOTOMETRIC_CFA: u64 = 32803;
const CFA_REPEAT: u16 = 33421;
const CFA_PATTERN: u16 = 33422;
/// Nikon's compressed raw encoding.
const NIKON_COMPRESSION: u64 = 34713;

/// The raw sensor IFD among the SubIFDs of the first IFD.
fn raw_ifd(tiff: &TiffFile) -> Option<Ifd> {
    let subs = tiff.sub_ifds(&tiff.ifds[0]).ok()?;
    subs.into_iter().find(|s| tiff.uint(s, tag::PHOTOMETRIC).ok().flatten() == Some(PHOTOMETRIC_CFA))
}

/// Returns `true` for a Nikon file with a raw CFA SubIFD.
pub(super) fn matches(tiff: &TiffFile) -> bool {
    let make = tiff.string(&tiff.ifds[0], tag::MAKE).ok().flatten().unwrap_or_default();
    make.trim().to_ascii_uppercase().starts_with("NIKON") && raw_ifd(tiff).is_some()
}

/// Builds the series of the raw sensor image.
///
/// # Errors
///
/// Returns [`Error::UnsupportedVariant`] for Nikon-compressed raw data.
pub(super) fn series(tiff: &TiffFile) -> Result<Vec<Series>> {
    let raw = raw_ifd(tiff).ok_or_else(|| Error::Corrupt("NEF without raw image".into()))?;
    if tiff.uint(&raw, tag::COMPRESSION)? == Some(NIKON_COMPRESSION) {
        return Err(Error::UnsupportedVariant("Nikon-compressed NEF raw data (compression 34713); only uncompressed NEF is supported".into()));
    }
    let image = TiffImage::new(tiff, &raw)?;
    let mut s = make_series(tiff, 0, 0, vec![vec![image]])?;
    // Camera print resolution, not a physical pixel size.
    s.meta.physical_size = Default::default();
    let repeat = tiff.uints(&raw, CFA_REPEAT)?.unwrap_or_default();
    let pattern = tiff.uints(&raw, CFA_PATTERN)?.unwrap_or_default();
    if repeat.len() == 2 && repeat[0] * repeat[1] == pattern.len() as u64 && !pattern.is_empty() {
        let letters: String = pattern.iter().map(|&c| match c {
            0 => 'R',
            1 => 'G',
            2 => 'B',
            _ => '?',
        }).collect();
        s.original.push(("CFA pattern".into(), format!("{letters} ({} x {})", repeat[0], repeat[1])));
        s.meta.channels[0].name = Some(format!("CFA {letters}"));
    }
    for (t, key) in [(tag::MAKE, "Make"), (tag::MODEL, "Model")] {
        if let Some(v) = tiff.string(&tiff.ifds[0], t)? {
            s.original.push((key.into(), v.trim().to_string()));
        }
    }
    Ok(vec![s])
}

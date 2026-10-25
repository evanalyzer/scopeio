//! # sis
//!
//! Olympus Soft Imaging Solutions TIFF (analySIS, cellSens, iTEM): a plain
//! TIFF whose first IFD has the OlympusSIS tag (33560) pointing to a
//! little-endian `SIS0` record: acquisition time (minute, hour, day, month
//! from 0, year from 1900), image name, then sub-records of (type, count,
//! offset). Sub-record 1 holds the pixel size as metres (`x`, `y` times
//! 10^exponent), the magnification, camera name and picture type.
//! analySIS writes gray images as palette images whose map is a gray ramp
//! with a few overlay colours; the indices are intensities.
//!
//! Provenance: tifffile (BSD-3-Clause, Christoph Gohlke; see
//! THIRD_PARTY_NOTICES.md) read as documentation of the `SIS0` record; own
//! analysis of public analySIS 5.0 TEM images (Figshare, CC BY 4.0);
//! checked against Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use super::Series;
use super::ifd::{Ifd, TiffFile};
use crate::Result;

pub(super) const OLYMPUS_SIS: u16 = 33560;
pub(super) const OLYMPUS_INI: u16 = 33471;
/// Most sub-records read, against corrupt counts.
const MAX_RECORDS: usize = 256;

/// Returns `true` if `ifd` has the OlympusSIS or OlympusINI tag.
pub(super) fn matches(ifd: &Ifd) -> bool {
    ifd.has(OLYMPUS_SIS) || ifd.has(OLYMPUS_INI)
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

fn i16_le(b: &[u8]) -> i16 {
    i16::from_le_bytes([b[0], b[1]])
}

fn f64_le(b: &[u8]) -> f64 {
    f64::from_le_bytes(b[..8].try_into().expect("8 bytes"))
}

/// Adds the `SIS0` metadata to the series. Unreadable records are ignored:
/// the image itself is a plain TIFF.
pub(super) fn apply(s: &mut Series, tiff: &TiffFile) -> Result<()> {
    let Some(at) = tiff.uint(&tiff.ifds[0], OLYMPUS_SIS)? else { return Ok(()) };
    let Ok(h) = tiff.read_bytes(at, 60) else { return Ok(()) };
    if &h[..4] != b"SIS0" {
        return Ok(());
    }
    let (minute, hour, day, month, year) = (i16_le(&h[10..]), i16_le(&h[12..]), i16_le(&h[14..]), i16_le(&h[16..]), i16_le(&h[18..]));
    if (0..60).contains(&minute) && (0..24).contains(&hour) && (1..32).contains(&day) && (0..12).contains(&month) && year > 0 {
        s.meta.acquisition_date = Some(format!("{:04}-{:02}-{:02}T{:02}:{:02}:00", 1900 + year as i32, month + 1, day, hour, minute));
    }
    let name = text(&h[26..58]);
    if !name.is_empty() {
        s.meta.name = Some(name.clone());
        s.original.push(("Name".into(), name));
    }
    let count = (u16::from_le_bytes([h[58], h[59]]) as usize).min(MAX_RECORDS);
    let Ok(records) = tiff.read_bytes(at + 60, 8 * count) else { return Ok(()) };
    for r in records.chunks_exact(8) {
        let offset = u32::from_le_bytes(r[4..8].try_into().expect("4 bytes")) as u64;
        // General data: pixel size, magnification, camera, picture type.
        if i16_le(r) != 1 {
            continue;
        }
        let Ok(g) = tiff.read_bytes(offset, 112) else { continue };
        let exponent = i16_le(&g[10..]) as i32;
        let metres = 10f64.powi(exponent);
        let size = |v: f64| Some(v * metres * 1e9).filter(|v| v.is_finite() && *v > 0.0);
        if let Some(x) = size(f64_le(&g[12..])) {
            s.meta.physical_size.x_nm = Some(x);
            s.meta.physical_size.y_nm = size(f64_le(&g[20..])).or(Some(x));
        }
        let magnification = f64_le(&g[36..]);
        if magnification.is_finite() && magnification > 0.0 {
            s.original.push(("Magnification".into(), magnification.to_string()));
        }
        for (key, value) in [("Camera", text(&g[46..80])), ("Picture type", text(&g[80..112]))] {
            if !value.is_empty() {
                s.original.push((key.into(), value));
            }
        }
    }
    Ok(())
}

//! # fluoview
//!
//! Olympus FluoView TIFF (FV300, FV500): one IFD per plane; the first IFD
//! carries MM_Header (tag 34361), a packed record with the image name and
//! ten dimensions (name, size, origin, resolution, unit) at byte 284. X and
//! Y come first, the further dimensions are listed fastest first: `Z`,
//! `T`/`Time`, `Ch`/`Wavelength`; others count as time points. Pixel sizes
//! and the time step are the resolutions of X, Y, Z and T in their units.
//!
//! Provenance: tifffile (BSD-3-Clause, Christoph Gohlke; see
//! THIRD_PARTY_NOTICES.md) read as documentation of the MM_Header record
//! and its dimension names; synthetic files checked against Bio-Formats as
//! a black box (dimension order and pixel sizes). No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use super::ifd::TiffFile;
use super::image::Samples;
use super::{PlaneRef, Series};
use crate::Result;

pub(super) const MM_HEADER: u16 = 34361;
/// Packed size of MM_Header.
const HEADER_LEN: usize = 1454;
/// Byte offset and size of the dimension records.
const DIMENSIONS: usize = 284;
const DIMENSION_LEN: usize = 100;

/// Axis a FluoView dimension maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    C,
    Z,
    T,
}

/// One dimension record.
#[derive(Debug)]
struct Dimension {
    name: String,
    size: u32,
    resolution: f64,
    unit: String,
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

/// Nanometres per length unit, seconds per time unit.
fn scale(unit: &str) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "nm" => Some(1.0),
        "um" | "µm" | "micron" | "microns" => Some(1e3),
        "mm" => Some(1e6),
        "s" | "sec" => Some(1.0),
        "ms" => Some(1e-3),
        "min" => Some(60.0),
        "h" => Some(3600.0),
        _ => None,
    }
}

/// Turns the plain series of a FluoView file into its C/Z/T stack. Returns
/// `false` if the file has no usable MM_Header (the plain series stays).
pub(super) fn apply(s: &mut Series, tiff: &TiffFile) -> Result<bool> {
    let ifd = &tiff.ifds[0];
    let Some(entry) = ifd.entry(MM_HEADER) else { return Ok(false) };
    if entry.count < HEADER_LEN as u64 || s.levels.is_empty() || s.meta.size_c != 1 || s.meta.samples_per_pixel != 1 {
        return Ok(false);
    }
    let h = tiff.read_bytes(tiff.value_offset(entry), HEADER_LEN)?;
    let f64_at = |o: usize| f64::from_bits(tiff.u64(&h[o..]));
    let dims: Vec<Dimension> = (0..10)
        .map(|i| {
            let d = DIMENSIONS + DIMENSION_LEN * i;
            Dimension {
                name: text(&h[d..d + 16]),
                size: tiff.u32(&h[d + 16..]).min(1 << 24),
                resolution: f64_at(d + 28),
                unit: text(&h[d + 36..d + 100]),
            }
        })
        .collect();
    // Dimensions beyond X and Y, fastest first, with their page stride.
    let mut axes: Vec<(Axis, u32, u64)> = Vec::new();
    let mut stride = 1u64;
    for d in dims.iter().skip(2).filter(|d| d.size > 1) {
        let axis = match d.name.to_ascii_uppercase().as_str() {
            "Z" => Axis::Z,
            "CH" | "WAVELENGTH" => Axis::C,
            _ => Axis::T,
        };
        axes.push((axis, d.size, stride));
        stride = stride.saturating_mul(d.size as u64);
    }
    let pages = s.levels[0].len() as u64;
    if stride != pages {
        return Ok(false);
    }
    let size = |a: Axis| axes.iter().filter(|x| x.0 == a).map(|x| x.1).product::<u32>();
    let (size_c, size_z, size_t) = (size(Axis::C), size(Axis::Z), size(Axis::T));
    // Page of an axis index: mixed radix over that axis's dimensions.
    let page_part = |a: Axis, mut index: u32| -> u64 {
        let mut page = 0;
        for &(_, n, stride) in axes.iter().filter(|x| x.0 == a) {
            page += (index % n) as u64 * stride;
            index /= n;
        }
        page
    };
    let mut planes = Vec::with_capacity(pages as usize);
    for t in 0..size_t {
        for z in 0..size_z {
            for c in 0..size_c {
                let page = page_part(Axis::C, c) + page_part(Axis::Z, z) + page_part(Axis::T, t);
                planes.push(Some(PlaneRef { page: page as usize, samples: Samples::One(0) }));
            }
        }
    }
    s.planes = planes;
    s.meta.size_c = size_c;
    s.meta.size_z = size_z;
    s.meta.size_t = size_t;
    let channel = s.meta.channels.first().cloned().unwrap_or_default();
    s.meta.channels = vec![channel; size_c as usize];
    // Pixel sizes and time step from the resolutions.
    let step = |d: &Dimension| scale(&d.unit).map(|f| d.resolution * f).filter(|v| v.is_finite() && *v > 0.0);
    let by_name = |name: &str| dims.iter().find(|d| d.name.eq_ignore_ascii_case(name) && d.size > 0);
    if let Some(x) = by_name("X").and_then(step) {
        s.meta.physical_size.x_nm = Some(x);
        s.meta.physical_size.y_nm = by_name("Y").and_then(step).or(Some(x));
    }
    s.meta.physical_size.z_nm = by_name("Z").and_then(step).filter(|_| size_z > 1);
    s.meta.time_increment_s = by_name("T").or_else(|| by_name("Time")).and_then(step).filter(|_| size_t > 1);
    let name = text(&h[3..260]);
    if !name.is_empty() {
        s.meta.name = Some(name.clone());
        s.original.push(("ImageName".into(), name));
    }
    for d in dims.iter().filter(|d| !d.name.is_empty()) {
        s.original.push((format!("Dimension {}", d.name), format!("{} x {} {}", d.size, d.resolution, d.unit).trim_end().to_string()));
    }
    Ok(true)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units() {
        assert_eq!(scale("um"), Some(1e3));
        assert_eq!(scale("ms"), Some(1e-3));
        assert_eq!(scale(""), None);
        assert_eq!(text(b"Ch\0\0junk"), "Ch");
    }
}

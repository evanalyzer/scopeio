//! # stk
//!
//! MetaMorph STK stacks: a TIFF whose first IFD describes one plane, with
//! the further planes stored contiguously after it. The UIC2 tag (33629)
//! has one record per plane (z distance as a fraction, creation and
//! modification Julian day and milliseconds); UIC1 (33628) holds tagged
//! values such as the x/y calibration (fractions) and its unit. A stack
//! whose z distances are all zero is a time series, otherwise a z stack.
//!
//! Provenance: Molecular Devices "MetaMorph STK and TIFF file formats"
//! (T10243, in `doc/metamorph-stk/`); tifffile (BSD-3-Clause, Christoph
//! Gohlke; see THIRD_PARTY_NOTICES.md) read as documentation of the UIC
//! tags; own analysis of public sample files (Figshare, CC0 / CC BY 4.0);
//! checked against Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use super::ifd::TiffFile;
use super::{PlaneRef, Series, Slot};
use crate::Result;

pub(super) const UIC1: u16 = 33628;
pub(super) const UIC2: u16 = 33629;
/// UIC1 tag ids of the calibration.
const X_CALIBRATION: u32 = 4;
const Y_CALIBRATION: u32 = 5;
const CALIBRATION_UNITS: u32 = 6;
/// Most planes accepted, against corrupt counts.
const MAX_PLANES: u64 = 1 << 20;

fn u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes(b[..4].try_into().expect("4 bytes"))
}

/// Nanometres per calibration unit.
fn unit_nm(unit: &str) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "um" | "µm" | "micron" | "microns" | "micrometer" => Some(1e3),
        "nm" | "nanometer" => Some(1.0),
        "mm" | "millimeter" => Some(1e6),
        _ => None,
    }
}

/// Pixel size (x, y in nm) of a MetaMorph 7.5+ TIFF: `<prop>` entries
/// `spatial-calibration-state` (`on`), `-x`, `-y` and `-units` in the XML
/// ImageDescription (`<MetaData>`).
pub(super) fn metaseries_calibration(description: &str) -> Option<(f64, f64)> {
    if !description.trim_start().starts_with("<MetaData>") {
        return None;
    }
    let prop = |id: &str| -> Option<&str> {
        let key = format!("id=\"{id}\"");
        let at = description.find(&key)?;
        let rest = &description[at..description[at..].find("/>").map_or(description.len(), |e| at + e)];
        let v = rest.find("value=\"")? + 7;
        Some(&rest[v..v + rest[v..].find('"')?])
    };
    if prop("spatial-calibration-state") != Some("on") {
        return None;
    }
    let nm = unit_nm(prop("spatial-calibration-units")?)?;
    let x: f64 = prop("spatial-calibration-x")?.parse().ok().filter(|v: &f64| *v > 0.0)?;
    let y: f64 = prop("spatial-calibration-y").and_then(|v| v.parse().ok()).filter(|v: &f64| *v > 0.0).unwrap_or(x);
    Some((x * nm, y * nm))
}

/// Turns a one-plane series of an STK file into its stack. Returns `false`
/// for files without UIC2 tag or with non-contiguous planes.
pub(super) fn apply(s: &mut Series, tiff: &TiffFile) -> Result<bool> {
    let ifd = &tiff.ifds[0];
    let Some(uic2) = ifd.entry(UIC2) else { return Ok(false) };
    let n = uic2.count;
    if n == 0 || n > MAX_PLANES || s.levels.len() != 1 || s.levels[0].len() != 1 || s.meta.samples_per_pixel != 1 || s.meta.size_c != 1 {
        return Ok(false);
    }
    let Some(Slot::Ready(first)) = &s.levels[0][0] else { return Ok(false) };
    let Some(plane_bytes) = first.image.contiguous_size(tiff)? else { return Ok(false) };
    let file = first.file;
    if first.image.data_start(tiff)?.saturating_add(plane_bytes.saturating_mul(n)) > tiff.len {
        return Ok(false);
    }
    let images: Vec<_> = (1..n).map(|k| Some(Slot::ready(file, first.image.shifted(k * plane_bytes)))).collect();
    // Plane records: z distance (fraction), dates and times.
    let records = tiff.read_bytes(tiff.value_offset(uic2), 24 * n as usize)?;
    let z: Vec<f64> = records.chunks_exact(24).map(|r| (u32_le(&r[0..]) as f64) / (u32_le(&r[4..]).max(1) as f64)).collect();
    let millis: Vec<f64> = records.chunks_exact(24).map(|r| u32_le(&r[8..]) as f64 * 86_400_000.0 + u32_le(&r[12..]) as f64).collect();
    s.levels[0].extend(images);
    let z_stack = z.iter().any(|v| *v != 0.0);
    if z_stack {
        s.meta.size_z = n as u32;
    } else {
        s.meta.size_t = n as u32;
    }
    s.planes = (0..n as usize).map(|page| Some(PlaneRef { page, samples: super::image::Samples::One(0) })).collect();
    // Calibration from UIC1: (tag id, value or offset) pairs.
    let mut calibration = (None, None, None);
    if let Some(uic1) = ifd.entry(UIC1).filter(|e| e.count <= 4096) {
        let pairs = tiff.read_bytes(tiff.value_offset(uic1), 8 * uic1.count as usize).unwrap_or_default();
        for p in pairs.chunks_exact(8) {
            let (id, value) = (u32_le(p), u32_le(&p[4..]) as u64);
            match id {
                X_CALIBRATION | Y_CALIBRATION => {
                    if let Ok(f) = tiff.read_bytes(value, 8) {
                        let v = u32_le(&f) as f64 / u32_le(&f[4..]).max(1) as f64;
                        if id == X_CALIBRATION { calibration.0 = Some(v) } else { calibration.1 = Some(v) }
                    }
                }
                CALIBRATION_UNITS => {
                    if let Ok(l) = tiff.read_bytes(value, 4) {
                        let len = (u32_le(&l) as usize).min(256);
                        calibration.2 = tiff.read_bytes(value + 4, len).ok().map(|t| String::from_utf8_lossy(&t).trim_end_matches('\0').to_string());
                    }
                }
                _ => {}
            }
        }
    }
    let nm = calibration.2.as_deref().and_then(unit_nm);
    if let Some(x) = calibration.0.filter(|v| *v > 0.0) {
        // A calibration in pixels (or an unknown unit) is no physical size;
        // without a calibration the TIFF resolution tags stay.
        s.meta.physical_size.x_nm = nm.map(|nm| x * nm);
        s.meta.physical_size.y_nm = nm.map(|nm| calibration.1.filter(|v| *v > 0.0).unwrap_or(x) * nm);
    }
    if z_stack {
        s.meta.physical_size.z_nm = z.iter().copied().find(|v| *v > 0.0).map(|v| v * nm.unwrap_or(1e3));
    } else if n > 1 {
        let step = (millis[n as usize - 1] - millis[0]) / (n - 1) as f64 / 1e3;
        s.meta.time_increment_s = Some(step).filter(|v| *v > 0.0 && v.is_finite());
    }
    Ok(true)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metaseries_xml_calibration() {
        let d = "<MetaData>\n<prop id=\"spatial-calibration-state\" type=\"bool\" value=\"on\"/>\n<prop id=\"spatial-calibration-x\" type=\"float\" value=\"0.65\"/>\n<prop id=\"spatial-calibration-units\" type=\"string\" value=\"um\"/>";
        assert_eq!(metaseries_calibration(d), Some((650.0, 650.0)));
        assert_eq!(metaseries_calibration(&d.replace("\"on\"", "\"off\"")), None);
    }
}

//! # nikon
//!
//! Nikon NIS-Elements TIFF: a plain TIFF whose IFDs carry NIS private tags
//! 65325-65333. Tag 65331 is a small directory (entry count, then per
//! entry a UTF-16 name and four u32: two flags, size and offset of the
//! entry's data within the tag) whose `MetadataTiffV1_0` entry holds the
//! picture metadata in the ND2 "lite variant" encoding
//! (`SLxPictureMetadata`): calibration (µm per pixel), objective name and
//! NA, stage position (µm), absolute time (Julian day) and one plane
//! description per channel.
//!
//! Provenance: own analysis of public NIS-Elements TIFF exports (Zenodo,
//! CC BY 4.0); the lite-variant encoding as documented for ND2 (see
//! `src/formats/nd2/meta.rs`); checked against Bio-Formats as a black box
//! (which reads the pixels but none of this metadata). No GPL code
//! consulted.
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
use crate::formats::nd2::meta::{Value, decode_lv};
use crate::{ObjectiveMeta, Position, Result};

/// NIS private tag with the picture metadata directory.
pub(super) const TAG_METADATA: u16 = 65331;
/// Largest tag value read.
const MAX_TAG: u64 = 16 << 20;
/// Most directory entries read.
const MAX_ENTRIES: u32 = 64;

/// Returns `true` if `ifd` carries the NIS metadata tag.
pub(super) fn matches(ifd: &Ifd) -> bool {
    ifd.has(TAG_METADATA) && ifd.has(65330)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Data of the directory entry `name` of a NIS tag value.
fn entry<'a>(d: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let count = u32_at(d, 0)?.min(MAX_ENTRIES);
    let mut at = 4;
    for _ in 0..count {
        let units = u32_at(d, at)? as usize;
        let raw = d.get(at + 4..at + 4 + 2 * units)?;
        let entry_name = String::from_utf16_lossy(&raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect::<Vec<_>>());
        at += 4 + 2 * units;
        let (size, offset) = (u32_at(d, at + 8)? as usize, u32_at(d, at + 12)? as usize);
        at += 16;
        if entry_name.trim_end_matches('\0') == name {
            return d.get(offset..offset.checked_add(size)?);
        }
    }
    None
}

/// Adds the NIS picture metadata to the series; unreadable metadata is
/// ignored (the image is a plain TIFF).
pub(super) fn apply(s: &mut Series, tiff: &TiffFile) -> Result<()> {
    let Some(e) = tiff.ifds[0].entry(TAG_METADATA).filter(|e| e.count <= MAX_TAG) else { return Ok(()) };
    let Ok(raw) = tiff.raw(e) else { return Ok(()) };
    let Some(Ok(meta)) = entry(&raw, "MetadataTiffV1_0").map(decode_lv) else { return Ok(()) };
    let Some(pic) = meta.get("SLxPictureMetadata") else { return Ok(()) };
    let calibrated = !matches!(pic.get("bCalibrated"), Some(Value::Bool(false)));
    if let Some(um) = pic.float("dCalibration").filter(|v| calibrated && v.is_finite() && *v > 0.0) {
        let aspect = pic.float("dAspect").filter(|a| a.is_finite() && *a > 0.0).unwrap_or(1.0);
        s.meta.physical_size.x_nm = Some(um * 1e3);
        s.meta.physical_size.y_nm = Some(um * aspect * 1e3);
    }
    let objective = ObjectiveMeta {
        model: pic.text("wsObjectiveName"),
        magnification: pic.float("dObjectiveMag").filter(|m| *m > 0.0),
        numerical_aperture: pic.float("dObjectiveNA").filter(|v| *v > 0.0),
        ..Default::default()
    };
    if objective != ObjectiveMeta::default() {
        s.meta.objective = Some(objective);
    }
    let um = |k: &str| pic.float(k).filter(|v| v.is_finite()).map(|v| v * 1e3);
    if pic.get("dXPos").is_some() {
        s.meta.stage_position = Some(Position { x_nm: um("dXPos"), y_nm: um("dYPos"), z_nm: um("dZPos") });
    }
    if let Some(date) = pic.float("dTimeAbsolute").and_then(crate::formats::nd2::julian_to_iso) {
        s.meta.acquisition_date = Some(date);
    }
    // One plane description per channel (`a0`, `a1`, ...).
    if let Some(planes) = pic.get("sPicturePlanes").and_then(|p| p.get("sPlaneNew")) {
        let names: Vec<Option<String>> = planes.items().iter().map(|(_, p)| p.text("sDescription")).collect();
        if names.len() == s.meta.channels.len() {
            for (c, n) in s.meta.channels.iter_mut().zip(names) {
                c.name = n;
            }
        }
    }
    for (key, label) in [("wsObjectiveName", "Objective"), ("dObjectiveNA", "Objective NA"), ("dCalibration", "Calibration (um)"), ("dZoom", "Zoom")] {
        let value = match pic.get(key) {
            Some(Value::Text(t)) if !t.is_empty() => t.clone(),
            Some(Value::Float(f)) => f.to_string(),
            _ => continue,
        };
        s.original.push((label.into(), value));
    }
    Ok(())
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_entries() {
        let name: Vec<u8> = "Abc".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let mut d = Vec::new();
        d.extend(1u32.to_le_bytes());
        d.extend(3u32.to_le_bytes());
        d.extend(&name);
        for v in [1u32, 0, 2, 30] {
            d.extend(v.to_le_bytes());
        }
        d.resize(30, 0);
        d.extend([7, 8]);
        assert_eq!(entry(&d, "Abc"), Some(&[7u8, 8][..]));
        assert_eq!(entry(&d, "Xyz"), None);
        assert_eq!(entry(&d[..10], "Abc"), None);
    }
}

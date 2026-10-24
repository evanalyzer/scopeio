//! # imagej
//!
//! ImageJ hyperstack metadata stored in the TIFF ImageDescription, a list of
//! `key=value` lines starting with `ImageJ=`.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use super::ifd::{TiffFile, tag};
use super::{PlaneRef, Series, Slot};
use crate::Result;

/// Values of an ImageJ description.
#[derive(Debug, Default, PartialEq)]
pub(super) struct ImageJ {
    pub images: u32,
    pub channels: u32,
    pub slices: u32,
    pub frames: u32,
    pub unit: Option<String>,
    pub spacing: Option<f64>,
    pub finterval: Option<f64>,
    pub pairs: Vec<(String, String)>,
}

/// Parses an ImageJ description; `None` if `desc` is not one.
pub(super) fn parse(desc: &str) -> Option<ImageJ> {
    if !desc.starts_with("ImageJ=") {
        return None;
    }
    let mut ij = ImageJ { images: 1, channels: 1, slices: 1, frames: 1, ..Default::default() };
    for line in desc.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim();
        let n = || v.parse::<u32>().ok().filter(|&n| n > 0);
        match k.trim() {
            "images" => ij.images = n().unwrap_or(1),
            "channels" => ij.channels = n().unwrap_or(1),
            "slices" => ij.slices = n().unwrap_or(1),
            "frames" => ij.frames = n().unwrap_or(1),
            "unit" => ij.unit = Some(v.replace("\\u00B5", "µ")),
            "spacing" => ij.spacing = v.parse().ok(),
            "finterval" => ij.finterval = v.parse().ok(),
            _ => {}
        }
        ij.pairs.push((format!("ImageJ {}", k.trim()), v.to_string()));
    }
    Some(ij)
}

/// Nanometres per ImageJ length unit.
fn unit_nm(unit: &str) -> Option<f64> {
    Some(match unit {
        "micron" | "microns" | "µm" | "um" | "μm" => 1e3,
        "nm" | "nanometer" | "nanometre" => 1.0,
        "mm" | "millimeter" | "millimetre" => 1e6,
        "cm" | "centimeter" | "centimetre" => 1e7,
        "m" | "meter" | "metre" => 1e9,
        "inch" | "in" => 25.4e6,
        "Å" | "A" | "angstrom" => 0.1,
        _ => return None,
    })
}

/// Applies ImageJ dimensions and calibration to a single plain series.
pub(super) fn apply(ij: &ImageJ, s: &mut Series, tiff: &TiffFile) -> Result<()> {
    let (c, z, t) = (ij.channels, ij.slices, ij.frames);
    let total = c as usize * z as usize * t as usize;
    let gray = s.meta.samples_per_pixel == 1 && s.meta.size_c == 1;
    let pages = s.levels[0].len();
    if gray && pages == 1 && total > 1 && ij.images as usize == total && s.levels.len() == 1 {
        // Large ImageJ files: one IFD, the other planes follow contiguously.
        if let Some(Slot::Ready(first)) = &s.levels[0][0]
            && let Some(plane_bytes) = first.image.contiguous_size(tiff)? {
                let file = first.file;
                let end = first.image.data_start(tiff)?.saturating_add(plane_bytes.saturating_mul(total as u64));
                if end <= tiff.len {
                    let images: Vec<_> =
                        (1..total as u64).map(|k| Some(Slot::ready(file, first.image.shifted(k * plane_bytes)))).collect();
                    s.levels[0].extend(images);
                }
            }
    }
    if gray && s.levels[0].len() == total {
        s.meta.size_c = c;
        s.meta.size_z = z;
        s.meta.size_t = t;
        // ImageJ stores planes channel first, then slice, then frame, which
        // is the plane index order of the series.
        s.planes = (0..total).map(|page| Some(PlaneRef { page, samples: super::image::Samples::One(0) })).collect();
        s.meta.channels = vec![s.meta.channels[0].clone(); c as usize];
    }
    let ifd = &tiff.ifds[0];
    if let Some(nm) = ij.unit.as_deref().and_then(unit_nm) {
        let res = |t: u16| -> Result<Option<f64>> {
            Ok(tiff.floats(ifd, t)?.and_then(|v| v.first().copied()).filter(|r| r.is_finite() && *r > 0.0))
        };
        s.meta.physical_size.x_nm = res(tag::X_RESOLUTION)?.map(|r| nm / r);
        s.meta.physical_size.y_nm = res(tag::Y_RESOLUTION)?.map(|r| nm / r);
        s.meta.physical_size.z_nm = ij.spacing.filter(|v| *v > 0.0).map(|v| v * nm);
    }
    s.meta.time_increment_s = ij.finterval.filter(|v| *v > 0.0);
    Ok(())
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hyperstack_description() {
        let ij = parse("ImageJ=1.53t\nimages=46\nchannels=2\nframes=23\nhyperstack=true\nunit=\\u00B5m\nspacing=0.5\n").unwrap();
        assert_eq!((ij.images, ij.channels, ij.slices, ij.frames), (46, 2, 1, 23));
        assert_eq!(ij.unit.as_deref(), Some("µm"));
        assert_eq!(ij.spacing, Some(0.5));
        assert!(parse("<OME/>").is_none());
    }
}

//! # color_model
//!
//! Meaning and order of the samples within one pixel.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::PixelType;

/// Interpretation of the interleaved samples of one pixel.
///
/// Output samples always follow the order in the variant name (R, G, B, A),
/// with straight (not premultiplied) alpha, whatever order the file uses.
/// Fluorescence channels are not colour samples: they are separate planes
/// addressed by [`Plane::c`](crate::Plane::c), each with [`ColorModel::Gray`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ColorModel {
    /// One intensity sample.
    #[default]
    Gray,
    /// Intensity followed by alpha.
    GrayAlpha,
    /// Red, green, blue, e.g. brightfield or whole-slide images.
    Rgb,
    /// Red, green, blue, alpha.
    Rgba,
}

impl ColorModel {
    /// Number of interleaved samples per pixel.
    pub fn samples(self) -> usize {
        match self {
            ColorModel::Gray => 1,
            ColorModel::GrayAlpha => 2,
            ColorModel::Rgb => 3,
            ColorModel::Rgba => 4,
        }
    }

    /// Returns `true` if the last sample is alpha.
    pub fn has_alpha(self) -> bool {
        matches!(self, ColorModel::GrayAlpha | ColorModel::Rgba)
    }

    /// Swaps blue and red samples in place, turning BGR(A) data into RGB(A).
    ///
    /// Readers call this on decoded data from files storing blue first. Does
    /// nothing for models without colour.
    ///
    /// # Arguments
    ///
    /// * `buf` - Interleaved pixel data of this model.
    /// * `pixel_type` - Type of the samples in `buf`.
    pub fn bgr_to_rgb(self, buf: &mut [u8], pixel_type: PixelType) {
        if !matches!(self, ColorModel::Rgb | ColorModel::Rgba) {
            return;
        }
        let n = pixel_type.bytes();
        for pixel in buf.chunks_exact_mut(self.samples() * n) {
            let (b, rest) = pixel.split_at_mut(n);
            b.swap_with_slice(&mut rest[n..2 * n]);
        }
    }

    /// Interleaves planar sample data (all R, then all G, ...) into `out`.
    ///
    /// Readers call this for files storing each sample in its own plane.
    ///
    /// # Arguments
    ///
    /// * `planes` - One slice per sample, in output order, of equal length.
    /// * `pixel_type` - Type of the samples.
    /// * `out` - Output buffer of at least the total length of `planes`.
    ///
    /// # Panics
    ///
    /// Panics if the number of planes differs from [`ColorModel::samples`],
    /// the planes differ in length or `out` is too small.
    pub fn interleave(self, planes: &[&[u8]], pixel_type: PixelType, out: &mut [u8]) {
        assert_eq!(planes.len(), self.samples(), "one plane per sample required");
        let plane_len = planes[0].len();
        assert!(planes.iter().all(|p| p.len() == plane_len), "planes differ in length");
        assert!(out.len() >= plane_len * planes.len(), "output buffer too small");
        let n = pixel_type.bytes();
        for (s, plane) in planes.iter().enumerate() {
            for (dst, src) in out.chunks_exact_mut(planes.len() * n).zip(plane.chunks_exact(n)) {
                dst[s * n..(s + 1) * n].copy_from_slice(src);
            }
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_and_alpha() {
        assert_eq!(ColorModel::Gray.samples(), 1);
        assert_eq!(ColorModel::GrayAlpha.samples(), 2);
        assert_eq!(ColorModel::Rgb.samples(), 3);
        assert_eq!(ColorModel::Rgba.samples(), 4);
        assert!(ColorModel::Rgba.has_alpha() && !ColorModel::Rgb.has_alpha());
    }

    #[test]
    fn bgr_to_rgb_swaps_u8_and_keeps_alpha() {
        let mut buf = [1, 2, 3, 9, 4, 5, 6, 9];
        ColorModel::Rgba.bgr_to_rgb(&mut buf, PixelType::U8);
        assert_eq!(buf, [3, 2, 1, 9, 6, 5, 4, 9]);
    }

    #[test]
    fn bgr_to_rgb_swaps_whole_u16_samples() {
        let mut buf = [1, 1, 2, 2, 3, 3];
        ColorModel::Rgb.bgr_to_rgb(&mut buf, PixelType::U16);
        assert_eq!(buf, [3, 3, 2, 2, 1, 1]);

        let mut gray = [1, 2, 3];
        ColorModel::Gray.bgr_to_rgb(&mut gray, PixelType::U8);
        assert_eq!(gray, [1, 2, 3]);
    }

    #[test]
    fn interleave_planar_rgb() {
        let (r, g, b) = ([1, 2], [3, 4], [5, 6]);
        let mut out = [0u8; 6];
        ColorModel::Rgb.interleave(&[&r, &g, &b], PixelType::U8, &mut out);
        assert_eq!(out, [1, 3, 5, 2, 4, 6]);
    }

    #[test]
    fn interleave_planar_u16() {
        let (r, g) = ([1, 1, 2, 2], [3, 3, 4, 4]);
        let mut out = [0u8; 8];
        ColorModel::GrayAlpha.interleave(&[&r, &g], PixelType::U16, &mut out);
        assert_eq!(out, [1, 1, 3, 3, 2, 2, 4, 4]);
    }
}

//! # meta
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{ChannelMeta, Error, ObjectiveMeta, PixelType, Plane, Position, Region, Result, Roi};

/// Physical size of one pixel (x, y) and the z-step, in nanometres.
/// Fields are `None` if unknown.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysicalSize {
    pub x_nm: Option<f64>,
    pub y_nm: Option<f64>,
    pub z_nm: Option<f64>,
}

/// Dimensions and storage layout of one resolution level.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct LevelMeta {
    pub size_x: u64,
    pub size_y: u64,
    /// Native tile size; `None` if untiled (the whole plane is one tile).
    pub tile: Option<(u32, u32)>,
    /// Downsample factor relative to level 0 (1.0 for level 0).
    pub downsample: f64,
}

impl LevelMeta {
    /// Native tile size, the whole level if untiled.
    pub fn tile_size(&self) -> (u64, u64) {
        self.tile.map_or((self.size_x, self.size_y), |(w, h)| (w as u64, h as u64))
    }

    /// Number of tiles across and down.
    pub fn tile_counts(&self) -> (u32, u32) {
        let (w, h) = self.tile_size();
        (self.size_x.div_ceil(w) as u32, self.size_y.div_ceil(h) as u32)
    }

    /// Region of tile (`tile_x`, `tile_y`), cropped at the right and bottom
    /// edges.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the tile does not exist.
    pub fn tile_rect(&self, tile_x: u32, tile_y: u32) -> Result<Region> {
        let (across, down) = self.tile_counts();
        if tile_x >= across || tile_y >= down {
            return Err(Error::OutOfRange(format!("tile ({tile_x}, {tile_y}) of {across}x{down}")));
        }
        let (w, h) = self.tile_size();
        let (x, y) = (tile_x as u64 * w, tile_y as u64 * h);
        Ok(Region::new(x, y, w.min(self.size_x - x), h.min(self.size_y - y)))
    }
}

/// Core metadata of one series, parsed at open.
///
/// Units are fixed: lengths and wavelengths in nanometres, times in seconds.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct SeriesMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Acquisition start as ISO 8601 string.
    pub acquisition_date: Option<String>,
    /// Number of channels; a colour image is one channel.
    pub size_c: u32,
    pub size_z: u32,
    pub size_t: u32,
    pub pixel_type: PixelType,
    /// Bits actually used per sample, e.g. 12 for 12-bit data stored as u16.
    pub significant_bits: Option<u8>,
    /// Samples per pixel of every channel: 1 gray, 3 RGB, 4 RGBA.
    pub samples_per_pixel: u8,
    /// Resolution levels, level 0 first. Never empty.
    pub levels: Vec<LevelMeta>,
    pub physical_size: PhysicalSize,
    /// Nominal interval between time points.
    pub time_increment_s: Option<f64>,
    /// One entry per channel, `len() == size_c`.
    pub channels: Vec<ChannelMeta>,
    pub objective: Option<ObjectiveMeta>,
    pub stage_position: Option<Position>,
    pub rois: Vec<Roi>,
}

impl SeriesMeta {
    /// Checks that `plane` lies within the series dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if any coordinate is too large.
    pub fn check_plane(&self, plane: Plane) -> Result<()> {
        if plane.c >= self.size_c || plane.z >= self.size_z || plane.t >= self.size_t {
            return Err(Error::OutOfRange(format!(
                "plane c={} z={} t={} of {}x{}x{}",
                plane.c, plane.z, plane.t, self.size_c, self.size_z, self.size_t
            )));
        }
        Ok(())
    }

    /// Metadata of `level`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the level does not exist.
    pub fn level(&self, level: usize) -> Result<&LevelMeta> {
        self.levels
            .get(level)
            .ok_or_else(|| Error::OutOfRange(format!("level {level} of {}", self.levels.len())))
    }
}

/// Format-specific metadata not mapped to [`SeriesMeta`], parsed on request.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct OriginalMeta {
    /// Key-value pairs in file order.
    pub pairs: Vec<(String, String)>,
    /// Embedded documents by name, e.g. `("OME-XML", xml)`.
    pub documents: Vec<(String, String)>,
}

/// Images stored besides the series, e.g. the label of a slide.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssociatedKind {
    Label,
    Macro,
    Thumbnail,
    Other(String),
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_rect_crops_edges() {
        let l = LevelMeta { size_x: 300, size_y: 200, tile: Some((128, 128)), downsample: 1.0 };
        assert_eq!(l.tile_counts(), (3, 2));
        assert_eq!(l.tile_rect(2, 1).unwrap(), Region::new(256, 128, 44, 72));
        assert!(l.tile_rect(3, 0).is_err());
        let untiled = LevelMeta { size_x: 30, size_y: 20, tile: None, downsample: 1.0 };
        assert_eq!(untiled.tile_rect(0, 0).unwrap(), Region::new(0, 0, 30, 20));
    }

    #[test]
    fn check_plane_rejects_out_of_range() {
        let m = SeriesMeta { size_c: 2, size_z: 3, size_t: 1, ..Default::default() };
        assert!(m.check_plane(Plane::new(1, 2, 0)).is_ok());
        assert!(matches!(m.check_plane(Plane::new(2, 0, 0)), Err(Error::OutOfRange(_))));
    }
}

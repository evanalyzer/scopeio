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
    pub width: u64,
    pub height: u64,
    /// Native storage tile width; equal to `width` for untiled data.
    pub tile_width: u32,
    /// Native storage tile height; equal to the strip height for strip-based data.
    pub tile_height: u32,
    /// Downsample factor relative to level 0 (1.0 for level 0).
    pub downsample: f64,
}

/// Metadata of one series.
///
/// Units are fixed: lengths and wavelengths in nanometres, times in seconds.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct SeriesMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Acquisition start as ISO 8601 string.
    pub acquisition_date: Option<String>,
    pub size_c: u32,
    pub size_z: u32,
    pub size_t: u32,
    pub pixel_type: PixelType,
    /// Bits actually used per sample, e.g. 12 for 12-bit data stored as u16.
    pub significant_bits: Option<u8>,
    /// Interleaved samples per pixel (3 for RGB brightfield, 1 otherwise).
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
    /// Format-specific key-value metadata not mapped to a field above,
    /// in file order.
    pub original: Vec<(String, String)>,
}

impl SeriesMeta {
    /// Number of bytes needed to hold `region`.
    pub fn buffer_size(&self, region: Region) -> usize {
        region.width as usize
            * region.height as usize
            * self.samples_per_pixel as usize
            * self.pixel_type.bytes()
    }

    /// Checks that `plane` lies within the series dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PlaneOutOfRange`] if any coordinate is too large.
    pub fn check_plane(&self, plane: Plane) -> Result<()> {
        if plane.c >= self.size_c || plane.z >= self.size_z || plane.t >= self.size_t {
            return Err(Error::PlaneOutOfRange(plane));
        }
        Ok(())
    }

    /// Validates a read request against this series.
    ///
    /// Readers call this at the start of [`ImageReader::read_region_into`](crate::ImageReader::read_region_into).
    ///
    /// # Arguments
    ///
    /// * `level` - Resolution level index.
    /// * `plane` - Plane address.
    /// * `region` - Requested region in level coordinates.
    /// * `buf_len` - Length of the output buffer.
    ///
    /// # Returns
    ///
    /// The metadata of the requested level.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if any index is out of range, the region is empty or
    /// out of bounds, or the buffer is too small.
    pub fn check_request(
        &self,
        level: usize,
        plane: Plane,
        region: Region,
        buf_len: usize,
    ) -> Result<&LevelMeta> {
        let level_meta = self.levels.get(level).ok_or(Error::LevelOutOfRange {
            level,
            count: self.levels.len(),
        })?;
        self.check_plane(plane)?;
        let fits = |pos: u64, len: u32, max: u64| {
            len > 0 && pos.checked_add(len as u64).is_some_and(|end| end <= max)
        };
        if !fits(region.x, region.width, level_meta.width)
            || !fits(region.y, region.height, level_meta.height)
        {
            return Err(Error::RegionOutOfBounds(region));
        }
        let needed = self.buffer_size(region);
        if buf_len < needed {
            return Err(Error::BufferTooSmall { needed, got: buf_len });
        }
        Ok(level_meta)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> SeriesMeta {
        SeriesMeta {
            size_c: 2,
            size_z: 3,
            size_t: 1,
            pixel_type: PixelType::U16,
            samples_per_pixel: 1,
            levels: vec![
                LevelMeta { width: 1000, height: 800, tile_width: 256, tile_height: 256, downsample: 1.0 },
                LevelMeta { width: 500, height: 400, tile_width: 256, tile_height: 256, downsample: 2.0 },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn buffer_size_accounts_for_type_and_samples() {
        let mut m = meta();
        assert_eq!(m.buffer_size(Region::new(0, 0, 10, 5)), 100);
        m.samples_per_pixel = 3;
        m.pixel_type = PixelType::U8;
        assert_eq!(m.buffer_size(Region::new(0, 0, 10, 5)), 150);
    }

    #[test]
    fn check_request_accepts_valid_request() {
        let m = meta();
        let level = m
            .check_request(1, Plane::new(1, 2, 0), Region::new(244, 144, 256, 256), 256 * 256 * 2)
            .unwrap();
        assert_eq!(level.width, 500);
    }

    #[test]
    fn check_request_rejects_invalid_requests() {
        let m = meta();
        let ok_region = Region::new(0, 0, 10, 10);
        let ok_len = 200;
        assert!(matches!(
            m.check_request(2, Plane::default(), ok_region, ok_len),
            Err(Error::LevelOutOfRange { level: 2, count: 2 })
        ));
        assert!(matches!(
            m.check_request(0, Plane::new(2, 0, 0), ok_region, ok_len),
            Err(Error::PlaneOutOfRange(_))
        ));
        assert!(matches!(
            m.check_request(1, Plane::default(), Region::new(495, 0, 10, 10), ok_len),
            Err(Error::RegionOutOfBounds(_))
        ));
        assert!(matches!(
            m.check_request(0, Plane::default(), Region::new(0, 0, 0, 10), ok_len),
            Err(Error::RegionOutOfBounds(_))
        ));
        assert!(matches!(
            m.check_request(0, Plane::default(), Region::new(u64::MAX, 0, 10, 10), ok_len),
            Err(Error::RegionOutOfBounds(_))
        ));
        assert!(matches!(
            m.check_request(0, Plane::default(), ok_region, 199),
            Err(Error::BufferTooSmall { needed: 200, got: 199 })
        ));
    }
}

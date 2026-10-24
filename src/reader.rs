//! # reader
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Plane, PlaneMeta, PlateMeta, Region, Result, SeriesMeta};

/// Common interface implemented by all format readers.
///
/// Pixel data is written row-major, with samples interleaved, in native
/// byte order. Readers take `&self` so that one reader can serve parallel
/// reads from several threads.
pub trait ImageReader: Send + Sync {
    /// Number of series in the file.
    fn series_count(&self) -> usize;

    /// Metadata of one series.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SeriesOutOfRange`](crate::Error::SeriesOutOfRange) if `series` is not smaller than
    /// [`ImageReader::series_count`].
    fn series_meta(&self, series: usize) -> Result<&SeriesMeta>;

    /// Acquisition data of one plane (time stamp, exposure, stage position).
    ///
    /// The default implementation validates the request and returns empty
    /// metadata; readers override it if the format stores per-plane data.
    ///
    /// # Errors
    ///
    /// Returns [`Error`](crate::Error) if `series` or `plane` is out of range.
    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        self.series_meta(series)?.check_plane(plane)?;
        Ok(PlaneMeta::default())
    }

    /// Plates in the file. Empty unless the file comes from a plate screen.
    fn plates(&self) -> &[PlateMeta] {
        &[]
    }

    /// Decodes a region into a caller-provided buffer.
    ///
    /// # Arguments
    ///
    /// * `series` - Series index.
    /// * `level` - Resolution level index, 0 is full resolution.
    /// * `plane` - Plane address.
    /// * `region` - Region in level coordinates.
    /// * `buf` - Output buffer of at least [`SeriesMeta::buffer_size`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error`](crate::Error) if the request is invalid or decoding fails.
    fn read_region_into(
        &self,
        series: usize,
        level: usize,
        plane: Plane,
        region: Region,
        buf: &mut [u8],
    ) -> Result<()>;

    /// Decodes a region into a newly allocated buffer.
    ///
    /// # Errors
    ///
    /// Same as [`ImageReader::read_region_into`].
    fn read_region(
        &self,
        series: usize,
        level: usize,
        plane: Plane,
        region: Region,
    ) -> Result<Vec<u8>> {
        let size = self.series_meta(series)?.buffer_size(region);
        let mut buf = vec![0u8; size];
        self.read_region_into(series, level, plane, region, &mut buf)?;
        Ok(buf)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, LevelMeta, PixelType};

    /// Reader returning a constant pixel value, to test the default methods.
    struct Constant {
        meta: SeriesMeta,
    }

    impl Constant {
        fn new() -> Self {
            Self {
                meta: SeriesMeta {
                    size_c: 1,
                    size_z: 2,
                    size_t: 1,
                    pixel_type: PixelType::U16,
                    samples_per_pixel: 1,
                    levels: vec![LevelMeta { width: 8, height: 4, tile_width: 8, tile_height: 4, downsample: 1.0 }],
                    ..Default::default()
                },
            }
        }
    }

    impl ImageReader for Constant {
        fn series_count(&self) -> usize {
            1
        }

        fn series_meta(&self, series: usize) -> Result<&SeriesMeta> {
            match series {
                0 => Ok(&self.meta),
                _ => Err(Error::SeriesOutOfRange { series, count: 1 }),
            }
        }

        fn read_region_into(
            &self,
            series: usize,
            level: usize,
            plane: Plane,
            region: Region,
            buf: &mut [u8],
        ) -> Result<()> {
            let meta = self.series_meta(series)?;
            meta.check_request(level, plane, region, buf.len())?;
            buf[..meta.buffer_size(region)].fill(7);
            Ok(())
        }
    }

    #[test]
    fn read_region_allocates_exact_size() {
        let buf = Constant::new().read_region(0, 0, Plane::new(0, 1, 0), Region::new(2, 1, 3, 2)).unwrap();
        assert_eq!(buf, vec![7u8; 12]);
    }

    #[test]
    fn plane_meta_default_validates_request() {
        let r = Constant::new();
        assert_eq!(r.plane_meta(0, Plane::new(0, 1, 0)).unwrap(), PlaneMeta::default());
        assert!(matches!(r.plane_meta(0, Plane::new(0, 2, 0)), Err(Error::PlaneOutOfRange(_))));
        assert!(matches!(r.plane_meta(1, Plane::default()), Err(Error::SeriesOutOfRange { .. })));
        assert!(r.plates().is_empty());
    }
}

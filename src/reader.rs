//! # reader
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Plane, Region, Result, SeriesMeta};

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

//! # reader
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::types::as_bytes_mut;
use crate::{Error, PixelBuf, PixelType, Plane, PlaneMeta, PlateMeta, Region, Result, Sample, SeriesMeta};

/// Common interface implemented by all format readers.
///
/// Pixel data is written row-major, with samples interleaved (height x
/// width x samples per pixel), always in native byte order: readers convert
/// from the file byte order with [`ByteOrder::to_native`](crate::ByteOrder::to_native).
/// Colour samples are always R, G, B(, A), interleaved, with straight alpha:
/// readers normalise BGR and planar storage with [`ColorModel::bgr_to_rgb`](crate::ColorModel::bgr_to_rgb)
/// and [`ColorModel::interleave`](crate::ColorModel::interleave).
/// Readers take `&self` so that one reader can serve parallel reads from
/// several threads.
///
/// Implementors only provide [`ImageReader::read_region_into`]; the typed
/// methods of [`ImageReaderExt`] build on it without copying.
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

/// Typed pixel access, implemented for every [`ImageReader`] including
/// `dyn ImageReader`.
///
/// Buffers hold samples row-major and interleaved (height x width x samples
/// per pixel) in native byte order, so they can be handed to tensor
/// libraries as is. [`SeriesMeta::shape`] gives the matching tensor shape.
///
/// # Zero-copy use with tensor libraries
///
/// Either hand over the returned `Vec<T>`, or decode into memory the tensor
/// already owns. With kornia-rs:
///
/// ```ignore
/// use kornia_image::{Image, ImageSize};
///
/// // Vec<T> moves into the image without copying.
/// let data = reader.read_region_as::<u8>(0, 0, plane, region)?;
/// let size = ImageSize { width: region.width as usize, height: region.height as usize };
/// let rgb = Image::<u8, 3>::new(size, data)?;
///
/// // Reuse one image for a loop over equally sized tiles.
/// let mut tile = Image::<u8, 3>::from_size_val(size, 0)?;
/// reader.read_region_into_as(0, 0, plane, region, tile.as_slice_mut())?;
/// ```
pub trait ImageReaderExt: ImageReader {
    /// Decodes a region into a caller-provided typed buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - Output buffer of at least width x height x samples per pixel
    ///   elements. Other arguments as in [`ImageReader::read_region_into`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::PixelTypeMismatch`] if `T` does not match
    /// [`SeriesMeta::pixel_type`], otherwise same as
    /// [`ImageReader::read_region_into`].
    fn read_region_into_as<T: Sample>(
        &self,
        series: usize,
        level: usize,
        plane: Plane,
        region: Region,
        buf: &mut [T],
    ) -> Result<()> {
        let stored = self.series_meta(series)?.pixel_type;
        if stored != T::TYPE {
            return Err(Error::PixelTypeMismatch { stored, requested: T::TYPE });
        }
        self.read_region_into(series, level, plane, region, as_bytes_mut(buf))
    }

    /// Decodes a region into a newly allocated typed buffer.
    ///
    /// # Errors
    ///
    /// Same as [`ImageReaderExt::read_region_into_as`].
    fn read_region_as<T: Sample>(
        &self,
        series: usize,
        level: usize,
        plane: Plane,
        region: Region,
    ) -> Result<Vec<T>> {
        let meta = self.series_meta(series)?;
        let mut buf = vec![T::default(); meta.buffer_size(region) / meta.pixel_type.bytes()];
        self.read_region_into_as(series, level, plane, region, &mut buf)?;
        Ok(buf)
    }

    /// Decodes a region into a buffer of the series' own sample type.
    ///
    /// # Errors
    ///
    /// Same as [`ImageReader::read_region_into`].
    fn read_region_buf(
        &self,
        series: usize,
        level: usize,
        plane: Plane,
        region: Region,
    ) -> Result<PixelBuf> {
        macro_rules! read {
            ($variant:ident) => {
                PixelBuf::$variant(self.read_region_as(series, level, plane, region)?)
            };
        }
        Ok(match self.series_meta(series)?.pixel_type {
            PixelType::U8 => read!(U8),
            PixelType::I8 => read!(I8),
            PixelType::U16 => read!(U16),
            PixelType::I16 => read!(I16),
            PixelType::U32 => read!(U32),
            PixelType::I32 => read!(I32),
            PixelType::F32 => read!(F32),
            PixelType::F64 => read!(F64),
        })
    }

}

impl<R: ImageReader + ?Sized> ImageReaderExt for R {}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorModel, LevelMeta};

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
                    color: ColorModel::Gray,
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

    #[test]
    fn read_region_as_returns_typed_samples() {
        let r = Constant::new();
        let px: Vec<u16> = r.read_region_as(0, 0, Plane::default(), Region::new(0, 0, 3, 2)).unwrap();
        assert_eq!(px, vec![0x0707; 6]);
    }

    #[test]
    fn read_region_as_rejects_wrong_type() {
        let r = Constant::new();
        assert!(matches!(
            r.read_region_as::<f32>(0, 0, Plane::default(), Region::new(0, 0, 3, 2)),
            Err(Error::PixelTypeMismatch { stored: PixelType::U16, requested: PixelType::F32 })
        ));
    }

    #[test]
    fn typed_reads_work_through_dyn() {
        let r: Box<dyn ImageReader> = Box::new(Constant::new());
        let buf = r.read_region_buf(0, 0, Plane::default(), Region::new(0, 0, 2, 2)).unwrap();
        assert_eq!(buf, PixelBuf::U16(vec![0x0707; 4]));
        assert_eq!(buf.pixel_type(), PixelType::U16);
    }

}

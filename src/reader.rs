//! # reader
//!
//! The public [`ImageReader`] trait and the engine implementing it on top of
//! the format backends.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::pixel::{Sink, output_type};
use crate::{
    AssociatedKind, ChannelSelect, ColorOrder, Error, FormatInfo, ImageBuf, Layout, OriginalMeta, OutputShape,
    OutputSpec, PixelBuf, PixelType, PixelViewMut, Plane, PlaneMeta, PlateMeta, Region, RegionRequest, Result,
    SeriesMeta, TileRequest,
};

/// Common interface of all format readers.
///
/// Handles are stateless and shareable: every method takes `&self`, so one
/// handle serves reads from many threads at once. Reads are positional and
/// keep no pixel cache.
pub trait ImageReader: Send + Sync {
    /// Format and reader version that produced the data.
    fn format(&self) -> &FormatInfo;

    /// Number of series (scenes, positions, wells) in the file.
    fn series_count(&self) -> usize;

    /// Core metadata of one series.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `series` does not exist.
    fn series_meta(&self, series: usize) -> Result<&SeriesMeta>;

    /// Acquisition data of one plane: time stamp, exposure, stage position.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `series` or `plane` does not exist.
    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        self.series_meta(series)?.check_plane(plane)?;
        Ok(PlaneMeta::default())
    }

    /// Format-specific metadata, parsed on every call and not retained.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `series` does not exist, or an I/O or
    /// parse error of the metadata.
    fn original_metadata(&self, series: usize) -> Result<OriginalMeta>;

    /// Plates in the file; empty unless the file comes from a plate screen.
    fn plates(&self) -> &[PlateMeta] {
        &[]
    }

    /// Kinds of the associated images (label, macro, thumbnail).
    fn associated_images(&self) -> &[AssociatedKind] {
        &[]
    }

    /// Decodes an associated image.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the file has no such image, or a
    /// decoding error.
    fn read_associated(&self, kind: &AssociatedKind, output: &OutputSpec) -> Result<ImageBuf>;

    /// Decodes exactly one native tile into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] for invalid indices, [`Error::TypeMismatch`]
    /// if `out` does not match the output shape of the tile, or a decoding
    /// error.
    fn read_tile_into(&self, req: &TileRequest, out: PixelViewMut<'_>) -> Result<()>;

    /// Decodes any region into `out`.
    ///
    /// # Errors
    ///
    /// Same as [`ImageReader::read_tile_into`].
    fn read_region_into(&self, req: &RegionRequest, out: PixelViewMut<'_>) -> Result<()>;

    /// Decodes a region into a newly allocated, contiguous, typed buffer.
    ///
    /// # Errors
    ///
    /// Same as [`ImageReader::read_region_into`].
    fn read_region(&self, req: &RegionRequest) -> Result<ImageBuf>;

    /// Size and element type of the output of `req`, to preallocate buffers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] for invalid indices.
    fn output_shape(&self, req: &RegionRequest) -> Result<OutputShape>;
}

/// Native image data of an associated image.
pub(crate) struct NativeImage {
    pub width: u64,
    pub height: u64,
    pub samples_per_pixel: u8,
    pub pixel_type: PixelType,
    /// Row-major native samples, colour ordered R, G, B(, A).
    pub data: Vec<u8>,
}

/// Decodes `region` with `read` into a native buffer.
/// Largest decoded associated image (label, macro, thumbnail).
const MAX_ASSOCIATED_BYTES: u128 = 1 << 30;

pub(crate) fn native_image(
    region: Region,
    spp: u8,
    pixel_type: PixelType,
    read: impl FnOnce(&mut Sink) -> Result<()>,
) -> Result<NativeImage> {
    let w = region.width as usize;
    // Associated images are small; larger sizes come from corrupt files.
    let len = (region.width as u128) * (region.height as u128) * spp as u128 * pixel_type.bytes() as u128;
    if len > MAX_ASSOCIATED_BYTES {
        return Err(Error::Corrupt(format!("associated image of {} x {} pixels", region.width, region.height)));
    }
    let mut data = Vec::new();
    data.try_reserve_exact(len as usize).map_err(|_| Error::OutOfRange(format!("associated image of {len} bytes cannot be allocated")))?;
    data.resize(len as usize, 0u8);
    let map: Vec<u8> = (0..spp).collect();
    let mut sink = Sink::new(
        &mut data,
        Layout::Hwc,
        w * spp as usize,
        0,
        spp as usize,
        0,
        &map,
        spp as usize,
        pixel_type,
        crate::OutputType::Native,
    );
    read(&mut sink)?;
    Ok(NativeImage { width: region.width, height: region.height, samples_per_pixel: spp, pixel_type, data })
}

/// What a format implements; the engine adds request handling and output
/// conversion.
pub(crate) trait Backend: Send + Sync + 'static {
    fn format(&self) -> &'static FormatInfo;

    fn series(&self) -> &[SeriesMeta];

    fn plane_meta(&self, _series: usize, _plane: Plane) -> Result<PlaneMeta> {
        Ok(PlaneMeta::default())
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta>;

    fn plates(&self) -> &[PlateMeta] {
        &[]
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &[]
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        Err(Error::OutOfRange(format!("no associated image {kind:?}")))
    }

    /// Delivers `region` of channel `plane.c` at `level` to `sink`, row by row.
    ///
    /// Called with validated indices and a region within the level.
    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()>;
}

/// [`ImageReader`] implementation shared by all formats.
pub(crate) struct Reader<B>(pub B);

/// Samples one source channel contributes, in output order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleMap {
    samples: [u8; 4],
    len: u8,
}

impl SampleMap {
    fn as_slice(&self) -> &[u8] {
        &self.samples[..self.len as usize]
    }
}

/// Source channels and the samples they contribute; built without
/// allocating, so reads of uncompressed data allocate nothing.
struct Plan<'r> {
    select: &'r ChannelSelect,
    map: SampleMap,
    out_channels: usize,
}

impl Plan<'_> {
    /// Source channels in output order, each with its samples.
    fn channels(&self) -> impl Iterator<Item = (u32, SampleMap)> + '_ {
        let list: &[u32] = match self.select {
            ChannelSelect::Channel(c) => std::slice::from_ref(c),
            ChannelSelect::Sample { channel, .. } => std::slice::from_ref(channel),
            ChannelSelect::Channels(list) => list,
        };
        list.iter().map(|&c| (c, self.map))
    }
}

/// Samples of one stored channel with `spp` samples, in output order.
fn channel_samples(spp: u8, spec: &OutputSpec) -> SampleMap {
    let (mut samples, mut len) = match (spp, spec.color_order) {
        (3 | 4, ColorOrder::Rgb) => ([0, 1, 2, 0], 3),
        (3 | 4, ColorOrder::Bgr) => ([2, 1, 0, 0], 3),
        // Gray, with or without alpha.
        _ => ([0; 4], 1),
    };
    if spec.keep_alpha && (spp == 4 || spp == 2) {
        samples[len as usize] = spp - 1;
        len += 1;
    }
    SampleMap { samples, len }
}

fn plan<'r>(meta: &SeriesMeta, select: &'r ChannelSelect, spec: &OutputSpec) -> Result<Plan<'r>> {
    let check = |c: u32| {
        if c >= meta.size_c {
            return Err(Error::OutOfRange(format!("channel {c} of {}", meta.size_c)));
        }
        Ok(())
    };
    let spp = meta.samples_per_pixel.max(1);
    let (map, count) = match select {
        ChannelSelect::Channel(c) => {
            check(*c)?;
            (channel_samples(spp, spec), 1)
        }
        ChannelSelect::Sample { channel, sample } => {
            check(*channel)?;
            if *sample >= spp {
                return Err(Error::OutOfRange(format!("sample {sample} of {spp}")));
            }
            (SampleMap { samples: [*sample, 0, 0, 0], len: 1 }, 1)
        }
        ChannelSelect::Channels(list) if list.is_empty() => {
            return Err(Error::OutOfRange("empty channel list".into()));
        }
        ChannelSelect::Channels(list) => {
            for &c in list {
                check(c)?;
            }
            (channel_samples(spp, spec), list.len())
        }
    };
    Ok(Plan { select, map, out_channels: map.len as usize * count })
}

impl<B: Backend> Reader<B> {
    fn checked<'r>(&self, req: &'r RegionRequest) -> Result<(&SeriesMeta, Plan<'r>)> {
        let meta = self.series_meta(req.series)?;
        let level = meta.level(req.level)?;
        if !req.region.fits(level.size_x, level.size_y) {
            return Err(Error::OutOfRange(format!(
                "region {:?} outside level {} of {}x{}",
                req.region, req.level, level.size_x, level.size_y
            )));
        }
        meta.check_plane(Plane::new(0, req.z, req.t))?;
        Ok((meta, plan(meta, &req.channels, &req.output)?))
    }
}

impl<B: Backend> ImageReader for Reader<B> {
    fn format(&self) -> &FormatInfo {
        self.0.format()
    }

    fn series_count(&self) -> usize {
        self.0.series().len()
    }

    fn series_meta(&self, series: usize) -> Result<&SeriesMeta> {
        let all = self.0.series();
        all.get(series).ok_or_else(|| Error::OutOfRange(format!("series {series} of {}", all.len())))
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        self.series_meta(series)?.check_plane(plane)?;
        self.0.plane_meta(series, plane)
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        self.series_meta(series)?;
        self.0.original_metadata(series)
    }

    fn plates(&self) -> &[PlateMeta] {
        self.0.plates()
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        self.0.associated_images()
    }

    fn read_associated(&self, kind: &AssociatedKind, output: &OutputSpec) -> Result<ImageBuf> {
        let img = self.0.read_associated_native(kind)?;
        let samples = channel_samples(img.samples_per_pixel, output);
        let map = samples.as_slice();
        let (w, h) = (img.width as usize, img.height as usize);
        let pixel_type = output_type(output.dtype, img.pixel_type);
        let mut data = PixelBuf::try_zeros(pixel_type, w * h * map.len()).ok_or_else(|| Error::OutOfRange(format!("output of {} samples cannot be allocated", w * h * map.len())))?;
        {
            let mut slice = data.as_slice_mut();
            let (row_stride, plane_stride) = match output.layout {
                Layout::Hwc => (w * map.len(), 0),
                Layout::Chw => (w, w * h),
            };
            let spp = img.samples_per_pixel as usize;
            let mut sink = Sink::new(
                slice.bytes_mut(),
                output.layout,
                row_stride,
                plane_stride,
                map.len(),
                0,
                map,
                spp,
                img.pixel_type,
                output.dtype,
            );
            let row = w * spp * img.pixel_type.bytes();
            for (y, src) in img.data.chunks_exact(row).take(h).enumerate() {
                sink.put(y, 0, src);
            }
        }
        Ok(ImageBuf {
            data,
            width: img.width,
            height: img.height,
            channels: map.len() as u32,
            layout: output.layout,
            color_order: output.color_order,
        })
    }

    fn read_tile_into(&self, req: &TileRequest, out: PixelViewMut<'_>) -> Result<()> {
        let region = self.series_meta(req.series)?.level(req.level)?.tile_rect(req.tile_x, req.tile_y)?;
        let region_req = RegionRequest {
            series: req.series,
            level: req.level,
            z: req.z,
            t: req.t,
            channels: req.channels.clone(),
            region,
            output: req.output,
        };
        self.read_region_into(&region_req, out)
    }

    fn read_region_into(&self, req: &RegionRequest, mut out: PixelViewMut<'_>) -> Result<()> {
        let (meta, plan) = self.checked(req)?;
        let pixel_type = output_type(req.output.dtype, meta.pixel_type);
        if out.data.pixel_type() != pixel_type {
            return Err(Error::TypeMismatch(format!(
                "buffer holds {:?}, output is {pixel_type:?}",
                out.data.pixel_type()
            )));
        }
        let r = req.region;
        if (out.width, out.height, out.channels as usize) != (r.width, r.height, plan.out_channels)
            || out.layout != req.output.layout
        {
            return Err(Error::TypeMismatch(format!(
                "view is {}x{}x{} {:?}, output is {}x{}x{} {:?}",
                out.width, out.height, out.channels, out.layout, r.width, r.height, plan.out_channels, req.output.layout
            )));
        }
        out.check()?;
        let (row_stride, plane_stride) = (out.row_stride, out.plane_stride);
        let bytes = out.data.bytes_mut();
        let mut base = 0;
        for (c, map) in plan.channels() {
            let mut sink = Sink::new(
                &mut *bytes,
                req.output.layout,
                row_stride,
                plane_stride,
                plan.out_channels,
                base,
                map.as_slice(),
                meta.samples_per_pixel.max(1) as usize,
                meta.pixel_type,
                req.output.dtype,
            )
            .with_significant_bits(meta.significant_bits);
            self.0.read_native(req.series, req.level, Plane::new(c, req.z, req.t), r, &mut sink)?;
            base += map.len as usize;
        }
        Ok(())
    }

    fn read_region(&self, req: &RegionRequest) -> Result<ImageBuf> {
        let shape = self.output_shape(req)?;
        let mut data = PixelBuf::try_zeros(shape.pixel_type, shape.len()).ok_or_else(|| Error::OutOfRange(format!("output of {} samples cannot be allocated", shape.len())))?;
        let view = PixelViewMut::contiguous(data.as_slice_mut(), shape.width, shape.height, shape.channels, shape.layout);
        self.read_region_into(req, view)?;
        Ok(ImageBuf {
            data,
            width: shape.width,
            height: shape.height,
            channels: shape.channels,
            layout: shape.layout,
            color_order: req.output.color_order,
        })
    }

    fn output_shape(&self, req: &RegionRequest) -> Result<OutputShape> {
        let (meta, plan) = self.checked(req)?;
        Ok(OutputShape {
            width: req.region.width,
            height: req.region.height,
            channels: plan.out_channels as u32,
            layout: req.output.layout,
            pixel_type: output_type(req.output.dtype, meta.pixel_type),
        })
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_samples_follow_spec() {
        let rgb = OutputSpec::default();
        assert_eq!(channel_samples(1, &rgb).as_slice(), [0]);
        assert_eq!(channel_samples(3, &rgb).as_slice(), [0, 1, 2]);
        assert_eq!(channel_samples(4, &rgb).as_slice(), [0, 1, 2]);
        let bgra = OutputSpec { color_order: ColorOrder::Bgr, keep_alpha: true, ..Default::default() };
        assert_eq!(channel_samples(4, &bgra).as_slice(), [2, 1, 0, 3]);
        assert_eq!(channel_samples(2, &bgra).as_slice(), [0, 1]);
    }
}

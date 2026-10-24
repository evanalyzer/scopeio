//! # pixel
//!
//! Sample types and the single-pass writer that converts native samples
//! into the requested output layout and type.
//!
//! Backends deliver native rows: samples of one stored channel in native
//! byte order, colour samples ordered R, G, B(, A). A [`Sink`] writes them
//! into the caller's buffer, applying sample selection, colour order,
//! alpha dropping, Hwc/Chw layout and dtype conversion in the same pass.
//! When no conversion is needed it hands the backend the destination row
//! itself, so data is written exactly once.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Layout, OutputType, PixelType, Result, scratch};

mod sealed {
    pub trait Sealed {}
}

/// Rust type of one sample, implemented for all types in [`PixelType`].
///
/// The trait is sealed: only primitive numeric types without padding
/// implement it, which makes viewing a `[T]` as bytes sound.
pub trait Pixel: sealed::Sealed + Copy + Default + Send + Sync + 'static {
    /// Pixel type corresponding to `Self`.
    const TYPE: PixelType;
}

macro_rules! impl_pixel {
    ($($t:ty => $variant:ident),* $(,)?) => {
        $(
            impl sealed::Sealed for $t {}
            impl Pixel for $t {
                const TYPE: PixelType = PixelType::$variant;
            }
        )*
    };
}

impl_pixel!(u8 => U8, i8 => I8, u16 => U16, i16 => I16, u32 => U32, i32 => I32, f32 => F32, f64 => F64);

/// Views a sample slice as its raw bytes in native byte order.
pub(crate) fn as_bytes<T: Pixel>(samples: &[T]) -> &[u8] {
    // SAFETY: `Pixel` is sealed and only implemented for primitive numeric
    // types, which have no padding. The byte view covers exactly the memory
    // of `samples` and borrows it immutably.
    unsafe { std::slice::from_raw_parts(samples.as_ptr().cast::<u8>(), size_of_val(samples)) }
}

/// Views a sample slice as its raw bytes in native byte order.
pub(crate) fn as_bytes_mut<T: Pixel>(samples: &mut [T]) -> &mut [u8] {
    // SAFETY: as in `as_bytes`; in addition every bit pattern is a valid
    // value of these types, so writes through the byte view are sound.
    unsafe { std::slice::from_raw_parts_mut(samples.as_mut_ptr().cast::<u8>(), size_of_val(samples)) }
}

/// Reading a native sample as f32.
trait Elem: Copy {
    const SIZE: usize;
    /// Divisor of [`OutputType::F32Normalized`].
    const NORM: f32;
    fn read(b: &[u8]) -> f32;
}

macro_rules! impl_elem {
    ($($t:ty, $norm:expr);* $(;)?) => {
        $(
            impl Elem for $t {
                const SIZE: usize = size_of::<$t>();
                const NORM: f32 = $norm;
                fn read(b: &[u8]) -> f32 {
                    <$t>::from_ne_bytes(b[..size_of::<$t>()].try_into().expect("sample size")) as f32
                }
            }
        )*
    };
}

impl_elem!(u8, u8::MAX as f32; i8, i8::MAX as f32; u16, u16::MAX as f32; i16, i16::MAX as f32;
    u32, u32::MAX as f32; i32, i32::MAX as f32; f32, 1.0; f64, 1.0);

/// Output element type of a dtype for samples of `pixel_type`.
pub(crate) fn output_type(dtype: OutputType, pixel_type: PixelType) -> PixelType {
    match dtype {
        OutputType::Native => pixel_type,
        _ => PixelType::F32,
    }
}

/// Destination of one source channel within an output buffer.
pub(crate) struct Sink<'a> {
    out: &'a mut [u8],
    out_elem: usize,
    layout: Layout,
    row_stride: usize,
    plane_stride: usize,
    out_channels: usize,
    /// First output channel this source channel is written to.
    base: usize,
    /// Source sample of each output channel written, in output order.
    map: &'a [u8],
    src_spp: usize,
    src_type: PixelType,
    dtype: OutputType,
    direct: bool,
}

impl<'a> Sink<'a> {
    /// Creates a sink writing the samples `map` of a source channel with
    /// `src_spp` samples of `src_type` to output channels `base..`.
    ///
    /// `out` is the whole destination buffer as bytes; strides are in
    /// elements of the output type.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        out: &'a mut [u8],
        layout: Layout,
        row_stride: usize,
        plane_stride: usize,
        out_channels: usize,
        base: usize,
        map: &'a [u8],
        src_spp: usize,
        src_type: PixelType,
        dtype: OutputType,
    ) -> Self {
        let identity = map.iter().enumerate().all(|(i, &s)| i == s as usize);
        let direct = dtype == OutputType::Native
            && identity
            && match layout {
                Layout::Hwc => out_channels == src_spp && map.len() == src_spp,
                Layout::Chw => src_spp == 1 && map.len() == 1,
            };
        let out_elem = output_type(dtype, src_type).bytes();
        Self { out, out_elem, layout, row_stride, plane_stride, out_channels, base, map, src_spp, src_type, dtype, direct }
    }

    /// Bytes of one native source pixel.
    pub fn pixel_bytes(&self) -> usize {
        self.src_spp * self.src_type.bytes()
    }

    /// Writes `w` pixels starting at (`x`, `y`) of the requested region.
    ///
    /// `fill` receives a buffer of `w` native pixels to fill: the
    /// destination itself if no conversion is needed, otherwise a scratch
    /// row that is converted afterwards.
    pub fn write_with(&mut self, y: usize, x: usize, w: usize, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<()> {
        let len = w * self.pixel_bytes();
        if self.direct {
            let start = match self.layout {
                Layout::Hwc => (y * self.row_stride + x * self.out_channels) * self.out_elem,
                Layout::Chw => (self.base * self.plane_stride + y * self.row_stride + x) * self.out_elem,
            };
            return fill(&mut self.out[start..start + len]);
        }
        scratch::with(len, |buf| {
            fill(buf)?;
            self.put(y, x, buf);
            Ok(())
        })
    }

    /// Converts `src`, native pixels starting at (`x`, `y`), into the output.
    pub fn put(&mut self, y: usize, x: usize, src: &[u8]) {
        match self.dtype {
            OutputType::Native => self.put_native(y, x, src),
            _ => match self.src_type {
                PixelType::U8 => self.put_f32::<u8>(y, x, src),
                PixelType::I8 => self.put_f32::<i8>(y, x, src),
                PixelType::U16 => self.put_f32::<u16>(y, x, src),
                PixelType::I16 => self.put_f32::<i16>(y, x, src),
                PixelType::U32 => self.put_f32::<u32>(y, x, src),
                PixelType::I32 => self.put_f32::<i32>(y, x, src),
                PixelType::F32 => self.put_f32::<f32>(y, x, src),
                PixelType::F64 => self.put_f32::<f64>(y, x, src),
            },
        }
    }

    /// Element index of output channel `k` of pixel (`x`, `y`).
    fn index(&self, y: usize, x: usize, k: usize) -> usize {
        match self.layout {
            Layout::Hwc => y * self.row_stride + x * self.out_channels + self.base + k,
            Layout::Chw => (self.base + k) * self.plane_stride + y * self.row_stride + x,
        }
    }

    fn put_native(&mut self, y: usize, x: usize, src: &[u8]) {
        let n = self.src_type.bytes();
        let px = self.pixel_bytes();
        for (i, pixel) in src.chunks_exact(px).enumerate() {
            for (k, &s) in self.map.iter().enumerate() {
                let at = self.index(y, x + i, k) * n;
                let from = s as usize * n;
                self.out[at..at + n].copy_from_slice(&pixel[from..from + n]);
            }
        }
    }

    fn put_f32<S: Elem>(&mut self, y: usize, x: usize, src: &[u8]) {
        let (scale, offset) = match self.dtype {
            OutputType::F32Normalized => (1.0 / S::NORM, 0.0),
            OutputType::F32Range { min, max } => {
                let span = if max != min { max - min } else { 1.0 };
                (1.0 / span, -min / span)
            }
            _ => (1.0, 0.0),
        };
        let clamp = matches!(self.dtype, OutputType::F32Range { .. });
        let px = self.pixel_bytes();
        for (i, pixel) in src.chunks_exact(px).enumerate() {
            for (k, &s) in self.map.iter().enumerate() {
                let mut v = S::read(&pixel[s as usize * S::SIZE..]) * scale + offset;
                if clamp {
                    v = v.clamp(0.0, 1.0);
                }
                let at = self.index(y, x + i, k) * 4;
                self.out[at..at + 4].copy_from_slice(&v.to_ne_bytes());
            }
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwc_identity_is_direct() {
        let mut out = vec![0u8; 2 * 2 * 3];
        let map = [0, 1, 2];
        let mut s = Sink::new(&mut out, Layout::Hwc, 6, 0, 3, 0, &map, 3, PixelType::U8, OutputType::Native);
        s.write_with(1, 1, 1, |d| {
            d.copy_from_slice(&[7, 8, 9]);
            Ok(())
        })
        .unwrap();
        assert_eq!(out, [0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn bgr_to_chw_and_normalization() {
        // One RGB pixel 255,0,51 as Chw f32, blue first.
        let mut out = vec![0u8; 3 * 4];
        let map = [2, 1, 0];
        let mut s = Sink::new(&mut out, Layout::Chw, 1, 1, 3, 0, &map, 3, PixelType::U8, OutputType::F32Normalized);
        s.put(0, 0, &[255, 0, 51]);
        let v: Vec<f32> = out.chunks_exact(4).map(|b| f32::from_ne_bytes(b.try_into().unwrap())).collect();
        assert!(v.iter().zip([0.2, 0.0, 1.0]).all(|(a, b)| (a - b).abs() < 1e-6), "{v:?}");
    }

    #[test]
    fn range_clamps() {
        let mut out = vec![0u8; 8];
        let map = [0];
        let src: Vec<u8> = [100u16, 900].iter().flat_map(|v| v.to_ne_bytes()).collect();
        let dtype = OutputType::F32Range { min: 200.0, max: 600.0 };
        let mut s = Sink::new(&mut out, Layout::Hwc, 2, 0, 1, 0, &map, 1, PixelType::U16, dtype);
        s.put(0, 0, &src);
        let v: Vec<f32> = out.chunks_exact(4).map(|b| f32::from_ne_bytes(b.try_into().unwrap())).collect();
        assert_eq!(v, [0.0, 1.0]);
    }
}

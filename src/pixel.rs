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
    fn read(b: &[u8]) -> f32;
}

macro_rules! impl_elem {
    ($($t:ty),* $(,)?) => {
        $(
            impl Elem for $t {
                const SIZE: usize = size_of::<$t>();
                fn read(b: &[u8]) -> f32 {
                    <$t>::from_ne_bytes(b[..size_of::<$t>()].try_into().expect("sample size")) as f32
                }
            }
        )*
    };
}

impl_elem!(u8, i8, u16, i16, u32, i32, f32, f64);

/// Value range of an integer sample type; `None` for floats.
fn int_range(t: PixelType) -> Option<(f64, f64)> {
    Some(match t {
        PixelType::U8 => (0.0, u8::MAX as f64),
        PixelType::I8 => (i8::MIN as f64, i8::MAX as f64),
        PixelType::U16 => (0.0, u16::MAX as f64),
        PixelType::I16 => (i16::MIN as f64, i16::MAX as f64),
        PixelType::U32 => (0.0, u32::MAX as f64),
        PixelType::I32 => (i32::MIN as f64, i32::MAX as f64),
        PixelType::F32 | PixelType::F64 => return None,
    })
}

/// `value * scale + offset` (then clamped to 0..1 if `clamp`) of an f32
/// output type for samples of `t` with `bits` significant bits.
fn linear(dtype: OutputType, t: PixelType, bits: Option<u8>) -> (f32, f32, bool) {
    let range = int_range(t);
    let (scale, offset, clamp) = match dtype {
        OutputType::F32Normalized => (range.map_or(1.0, |r| 1.0 / r.1), 0.0, false),
        OutputType::F32Range { min, max } => {
            let span = if max != min { (max - min) as f64 } else { 1.0 };
            (1.0 / span, -min as f64 / span, true)
        }
        OutputType::F32Scaled { scale, offset } => (scale as f64, offset as f64, false),
        OutputType::F32SignificantBits => match range {
            Some((min, max)) => {
                let type_bits = 8 * t.bytes() as u32;
                let b = bits.map_or(type_bits, |b| (b as u32).clamp(1, type_bits));
                // Signed types keep one bit for the sign.
                let b = if min < 0.0 { b.saturating_sub(1).max(1) } else { b };
                (1.0 / (2f64.powi(b as i32) - 1.0).min(max), 0.0, false)
            }
            None => (1.0, 0.0, false),
        },
        OutputType::F32NormalizedShifted => match range {
            Some((min, max)) => (1.0 / (max - min), -min / (max - min), false),
            None => (1.0, 0.0, false),
        },
        OutputType::Native | OutputType::F32 => (1.0, 0.0, false),
    };
    (scale as f32, offset as f32, clamp)
}

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
    /// f32 conversion: `value * scale + offset`, clamped if `clamp`.
    scale: f32,
    offset: f32,
    clamp: bool,
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
        let (scale, offset, clamp) = linear(dtype, src_type, None);
        Self { out, out_elem, layout, row_stride, plane_stride, out_channels, base, map, src_spp, src_type, dtype, scale, offset, clamp, direct }
    }

    /// Sets the significant bits of the samples, used by
    /// [`OutputType::F32SignificantBits`].
    pub fn with_significant_bits(mut self, bits: Option<u8>) -> Self {
        (self.scale, self.offset, self.clamp) = linear(self.dtype, self.src_type, bits);
        self
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
        let count = src.len() / px;
        if count == 0 {
            return;
        }
        match self.layout {
            // Each output channel is a contiguous row of its plane.
            Layout::Chw => {
                for (k, &s) in self.map.iter().enumerate() {
                    let start = self.index(y, x, k) * n;
                    let from = s as usize * n;
                    let dst = &mut self.out[start..start + count * n];
                    if n == 1 {
                        for (d, p) in dst.iter_mut().zip(src[from..].iter().step_by(px)) {
                            *d = *p;
                        }
                    } else {
                        for (d, p) in dst.chunks_exact_mut(n).zip(src.chunks_exact(px)) {
                            d.copy_from_slice(&p[from..from + n]);
                        }
                    }
                }
            }
            // Pixels at a fixed stride, the selected samples of each.
            Layout::Hwc => {
                let stride = self.out_channels * n;
                let start = self.index(y, x, 0) * n;
                let dst = &mut self.out[start..start + (count - 1) * stride + self.map.len() * n];
                for (i, p) in src.chunks_exact(px).enumerate() {
                    for (k, &s) in self.map.iter().enumerate() {
                        let (at, from) = (i * stride + k * n, s as usize * n);
                        dst[at..at + n].copy_from_slice(&p[from..from + n]);
                    }
                }
            }
        }
    }

    fn put_f32<S: Elem>(&mut self, y: usize, x: usize, src: &[u8]) {
        let (scale, offset, clamp) = (self.scale, self.offset, self.clamp);
        let px = self.pixel_bytes();
        // One output sample per pixel into a contiguous row: one tight loop.
        if self.map.len() == 1 && (self.layout == Layout::Chw || self.out_channels == 1) {
            let from = self.map[0] as usize * S::SIZE;
            let start = self.index(y, x, 0) * 4;
            let n = src.len() / px;
            for (d, pixel) in self.out[start..start + 4 * n].chunks_exact_mut(4).zip(src.chunks_exact(px)) {
                let mut v = S::read(&pixel[from..]) * scale + offset;
                if clamp {
                    v = v.clamp(0.0, 1.0);
                }
                d.copy_from_slice(&v.to_ne_bytes());
            }
            return;
        }
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
    fn linear_conversions() {
        assert_eq!(linear(OutputType::F32Normalized, PixelType::U8, None), (1.0 / 255.0, 0.0, false));
        assert_eq!(linear(OutputType::F32SignificantBits, PixelType::U16, Some(12)), (1.0 / 4095.0, 0.0, false));
        assert_eq!(linear(OutputType::F32SignificantBits, PixelType::U16, None), (1.0 / 65535.0, 0.0, false));
        // More bits than the type holds fall back to the type.
        assert_eq!(linear(OutputType::F32SignificantBits, PixelType::U8, Some(12)), (1.0 / 255.0, 0.0, false));
        assert_eq!(linear(OutputType::F32SignificantBits, PixelType::I16, Some(12)), (1.0 / 2047.0, 0.0, false));
        assert_eq!(linear(OutputType::F32SignificantBits, PixelType::F32, Some(12)), (1.0, 0.0, false));
        let (s, o, c) = linear(OutputType::F32NormalizedShifted, PixelType::I8, None);
        assert!(((-128.0 * s + o) - 0.0).abs() < 1e-7 && ((127.0 * s + o) - 1.0).abs() < 1e-6 && !c);
        assert_eq!(linear(OutputType::F32Scaled { scale: 2.0, offset: -1.0 }, PixelType::U16, None), (2.0, -1.0, false));
        assert!(linear(OutputType::F32Range { min: 0.0, max: 10.0 }, PixelType::U16, None).2);
    }

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

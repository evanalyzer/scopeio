//! # sample
//!
//! Typed access to decoded pixel data.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::PixelType;

mod sealed {
    pub trait Sealed {}
}

/// Rust type of one sample, implemented for all types in [`PixelType`].
///
/// The trait is sealed: only plain numeric types without padding implement
/// it, which makes viewing a `[T]` as bytes sound.
pub trait Sample: sealed::Sealed + Copy + Default + Send + Sync + 'static {
    /// Pixel type corresponding to `Self`.
    const TYPE: PixelType;
}

macro_rules! impl_sample {
    ($($t:ty => $variant:ident),* $(,)?) => {
        $(
            impl sealed::Sealed for $t {}
            impl Sample for $t {
                const TYPE: PixelType = PixelType::$variant;
            }
        )*
    };
}

impl_sample!(u8 => U8, i8 => I8, u16 => U16, i16 => I16, u32 => U32, i32 => I32, f32 => F32, f64 => F64);

/// Views a sample slice as its raw bytes in native byte order.
pub(crate) fn as_bytes_mut<T: Sample>(samples: &mut [T]) -> &mut [u8] {
    // SAFETY: `Sample` is sealed and only implemented for primitive numeric
    // types, which have no padding and accept any bit pattern. The byte view
    // covers exactly the memory of `samples` and borrows it mutably.
    unsafe {
        std::slice::from_raw_parts_mut(samples.as_mut_ptr().cast::<u8>(), size_of_val(samples))
    }
}

/// Decoded pixel data with its sample type known at runtime.
///
/// Samples are row-major and interleaved (height x width x samples per
/// pixel), in native byte order.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PixelBuf {
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    U32(Vec<u32>),
    I32(Vec<i32>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl PixelBuf {
    /// Sample type of the buffer.
    pub fn pixel_type(&self) -> PixelType {
        match self {
            PixelBuf::U8(_) => PixelType::U8,
            PixelBuf::I8(_) => PixelType::I8,
            PixelBuf::U16(_) => PixelType::U16,
            PixelBuf::I16(_) => PixelType::I16,
            PixelBuf::U32(_) => PixelType::U32,
            PixelBuf::I32(_) => PixelType::I32,
            PixelBuf::F32(_) => PixelType::F32,
            PixelBuf::F64(_) => PixelType::F64,
        }
    }

    /// Number of samples in the buffer.
    pub fn len(&self) -> usize {
        match self {
            PixelBuf::U8(v) => v.len(),
            PixelBuf::I8(v) => v.len(),
            PixelBuf::U16(v) => v.len(),
            PixelBuf::I16(v) => v.len(),
            PixelBuf::U32(v) => v.len(),
            PixelBuf::I32(v) => v.len(),
            PixelBuf::F32(v) => v.len(),
            PixelBuf::F64(v) => v.len(),
        }
    }

    /// Returns `true` if the buffer holds no samples.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Byte order of sample data as stored in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    /// Byte order of the target platform.
    #[cfg(target_endian = "little")]
    pub const NATIVE: ByteOrder = ByteOrder::Little;
    /// Byte order of the target platform.
    #[cfg(target_endian = "big")]
    pub const NATIVE: ByteOrder = ByteOrder::Big;

    /// Converts samples stored in this byte order to native byte order in place.
    ///
    /// Readers call this on decoded data before returning it. Does nothing if
    /// the byte order already is native or samples are single bytes.
    ///
    /// # Arguments
    ///
    /// * `buf` - Sample data; a trailing partial sample is left untouched.
    /// * `pixel_type` - Type of the samples in `buf`.
    pub fn to_native(self, buf: &mut [u8], pixel_type: PixelType) {
        let n = pixel_type.bytes();
        if self == Self::NATIVE || n == 1 {
            return;
        }
        for sample in buf.chunks_exact_mut(n) {
            sample.reverse();
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_types_match_pixel_type_sizes() {
        fn check<T: Sample>() {
            assert_eq!(size_of::<T>(), T::TYPE.bytes());
        }
        check::<u8>();
        check::<i8>();
        check::<u16>();
        check::<i16>();
        check::<u32>();
        check::<i32>();
        check::<f32>();
        check::<f64>();
    }

    #[test]
    fn as_bytes_mut_writes_through() {
        let mut v = [0u16; 2];
        as_bytes_mut(&mut v).copy_from_slice(&[1u16.to_ne_bytes(), 0x0302u16.to_ne_bytes()].concat());
        assert_eq!(v, [1, 0x0302]);
    }

    #[test]
    fn to_native_swaps_foreign_order_only() {
        let foreign = match ByteOrder::NATIVE {
            ByteOrder::Little => ByteOrder::Big,
            ByteOrder::Big => ByteOrder::Little,
        };
        let mut buf = 1.5f32.to_ne_bytes().iter().rev().copied().collect::<Vec<_>>();
        foreign.to_native(&mut buf, PixelType::F32);
        assert_eq!(f32::from_ne_bytes(buf.try_into().unwrap()), 1.5);

        let mut buf = 0x0102u16.to_ne_bytes();
        ByteOrder::NATIVE.to_native(&mut buf, PixelType::U16);
        assert_eq!(u16::from_ne_bytes(buf), 0x0102);
    }

    #[test]
    fn to_native_matches_std_decoding() {
        let mut be = 0x0102_0304u32.to_be_bytes();
        ByteOrder::Big.to_native(&mut be, PixelType::U32);
        assert_eq!(u32::from_ne_bytes(be), 0x0102_0304);
        let mut le = 0x0102_0304u32.to_le_bytes();
        ByteOrder::Little.to_native(&mut le, PixelType::U32);
        assert_eq!(u32::from_ne_bytes(le), 0x0102_0304);
    }
}

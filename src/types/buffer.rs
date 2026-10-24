//! # buffer
//!
//! Typed output buffers and views.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::pixel::{Pixel, as_bytes, as_bytes_mut};
use crate::{ColorOrder, Error, Layout, PixelType, Result};

macro_rules! pixel_enums {
    ($($variant:ident($t:ty)),* $(,)?) => {
        /// Owned, contiguous samples of one pixel type.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        pub enum PixelBuf {
            $($variant(Vec<$t>),)*
        }

        /// Mutable samples of one pixel type, borrowed from the caller.
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum PixelSliceMut<'a> {
            $($variant(&'a mut [$t]),)*
        }

        impl PixelBuf {
            /// Zero-filled buffer of `len` samples of `pixel_type`.
            pub fn zeros(pixel_type: PixelType, len: usize) -> Self {
                match pixel_type {
                    $(PixelType::$variant => PixelBuf::$variant(vec![<$t>::default(); len]),)*
                }
            }

            /// Zero-filled buffer like [`PixelBuf::zeros`], or `None` if the
            /// memory cannot be allocated.
            pub fn try_zeros(pixel_type: PixelType, len: usize) -> Option<Self> {
                match pixel_type {
                    $(PixelType::$variant => {
                        let mut v: Vec<$t> = Vec::new();
                        v.try_reserve_exact(len).ok()?;
                        v.resize(len, <$t>::default());
                        Some(PixelBuf::$variant(v))
                    })*
                }
            }

            /// Sample type of the buffer.
            pub fn pixel_type(&self) -> PixelType {
                match self {
                    $(PixelBuf::$variant(_) => PixelType::$variant,)*
                }
            }

            /// Number of samples.
            pub fn len(&self) -> usize {
                match self {
                    $(PixelBuf::$variant(v) => v.len(),)*
                }
            }

            /// Returns `true` if the buffer holds no samples.
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// The samples as bytes in native byte order, without copying.
            pub fn as_bytes(&self) -> &[u8] {
                match self {
                    $(PixelBuf::$variant(v) => as_bytes(v),)*
                }
            }

            /// Mutable view of all samples.
            pub fn as_slice_mut(&mut self) -> PixelSliceMut<'_> {
                match self {
                    $(PixelBuf::$variant(v) => PixelSliceMut::$variant(v),)*
                }
            }

            /// The samples as `Vec<T>`, without copying.
            ///
            /// # Errors
            ///
            /// Returns [`Error::TypeMismatch`] if `T` is not the buffer's type.
            pub fn into_vec<T: Pixel>(self) -> Result<Vec<T>> {
                let found = self.pixel_type();
                match self {
                    $(PixelBuf::$variant(v) => {
                        let any: Box<dyn std::any::Any> = Box::new(v);
                        any.downcast::<Vec<T>>().map(|b| *b).map_err(|_| {
                            Error::TypeMismatch(format!("buffer holds {found:?}, requested {:?}", T::TYPE))
                        })
                    })*
                }
            }
        }

        impl PixelSliceMut<'_> {
            /// Sample type of the slice.
            pub fn pixel_type(&self) -> PixelType {
                match self {
                    $(PixelSliceMut::$variant(_) => PixelType::$variant,)*
                }
            }

            /// Number of samples.
            pub fn len(&self) -> usize {
                match self {
                    $(PixelSliceMut::$variant(v) => v.len(),)*
                }
            }

            /// Returns `true` if the slice holds no samples.
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// The samples as mutable bytes in native byte order.
            pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
                match self {
                    $(PixelSliceMut::$variant(v) => as_bytes_mut(v),)*
                }
            }
        }

        $(
            impl<'a> From<&'a mut [$t]> for PixelSliceMut<'a> {
                fn from(v: &'a mut [$t]) -> Self {
                    PixelSliceMut::$variant(v)
                }
            }
        )*
    };
}

pixel_enums!(U8(u8), I8(i8), U16(u16), I16(i16), U32(u32), I32(i32), F32(f32), F64(f64));

/// A caller-provided output buffer with its shape and strides, in
/// elements.
#[derive(Debug)]
#[non_exhaustive]
pub struct PixelViewMut<'a> {
    pub data: PixelSliceMut<'a>,
    pub width: u64,
    pub height: u64,
    /// Samples per pixel in this buffer.
    pub channels: u32,
    pub layout: Layout,
    /// Elements between rows (>= `width * channels` for Hwc, >= `width`
    /// for Chw).
    pub row_stride: usize,
    /// Elements between channel planes (Chw only).
    pub plane_stride: usize,
}

impl<'a> PixelViewMut<'a> {
    /// View of a contiguous buffer without padding.
    pub fn contiguous(data: impl Into<PixelSliceMut<'a>>, width: u64, height: u64, channels: u32, layout: Layout) -> Self {
        let (row_stride, plane_stride) = match layout {
            Layout::Hwc => (width as usize * channels as usize, 0),
            Layout::Chw => (width as usize, width as usize * height as usize),
        };
        Self { data: data.into(), width, height, channels, layout, row_stride, plane_stride }
    }

    /// View with explicit strides, e.g. a sub-rectangle of a larger image.
    #[allow(clippy::too_many_arguments)]
    pub fn strided(
        data: impl Into<PixelSliceMut<'a>>,
        width: u64,
        height: u64,
        channels: u32,
        layout: Layout,
        row_stride: usize,
        plane_stride: usize,
    ) -> Self {
        Self { data: data.into(), width, height, channels, layout, row_stride, plane_stride }
    }

    /// Checks that the strides fit the buffer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TypeMismatch`] if a row or plane overlaps the next
    /// or the buffer is too small.
    pub(crate) fn check(&self) -> Result<()> {
        let (w, h, c) = (self.width as usize, self.height as usize, self.channels as usize);
        let needed = match self.layout {
            Layout::Hwc => {
                if self.row_stride < w * c {
                    return Err(Error::TypeMismatch(format!("row stride {} < {}", self.row_stride, w * c)));
                }
                (h - 1) * self.row_stride + w * c
            }
            Layout::Chw => {
                let plane = (h - 1) * self.row_stride + w;
                if self.row_stride < w || c > 1 && self.plane_stride < plane {
                    return Err(Error::TypeMismatch("row or plane stride too small".into()));
                }
                (c - 1) * self.plane_stride + plane
            }
        };
        if self.data.len() < needed {
            return Err(Error::TypeMismatch(format!("buffer has {} elements, needs {needed}", self.data.len())));
        }
        Ok(())
    }
}

/// Size and element type of the output of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputShape {
    pub width: u64,
    pub height: u64,
    /// Samples per pixel of the output.
    pub channels: u32,
    pub layout: Layout,
    pub pixel_type: PixelType,
}

impl OutputShape {
    /// Number of elements of a contiguous buffer.
    pub fn len(&self) -> usize {
        self.width as usize * self.height as usize * self.channels as usize
    }

    /// Returns `true` if the output holds no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `[H, W, C]` for Hwc, `[C, H, W]` for Chw.
    pub fn dims(&self) -> [usize; 3] {
        let (h, w, c) = (self.height as usize, self.width as usize, self.channels as usize);
        match self.layout {
            Layout::Hwc => [h, w, c],
            Layout::Chw => [c, h, w],
        }
    }
}

/// An owned image: contiguous, aligned samples without padding.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ImageBuf {
    pub data: PixelBuf,
    pub width: u64,
    pub height: u64,
    /// Samples per pixel in this buffer (after channel selection).
    pub channels: u32,
    pub layout: Layout,
    pub color_order: ColorOrder,
}

impl ImageBuf {
    /// `[H, W, C]` for Hwc, `[C, H, W]` for Chw.
    pub fn shape(&self) -> [usize; 3] {
        let (h, w, c) = (self.height as usize, self.width as usize, self.channels as usize);
        match self.layout {
            Layout::Hwc => [h, w, c],
            Layout::Chw => [c, h, w],
        }
    }

    /// The samples as bytes in native byte order, without copying.
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_bytes()
    }

    /// The samples as `Vec<T>`, without copying.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TypeMismatch`] if `T` is not the buffer's type.
    pub fn into_vec<T: Pixel>(self) -> Result<Vec<T>> {
        self.data.into_vec()
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_vec_checks_type_without_copy() {
        let buf = PixelBuf::U16(vec![1, 2, 3]);
        let ptr = buf.as_bytes().as_ptr();
        let v: Vec<u16> = buf.clone().into_vec().unwrap();
        assert_eq!(v, [1, 2, 3]);
        assert!(buf.clone().into_vec::<u8>().is_err());
        let v: Vec<u16> = buf.into_vec().unwrap();
        assert_eq!(v.as_ptr().cast::<u8>(), ptr);
    }

    #[test]
    fn view_checks_strides() {
        let mut data = [0u8; 12];
        assert!(PixelViewMut::contiguous(&mut data[..], 2, 2, 3, Layout::Hwc).check().is_ok());
        assert!(PixelViewMut::contiguous(&mut data[..11], 2, 2, 3, Layout::Hwc).check().is_err());
        assert!(PixelViewMut::strided(&mut data[..], 2, 2, 3, Layout::Hwc, 5, 0).check().is_err());
        assert!(PixelViewMut::contiguous(&mut data[..], 2, 2, 3, Layout::Chw).check().is_ok());
    }
}

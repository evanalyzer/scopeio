//! # strided
//!
//! Samples of an uncompressed or whole-file-compressed raw array, read
//! plane by plane with element strides (ICS, NRRD).
//!
//! The samples are one array of `bytes`-wide elements in either byte
//! order, x and y with any strides, one z axis, and channel and time
//! dimensions made of several axes (fastest first). They are read from a
//! file at an offset, or decoded once into memory on first read
//! (gzip, text).
//!
//! Provenance: own design. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::{Error, Plane, Region, Result, scratch};

/// Decodes the samples into memory.
type Loader = Box<dyn Fn() -> Result<Vec<u8>> + Send + Sync>;

/// Where the samples are.
pub(super) enum Data {
    File { file: File, offset: u64 },
    Decoded { load: Loader, data: OnceLock<std::result::Result<Vec<u8>, String>> },
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Data::File { offset, .. } => f.debug_struct("File").field("offset", offset).finish(),
            Data::Decoded { .. } => f.write_str("Decoded"),
        }
    }
}

impl Data {
    /// `len` bytes of `path` at `offset`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] when the file is shorter.
    pub fn file(path: &Path, offset: u64, len: u64, what: &str) -> Result<Self> {
        let file = File::open(path)?;
        if offset.saturating_add(len) > file.metadata()?.len() {
            return Err(Error::Corrupt(format!("{what} data shorter than its layout")));
        }
        Ok(Data::File { file, offset })
    }

    /// Samples decoded by `load` on first read.
    pub fn decoded(load: impl Fn() -> Result<Vec<u8>> + Send + Sync + 'static) -> Self {
        Data::Decoded { load: Box::new(load), data: OnceLock::new() }
    }

    /// Gzip data of `path` from `offset`; the samples start `skip` bytes into
    /// the decompressed stream and are `len` bytes long.
    pub fn gzip(path: &Path, offset: u64, skip: u64, len: u64, what: &'static str) -> Self {
        let path = path.to_path_buf();
        Data::decoded(move || {
            let src = std::fs::read(&path)?;
            let src = src.get(offset as usize..).ok_or_else(|| Error::Corrupt(format!("{what} data offset beyond the file")))?;
            // Deflate expands by at most 1032:1: a larger claim is corrupt.
            let total = skip.checked_add(len).filter(|&n| n <= usize::MAX as u64 && n <= (src.len() as u64).saturating_mul(1032) + 64);
            let total = total.ok_or_else(|| Error::Corrupt(format!("{what} data larger than its compressed size allows")))?;
            let mut out = vec![0u8; total as usize];
            let n = crate::codecs::gzip(src, &mut out)?;
            if n < out.len() {
                return Err(Error::Corrupt(format!("compressed {what} data shorter than its layout")));
            }
            out.drain(..skip as usize);
            Ok(out)
        })
    }
}

/// A raw sample array.
#[derive(Debug)]
pub(super) struct Strided {
    pub data: Data,
    /// Element strides of x, y and z.
    pub sx: u64,
    pub sy: u64,
    pub sz: u64,
    /// Channel and time dimensions (fastest first) as (stride, size).
    pub c_axes: Vec<(u64, u64)>,
    pub t_axes: Vec<(u64, u64)>,
    /// Bytes per element.
    pub bytes: usize,
    pub little: bool,
    /// Rows stored from the bottom; `height` rows.
    pub flip_y: bool,
    pub height: u64,
    /// Bytes of the whole array.
    pub total: u64,
    /// Format name for error messages.
    pub what: &'static str,
}

/// Element offset of index `i` decomposed over `axes` (fastest first).
pub(super) fn axes_offset(axes: &[(u64, u64)], mut i: u64) -> u64 {
    let mut at = 0;
    for &(stride, size) in axes {
        at += (i % size) * stride;
        i /= size;
    }
    at
}

impl Strided {
    /// Copies `out.len()` bytes at `at` (from the start of the samples).
    fn read_at(&self, out: &mut [u8], at: u64) -> Result<()> {
        match &self.data {
            Data::File { file, offset } => read_exact_at(file, out, offset + at),
            Data::Decoded { load, data } => {
                let data = data.get_or_init(|| load().map_err(|e| e.to_string()));
                let data = data.as_ref().map_err(|e| Error::Corrupt(e.clone()))?;
                let start = at as usize;
                out.copy_from_slice(data.get(start..start + out.len()).ok_or_else(|| Error::Corrupt(format!("{} read beyond its data", self.what)))?);
                Ok(())
            }
        }
    }

    /// Writes `region` of `plane` to `sink` in native (little or big
    /// endian as the host) element order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for reads beyond the data.
    pub fn read_plane(&self, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let e = self.bytes as u64;
        let base = plane.z as u64 * self.sz + axes_offset(&self.c_axes, plane.c as u64) + axes_offset(&self.t_axes, plane.t as u64);
        let w = region.width as usize;
        // A row spans `(w - 1) * sx + 1` elements; with sx > 1 every sx-th is kept.
        let span = ((w as u64 - 1) * self.sx + 1) * e;
        let swap = self.bytes > 1 && self.little != cfg!(target_endian = "little");
        scratch::with(span as usize, |row| {
            for r in 0..region.height as usize {
                let y = region.y + r as u64;
                let y = if self.flip_y { self.height - 1 - y } else { y };
                let at = (base + y * self.sy + region.x * self.sx) * e;
                if at + span > self.total {
                    return Err(Error::Corrupt(format!("{} read beyond its data", self.what)));
                }
                self.read_at(row, at)?;
                sink.write_with(r, 0, w, |d| {
                    if self.sx == 1 {
                        d.copy_from_slice(row);
                    } else {
                        for (k, out) in d.chunks_exact_mut(self.bytes).enumerate() {
                            let s = k * (self.sx as usize) * self.bytes;
                            out.copy_from_slice(&row[s..s + self.bytes]);
                        }
                    }
                    if swap {
                        d.chunks_exact_mut(self.bytes).for_each(<[u8]>::reverse);
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decomposes_axes() {
        // Two channel axes of sizes 2 and 3 with strides 1 and 10.
        let axes = [(1, 2), (10, 3)];
        assert_eq!(axes_offset(&axes, 0), 0);
        assert_eq!(axes_offset(&axes, 1), 1);
        assert_eq!(axes_offset(&axes, 3), 11);
    }
}

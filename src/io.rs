//! # io
//!
//! Positioned file reads shared by all readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::sync::Arc;

use crate::{Error, Result};

/// Bytes readable at any offset without a shared cursor.
pub(crate) trait ReadAt {
    fn read_exact_at_io(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()>;
}

impl ReadAt for File {
    fn read_exact_at_io(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        read_at_impl(self, buf, offset)
    }
}

/// A file, or bytes held in memory (e.g. a stream of a compound file).
#[derive(Debug)]
pub(crate) enum Source {
    File(File),
    Memory(Arc<[u8]>),
}

impl Source {
    /// Length in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file size cannot be queried.
    pub fn len(&self) -> Result<u64> {
        match self {
            Source::File(f) => Ok(f.metadata()?.len()),
            Source::Memory(b) => Ok(b.len() as u64),
        }
    }
}

impl ReadAt for Source {
    fn read_exact_at_io(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        match self {
            Source::File(f) => read_at_impl(f, buf, offset),
            Source::Memory(b) => {
                let start = usize::try_from(offset).ok().filter(|&s| s.checked_add(buf.len()).is_some_and(|e| e <= b.len()));
                let start = start.ok_or(std::io::ErrorKind::UnexpectedEof)?;
                buf.copy_from_slice(&b[start..start + buf.len()]);
                Ok(())
            }
        }
    }
}

/// Fills `buf` from `file` starting at `offset`, without moving a shared
/// cursor, so that one file handle can serve parallel reads.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] if the file ends before `buf` is full, or
/// [`Error::Io`] on other failures.
pub(crate) fn read_exact_at(file: &(impl ReadAt + ?Sized), buf: &mut [u8], offset: u64) -> Result<()> {
    let result = file.read_exact_at_io(buf, offset);
    match result {
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            Err(Error::Corrupt(format!("file truncated: need {} bytes at offset {offset}", buf.len())))
        }
        other => other.map_err(Error::Io),
    }
}

/// Bytes of rows read with one call by [`read_rows`].
const BAND_BYTES: usize = 256 << 10;

/// Whether [`read_rows`] should read rows of `len` bytes `stride` bytes
/// apart in bands: short rows (one system call each costs more than the
/// copy) that fill most of the stride. Longer rows are read one by one,
/// which keeps them in cache.
pub(crate) fn rows_in_bands(len: usize, stride: usize) -> bool {
    len > 0 && len < 8 << 10 && stride <= 4 * len
}

/// Reads `rows` rows of `len` bytes, `stride` bytes apart from `offset`,
/// in bands of rows with one read each, and passes each row to `f`. Bytes
/// at or after `end` (the end of the available data) read as zeros.
///
/// # Errors
///
/// Returns the errors of [`read_exact_at`] and of `f`.
pub(crate) fn read_rows(
    file: &(impl ReadAt + ?Sized),
    offset: u64,
    stride: usize,
    len: usize,
    rows: usize,
    end: u64,
    mut f: impl FnMut(usize, &[u8]) -> Result<()>,
) -> Result<()> {
    if rows == 0 {
        return Ok(());
    }
    let band = (BAND_BYTES / stride.max(1)).clamp(1, rows);
    crate::scratch::with((band - 1) * stride + len, |buf| {
        for r0 in (0..rows).step_by(band) {
            let n = band.min(rows - r0);
            let span = (n - 1) * stride + len;
            let at = offset + (r0 * stride) as u64;
            let have = end.saturating_sub(at).min(span as u64) as usize;
            read_exact_at(file, &mut buf[..have], at)?;
            buf[have..span].fill(0);
            for r in 0..n {
                f(r0 + r, &buf[r * stride..r * stride + len])?;
            }
        }
        Ok(())
    })
}

#[cfg(unix)]
fn read_at_impl(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at_impl(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_read(buf, offset) {
            Ok(0) => return Err(std::io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => {
                buf = &mut buf[n..];
                offset += n as u64;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rows_in_bands_and_zero_fills_past_the_end() {
        let data: Vec<u8> = (0..100u8).collect();
        let mem = Source::Memory(Arc::from(data.as_slice()));
        let mut got = Vec::new();
        // Rows of 3 bytes every 10 bytes from 5; data ends at 37.
        read_rows(&mem, 5, 10, 3, 4, 37, |r, row| {
            got.push((r, row.to_vec()));
            Ok(())
        })
        .unwrap();
        assert_eq!(got, [(0, vec![5, 6, 7]), (1, vec![15, 16, 17]), (2, vec![25, 26, 27]), (3, vec![35, 36, 0])]);
        assert!(rows_in_bands(1000, 4000) && !rows_in_bands(100, 4000));
    }

    #[test]
    fn reads_at_offset_and_reports_truncation() {
        let path = std::env::temp_dir().join(format!("scopeio-io-{}", std::process::id()));
        std::fs::write(&path, b"0123456789").unwrap();
        let file = File::open(&path).unwrap();
        let mut buf = [0u8; 3];
        read_exact_at(&file, &mut buf, 4).unwrap();
        assert_eq!(&buf, b"456");
        assert!(matches!(read_exact_at(&file, &mut buf, 8), Err(Error::Corrupt(_))));
        std::fs::remove_file(path).unwrap();
        let mem = Source::Memory(Arc::from(&b"0123456789"[..]));
        read_exact_at(&mem, &mut buf, 7).unwrap();
        assert_eq!(&buf, b"789");
        assert!(matches!(read_exact_at(&mem, &mut buf, 8), Err(Error::Corrupt(_))));
    }
}

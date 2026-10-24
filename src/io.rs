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

use crate::{Error, Result};

/// Fills `buf` from `file` starting at `offset`, without moving a shared
/// cursor, so that one file handle can serve parallel reads.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] if the file ends before `buf` is full, or
/// [`Error::Io`] on other failures.
pub(crate) fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> Result<()> {
    let result = read_at_impl(file, buf, offset);
    match result {
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(Error::Corrupt(format!(
            "file truncated: need {} bytes at offset {offset}",
            buf.len()
        ))),
        other => other.map_err(Error::Io),
    }
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
    fn reads_at_offset_and_reports_truncation() {
        let path = std::env::temp_dir().join(format!("scopeio-io-{}", std::process::id()));
        std::fs::write(&path, b"0123456789").unwrap();
        let file = File::open(&path).unwrap();
        let mut buf = [0u8; 3];
        read_exact_at(&file, &mut buf, 4).unwrap();
        assert_eq!(&buf, b"456");
        assert!(matches!(read_exact_at(&file, &mut buf, 8), Err(Error::Corrupt(_))));
        std::fs::remove_file(path).unwrap();
    }
}

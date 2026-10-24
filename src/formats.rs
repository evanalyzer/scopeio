//! # formats
//!
//! Format readers, format detection and the entry points.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#[cfg(feature = "bmp")]
mod bmp;
#[cfg(feature = "tiff")]
mod tiff;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::reader::Reader;
use crate::{Error, FormatId, FormatInfo, ImageReader, OpenOptions, Result};

/// Number of leading bytes inspected to detect the format.
const MAGIC_LEN: usize = 512;

/// Formats compiled into this build.
static SUPPORTED: &[FormatInfo] = &[
    #[cfg(feature = "bmp")]
    bmp::FORMAT,
    #[cfg(feature = "tiff")]
    tiff::FORMAT,
    #[cfg(feature = "tiff")]
    tiff::OME_FORMAT,
];

/// Formats this build can read.
pub fn supported_formats() -> &'static [FormatInfo] {
    SUPPORTED
}

/// Detects the format of a file from its content.
///
/// # Returns
///
/// The format, or `None` if no reader of this build recognises the file.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read.
pub fn probe(path: &Path) -> Result<Option<FormatId>> {
    let mut head = Vec::with_capacity(MAGIC_LEN);
    File::open(path)?.take(MAGIC_LEN as u64).read_to_end(&mut head)?;
    #[cfg(feature = "bmp")]
    if bmp::matches(&head) {
        return Ok(Some(FormatId::Bmp));
    }
    #[cfg(feature = "tiff")]
    if let Some(id) = tiff::probe(path, &head)? {
        return Ok(Some(id));
    }
    let _ = &head;
    Ok(None)
}

/// Opens an image file with default options.
///
/// # Errors
///
/// Returns [`Error::UnsupportedFormat`] if no reader recognises the file, or
/// the error of the matching reader.
pub fn open(path: &Path) -> Result<Arc<dyn ImageReader>> {
    open_with(path, &OpenOptions::default())
}

/// Opens an image file.
///
/// # Errors
///
/// Same as [`open`]; [`Error::UnsupportedVariant`] if an option is not
/// implemented.
pub fn open_with(path: &Path, opts: &OpenOptions) -> Result<Arc<dyn ImageReader>> {
    if opts.mmap {
        return Err(Error::UnsupportedVariant("memory mapping is not implemented yet".into()));
    }
    let _ = opts;
    match probe(path)? {
        #[cfg(feature = "bmp")]
        Some(FormatId::Bmp) => Ok(Arc::new(Reader(bmp::Bmp::open(path)?))),
        #[cfg(feature = "tiff")]
        Some(FormatId::Tiff | FormatId::OmeTiff) => Ok(Arc::new(Reader(tiff::Tiff::open(path, opts)?))),
        _ => Err(Error::UnsupportedFormat(path.display().to_string())),
    }
}

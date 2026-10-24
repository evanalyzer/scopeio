//! # formats
//!
//! Format readers and format detection.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

pub mod bmp;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::{Error, ImageReader, Result};

/// Number of leading bytes inspected to detect the format.
const MAGIC_LEN: usize = 32;

/// Opens an image file with the reader matching its content.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] if no reader recognises the file, or the
/// error of the matching reader.
pub fn open(path: impl AsRef<Path>) -> Result<Box<dyn ImageReader>> {
    let path = path.as_ref();
    let mut head = Vec::with_capacity(MAGIC_LEN);
    File::open(path)?.take(MAGIC_LEN as u64).read_to_end(&mut head)?;
    if bmp::BmpReader::matches(&head) {
        return Ok(Box::new(bmp::BmpReader::open(path)?));
    }
    Err(Error::Unsupported(format!("unknown file format: {}", path.display())))
}

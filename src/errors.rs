//! # errors
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fmt;

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by scopeio.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Underlying I/O failure.
    Io(std::io::Error),
    /// No reader recognises the file.
    UnsupportedFormat(String),
    /// The format is recognised, but the file uses a feature (compression,
    /// layout, sample type) this reader does not implement.
    UnsupportedVariant(String),
    /// The file content is invalid or truncated.
    Corrupt(String),
    /// A series, level, tile, region, plane, channel or sample index is out
    /// of range.
    OutOfRange(String),
    /// An output buffer has the wrong element type, shape or stride.
    TypeMismatch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::UnsupportedFormat(msg) => write!(f, "unsupported format: {msg}"),
            Error::UnsupportedVariant(msg) => write!(f, "unsupported: {msg}"),
            Error::Corrupt(msg) => write!(f, "corrupt file: {msg}"),
            Error::OutOfRange(msg) => write!(f, "out of range: {msg}"),
            Error::TypeMismatch(msg) => write!(f, "output mismatch: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

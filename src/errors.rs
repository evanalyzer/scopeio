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

use crate::{Plane, Region};

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by readers.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Underlying I/O failure.
    Io(std::io::Error),
    /// The series index is not smaller than the series count.
    SeriesOutOfRange { series: usize, count: usize },
    /// The level index is not smaller than the level count of the series.
    LevelOutOfRange { level: usize, count: usize },
    /// One of the plane coordinates exceeds the series dimensions.
    PlaneOutOfRange(Plane),
    /// The region is empty or extends beyond the level dimensions.
    RegionOutOfBounds(Region),
    /// The output buffer cannot hold the requested region.
    BufferTooSmall { needed: usize, got: usize },
    /// The file uses a feature this reader does not implement.
    Unsupported(String),
    /// The file content is invalid.
    Corrupt(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::SeriesOutOfRange { series, count } => {
                write!(f, "series {series} out of range (count {count})")
            }
            Error::LevelOutOfRange { level, count } => {
                write!(f, "level {level} out of range (count {count})")
            }
            Error::PlaneOutOfRange(p) => {
                write!(f, "plane c={} z={} t={} out of range", p.c, p.z, p.t)
            }
            Error::RegionOutOfBounds(r) => write!(
                f,
                "region {}x{} at ({}, {}) out of bounds",
                r.width, r.height, r.x, r.y
            ),
            Error::BufferTooSmall { needed, got } => {
                write!(f, "buffer too small: need {needed} bytes, got {got}")
            }
            Error::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            Error::Corrupt(msg) => write!(f, "corrupt file: {msg}"),
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

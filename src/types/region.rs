//! # region
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Rectangular pixel region within one plane, in coordinates of the
/// selected level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region {
    pub x: u64,
    pub y: u64,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// Creates a region from its top-left corner and size.
    pub fn new(x: u64, y: u64, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

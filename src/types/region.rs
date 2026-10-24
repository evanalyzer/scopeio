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
/// requested level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Region {
    pub x: u64,
    pub y: u64,
    pub width: u64,
    pub height: u64,
}

impl Region {
    /// Creates a region from its top-left corner and size.
    pub fn new(x: u64, y: u64, width: u64, height: u64) -> Self {
        Self { x, y, width, height }
    }

    /// Returns `true` if the region is not empty and lies within an image
    /// of `width` x `height` pixels.
    pub fn fits(&self, width: u64, height: u64) -> bool {
        let fits = |pos: u64, len: u64, max: u64| len > 0 && pos.checked_add(len).is_some_and(|end| end <= max);
        fits(self.x, self.width, width) && fits(self.y, self.height, height)
    }
}

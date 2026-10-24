//! # plane
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Address of a single 2D plane within a series.
///
/// Marked `#[non_exhaustive]` so further dimensions can be added later;
/// construct it with [`Plane::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct Plane {
    pub c: u32,
    pub z: u32,
    pub t: u32,
}

impl Plane {
    /// Creates a plane address from channel, z-slice and time point.
    pub fn new(c: u32, z: u32, t: u32) -> Self {
        Self { c, z, t }
    }
}

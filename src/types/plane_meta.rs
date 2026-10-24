//! # plane_meta
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::Position;

/// Acquisition data of one plane. Fields are `None` if unknown.
///
/// Returned by [`ImageReader::plane_meta`](crate::ImageReader::plane_meta)
/// so readers can load it lazily.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub struct PlaneMeta {
    /// Time since the start of the acquisition.
    pub delta_t_s: Option<f64>,
    pub exposure_s: Option<f64>,
    pub position: Option<Position>,
}

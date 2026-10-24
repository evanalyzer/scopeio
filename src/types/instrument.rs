//! # instrument
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Objective used for a series. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ObjectiveMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub magnification: Option<f64>,
    pub numerical_aperture: Option<f64>,
    /// Immersion medium, e.g. "Oil", "Water", "Air".
    pub immersion: Option<String>,
}

//! # position
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Stage position in nanometres. Fields are `None` if unknown.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x_nm: Option<f64>,
    pub y_nm: Option<f64>,
    pub z_nm: Option<f64>,
}

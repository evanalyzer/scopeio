//! # plate
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::Position;

/// Layout of a multi-well plate.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct PlateMeta {
    pub name: Option<String>,
    pub rows: u32,
    pub cols: u32,
    /// One entry per imaged field.
    pub wells: Vec<WellSample>,
}

/// One imaged field of a well. `row`, `col` and `field` are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub struct WellSample {
    pub row: u32,
    pub col: u32,
    pub field: u32,
    /// Index of the series holding this field's image.
    pub series: usize,
    pub position: Option<Position>,
}

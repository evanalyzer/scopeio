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
    pub columns: u32,
    /// Imaged wells only.
    pub wells: Vec<WellMeta>,
}

/// One well of a plate. `row` and `column` are 0-based.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct WellMeta {
    pub row: u32,
    pub column: u32,
    pub fields: Vec<WellField>,
}

/// One imaged field within a well.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub struct WellField {
    /// Index of the series holding this field's image.
    pub series: usize,
    pub position: Option<Position>,
}

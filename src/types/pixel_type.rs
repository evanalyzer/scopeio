//! # pixel_type
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Data type of a single sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum PixelType {
    #[default]
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    F64,
}

impl PixelType {
    /// Size of one sample in bytes.
    pub fn bytes(self) -> usize {
        match self {
            PixelType::U8 | PixelType::I8 => 1,
            PixelType::U16 | PixelType::I16 => 2,
            PixelType::U32 | PixelType::I32 | PixelType::F32 => 4,
            PixelType::F64 => 8,
        }
    }
}

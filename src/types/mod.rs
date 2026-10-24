//! # types
//!
//! Data types shared by all readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod meta;
mod pixel_type;
mod plane;
mod region;

pub use meta::{LevelMeta, PhysicalSize, SeriesMeta};
pub use pixel_type::PixelType;
pub use plane::Plane;
pub use region::Region;

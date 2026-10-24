//! # lib
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.
//!
//! ## Data model
//! A file contains one or more **series**. Each series has one or more
//! resolution **levels** (level 0 is full resolution) and a set of
//! **planes** addressed by channel, z-slice and time point. Pixels are read
//! as rectangular **regions** of one plane at one level.
//!
//! ## Units
//! All metadata uses fixed units: lengths and wavelengths in nanometres,
//! times in seconds. Field names carry the unit (`_nm`, `_s`).

mod errors;
mod reader;
mod types;

pub use errors::{Error, Result};
pub use reader::ImageReader;
pub use types::{
    ChannelMeta, Geometry, LevelMeta, ObjectiveMeta, PhysicalSize, PixelType, Plane, PlaneMeta,
    PlateMeta, Position, Region, Roi, SeriesMeta, Shape, WellField, WellMeta,
};

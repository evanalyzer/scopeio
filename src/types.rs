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

mod buffer;
mod channel;
mod format;
mod instrument;
mod meta;
mod pixel_type;
mod plane;
mod plane_meta;
mod plate;
mod position;
mod region;
mod request;
mod roi;

pub use buffer::{ImageBuf, OutputShape, PixelBuf, PixelSliceMut, PixelViewMut};
pub use channel::ChannelMeta;
pub use format::{FormatId, FormatInfo, OpenOptions, Parallelism};
pub use instrument::ObjectiveMeta;
pub use meta::{AssociatedKind, LevelMeta, OriginalMeta, PhysicalSize, SeriesMeta};
pub use pixel_type::PixelType;
pub use plane::Plane;
pub use plane_meta::PlaneMeta;
pub use plate::{PlateMeta, WellSample};
pub use position::Position;
pub use region::Region;
pub use request::{ChannelSelect, ColorOrder, Layout, OutputSpec, OutputType, RegionRequest, TileRequest};
pub use roi::{Geometry, Roi, Shape};

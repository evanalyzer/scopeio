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
//! ## Pixel data
//! Samples are returned row-major and interleaved (height x width x samples
//! per pixel), always in native byte order regardless of the file. Colour
//! samples are always R, G, B(, A) with straight alpha, see [`ColorModel`]. Use
//! [`ImageReaderExt::read_region_as`] for a typed `Vec<T>` or
//! [`ImageReaderExt::read_region_buf`] if the type is only known at runtime.
//! Both hand over to tensor libraries such as kornia-rs without copying,
//! see [`ImageReaderExt`].
//!
//! ## Units
//! All metadata uses fixed units: lengths and wavelengths in nanometres,
//! times in seconds. Field names carry the unit (`_nm`, `_s`).

mod errors;
mod reader;
mod types;

pub use errors::{Error, Result};
pub use reader::{ImageReader, ImageReaderExt};
pub use types::{
    ByteOrder, ChannelMeta, ColorModel, Geometry, LevelMeta, ObjectiveMeta, PhysicalSize, PixelBuf, PixelType,
    Plane, PlaneMeta, PlateMeta, Position, Region, Roi, Sample, SeriesMeta, Shape, WellField,
    WellMeta,
};

//! # request
//!
//! Read requests and output specification.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::Region;

/// Channels and samples a read returns.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChannelSelect {
    /// One channel as stored: gray, or a colour image with all its samples.
    Channel(u32),
    /// One sample of a colour channel as a gray image (e.g. only green),
    /// sample 0 being red.
    Sample { channel: u32, sample: u8 },
    /// Several channels stacked into one multi-channel output, e.g. a CHW
    /// tensor [3, H, W] from c = 0, 1, 2. A colour channel contributes all
    /// its samples.
    Channels(Vec<u32>),
    /// Selected samples of one colour channel, one output channel each, in
    /// the given order (stored order: 0 red, 1 green, 2 blue, 3 alpha;
    /// `ColorOrder` does not apply). Each native tile is decoded once, e.g.
    /// `samples: vec![0, 2]` with `Layout::Chw` gives a [2, H, W] red and
    /// blue tensor. 1 to 4 samples.
    Samples { channel: u32, samples: Vec<u8> },
}

impl Default for ChannelSelect {
    fn default() -> Self {
        ChannelSelect::Channel(0)
    }
}

/// Memory layout of output buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Layout {
    /// Height x width x channels, interleaved (OpenCV, kornia-rs, image).
    #[default]
    Hwc,
    /// Channels x height x width, planar (torch, ndarray).
    Chw,
}

/// Order of colour samples in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorOrder {
    #[default]
    Rgb,
    /// For OpenCV.
    Bgr,
}

/// Element type of the output.
///
/// The f32 types convert in the same pass as the read, so no native buffer
/// is allocated. Signed integers: `F32Normalized` and `F32SignificantBits`
/// divide by the positive maximum (int8 -128 -> -1.008, 127 -> 1.0);
/// `F32NormalizedShifted` shifts by the type's minimum first, like ImageJ
/// (int8 -128 -> 0.0, 127 -> 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum OutputType {
    /// The stored type, lossless.
    #[default]
    Native,
    /// The value as f32, not scaled.
    F32,
    /// The value divided by the maximum of the stored integer type (0..1
    /// for unsigned data); float data unchanged.
    F32Normalized,
    /// Linear rescale of [min, max] to 0..1, clamped.
    F32Range { min: f32, max: f32 },
    /// `value * scale + offset`, not clamped.
    F32Scaled { scale: f32, offset: f32 },
    /// The value divided by the maximum of the series' significant bits
    /// (`SeriesMeta::significant_bits`, else the type's bits): 12-bit data
    /// in u16 becomes 0..1. Float data unchanged.
    F32SignificantBits,
    /// `(value - type min) / (type max - type min)`: 0..1 for unsigned and
    /// signed integers alike. Float data unchanged.
    F32NormalizedShifted,
}

/// How output buffers are laid out and typed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputSpec {
    pub layout: Layout,
    pub color_order: ColorOrder,
    pub dtype: OutputType,
    /// Keep the alpha sample of RGBA images; dropped by default.
    pub keep_alpha: bool,
}

/// Request for one native tile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileRequest {
    pub series: usize,
    pub level: usize,
    pub z: u32,
    pub t: u32,
    pub channels: ChannelSelect,
    pub tile_x: u32,
    pub tile_y: u32,
    pub output: OutputSpec,
}

/// Request for any region of one plane.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegionRequest {
    pub series: usize,
    pub level: usize,
    pub z: u32,
    pub t: u32,
    pub channels: ChannelSelect,
    pub region: Region,
    pub output: OutputSpec,
}

impl RegionRequest {
    /// Request for `region` of channel 0, z 0, t 0 at `level` of `series`,
    /// with default output.
    pub fn new(series: usize, level: usize, region: Region) -> Self {
        Self { series, level, region, ..Default::default() }
    }
}

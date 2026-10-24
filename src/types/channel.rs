//! # channel
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Acquisition settings of one channel. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ChannelMeta {
    pub name: Option<String>,
    /// Display colour as RGBA.
    pub color: Option<[u8; 4]>,
    pub excitation_wavelength_nm: Option<f64>,
    pub emission_wavelength_nm: Option<f64>,
    /// Fluorophore name, e.g. "DAPI".
    pub fluor: Option<String>,
    /// Contrast method, e.g. "Fluorescence", "Brightfield", "DIC".
    pub contrast_method: Option<String>,
    pub pinhole_size_nm: Option<f64>,
    /// Exposure time configured for this channel. Actual per-plane values
    /// are in [`PlaneMeta::exposure_s`](crate::PlaneMeta::exposure_s).
    pub exposure_s: Option<f64>,
    pub detector_gain: Option<f64>,
    pub detector_offset: Option<f64>,
    /// Detector binning as (x, y).
    pub binning: Option<(u16, u16)>,
}

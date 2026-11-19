//! # instrument
//!
//! Microscope, objective, detector, light source and filter descriptions,
//! modelled on the `Instrument` part of the OME 2016-06 schema.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Microscope stand used for a series. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct MicroscopeMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// Stand type, e.g. "Upright", "Inverted", "Dissection".
    pub kind: Option<String>,
}

/// Objective used for a series. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ObjectiveMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub magnification: Option<f64>,
    pub numerical_aperture: Option<f64>,
    /// Immersion medium, e.g. "Oil", "Water", "Air".
    pub immersion: Option<String>,
    /// Optical correction, e.g. "PlanApo", "PlanFluor", "Achromat".
    pub correction: Option<String>,
    pub working_distance_nm: Option<f64>,
    /// Refractive index of the immersion medium used for this series.
    pub refractive_index: Option<f64>,
}

/// Detector (camera, PMT, ...) used for a channel. Settings that change
/// per acquisition, such as gain, are in [`ChannelMeta`](crate::ChannelMeta).
/// Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct DetectorMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// Detector type, e.g. "CCD", "EMCCD", "CMOS", "PMT", "APD".
    pub kind: Option<String>,
}

/// Light source used for a channel. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct LightSourceMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// Source class: "Laser", "Arc", "Filament", "LightEmittingDiode" or
    /// "GenericExcitationSource".
    pub kind: Option<String>,
    /// Subtype within the class, e.g. "SolidState" or "Gas" for a laser,
    /// "Xe" or "Hg" for an arc lamp.
    pub subtype: Option<String>,
    /// Laser medium, e.g. "Ar", "HeNe".
    pub laser_medium: Option<String>,
    /// Wavelength used for this channel (the laser line), falling back to
    /// the nominal wavelength of the laser.
    pub wavelength_nm: Option<f64>,
    /// Nominal output power of the source in milliwatts.
    pub power_mw: Option<f64>,
    /// Fraction of the light blocked for this channel, 0.0 to 1.0.
    pub attenuation: Option<f64>,
}

/// Optical filter in the excitation or emission path of a channel.
/// Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct FilterMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Filter type, e.g. "BandPass", "LongPass", "ShortPass", "NeutralDensity".
    pub kind: Option<String>,
    /// Wheel or slider the filter sits in.
    pub filter_wheel: Option<String>,
    /// Lower edge of the transmitted band.
    pub cut_in_nm: Option<f64>,
    /// Upper edge of the transmitted band.
    pub cut_out_nm: Option<f64>,
    /// Fraction of light transmitted in the band, 0.0 to 1.0.
    pub transmittance: Option<f64>,
}

/// Dichroic mirror (beam splitter) of a channel. Fields are `None` if
/// unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct DichroicMeta {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
}

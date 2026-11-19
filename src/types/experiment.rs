//! # experiment
//!
//! Imaging environment and ownership of a series, modelled on the
//! `ImagingEnvironment`, `Experimenter` and `Project` elements of the OME
//! 2016-06 schema.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-19
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Conditions around the sample during acquisition. Fields are `None` if
/// unknown.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub struct EnvironmentMeta {
    pub temperature_c: Option<f64>,
    /// CO₂ concentration as a fraction, 0.0 to 1.0 (0.05 is 5 %).
    pub co2_fraction: Option<f64>,
    /// Relative humidity as a fraction, 0.0 to 1.0.
    pub humidity_fraction: Option<f64>,
    pub air_pressure_mbar: Option<f64>,
}

/// Person who acquired or owns a series. Fields are `None` if unknown.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ExperimenterMeta {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub institution: Option<String>,
    /// Login name, e.g. in OMERO.
    pub user_name: Option<String>,
}

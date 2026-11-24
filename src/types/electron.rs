//! # electron
//!
//! Electron microscope settings of a series (TEM / STEM / SEM), with field
//! choice and meaning following the NeXus `NXem` application definition
//! and the HyperSpy metadata conventions (prose only).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Operating mode of an electron microscope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ElectronMicroscopyMode {
    /// Conventional (parallel-beam) TEM imaging.
    Tem,
    /// Scanning TEM: a focused probe scanned over the specimen.
    Stem,
    /// Electron diffraction (the back focal plane is imaged).
    Diffraction,
    /// Scanning electron microscopy: a focused probe scanned over the
    /// specimen surface, recording backscattered or secondary electrons.
    Sem,
}

/// Electron microscope settings of a series. Fields are `None` if unknown
/// or if their meaning in the file is not certain; the vendor values stay
/// in the original metadata.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ElectronMicroscopyMeta {
    /// Accelerating voltage of the electron gun in volts.
    pub accelerating_voltage_v: Option<f64>,
    /// Nominal (indicated) magnification of an imaging mode.
    pub magnification: Option<f64>,
    /// Camera length of a diffraction or STEM mode in millimetres.
    pub camera_length_mm: Option<f64>,
    /// Convergence semi-angle of the beam in milliradians.
    pub convergence_semi_angle_mrad: Option<f64>,
    pub mode: Option<ElectronMicroscopyMode>,
    /// Condenser spot size setting (instrument-specific index).
    pub spot_size: Option<u32>,
    /// Defocus in nanometres; negative is underfocus.
    pub defocus_nm: Option<f64>,
    /// Detector conversion: electrons per recorded count at the used
    /// accelerating voltage (dose calibration).
    pub electrons_per_count: Option<f64>,
    /// SEM working distance (final lens to specimen surface) in millimetres.
    pub working_distance_mm: Option<f64>,
    /// Beam current on the specimen in nanoamperes.
    pub probe_current_na: Option<f64>,
}

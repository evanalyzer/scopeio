//! # slide
//!
//! Backend shared by the TIFF-based slide formats (Aperio SVS, Hamamatsu
//! NDPI, Leica SCN, Ventana BIF, Akoya QPTIFF): series of pyramid levels
//! whose planes are TIFF images, and associated images (label, macro,
//! thumbnail) that are TIFF images too. The vendor modules classify the
//! IFDs and parse their metadata; reading is the TIFF core's.
//!
//! Provenance: TIFF 6.0 specification; see the vendor modules.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

pub(super) mod restart;

use restart::RestartJpeg;

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{AssociatedKind, Error, FormatInfo, OriginalMeta, Plane, PlaneMeta, Region, Result, SeriesMeta};

/// One plane of one level: the image and the samples it contributes.
#[derive(Debug)]
pub(super) struct PlaneImage {
    pub image: TiffImage,
    pub samples: Samples,
    /// Read by restart interval instead of by TIFF chunk.
    pub restart: Option<Box<RestartJpeg>>,
}

impl PlaneImage {
    /// All samples of `image` (gray or colour).
    pub fn whole(image: TiffImage) -> Self {
        let samples = if image.samples_out == 1 { Samples::One(0) } else { Samples::All };
        Self { image, samples, restart: None }
    }

    pub fn with_restart(mut self, restart: Option<RestartJpeg>) -> Self {
        self.restart = restart.map(Box::new);
        self
    }
}

/// A series: levels, each with its planes indexed by `c + size_c * (z + size_z * t)`.
#[derive(Debug, Default)]
pub(super) struct SlideSeries {
    pub levels: Vec<Vec<PlaneImage>>,
    pub plane_meta: Vec<PlaneMeta>,
}

/// A TIFF-based slide.
#[derive(Debug)]
pub(super) struct Slide {
    pub format: &'static FormatInfo,
    pub tiff: TiffFile,
    pub metas: Vec<SeriesMeta>,
    pub series: Vec<SlideSeries>,
    pub associated_kinds: Vec<AssociatedKind>,
    pub associated: Vec<TiffImage>,
    pub original: OriginalMeta,
}

impl Slide {
    pub fn new(format: &'static FormatInfo, tiff: TiffFile) -> Self {
        Slide {
            format,
            tiff,
            metas: Vec::new(),
            series: Vec::new(),
            associated_kinds: Vec::new(),
            associated: Vec::new(),
            original: OriginalMeta::default(),
        }
    }

    /// Adds an associated image unless one of the kind exists.
    pub fn add_associated(&mut self, kind: AssociatedKind, image: TiffImage) {
        if !self.associated_kinds.contains(&kind) {
            self.associated_kinds.push(kind);
            self.associated.push(image);
        }
    }

    /// Checks that every series has planes for all its levels and indices.
    pub fn validate(&self) -> Result<()> {
        if self.metas.is_empty() {
            return Err(Error::Corrupt(format!("{} without image", self.format.name)));
        }
        for (m, s) in self.metas.iter().zip(&self.series) {
            let planes = (m.size_c * m.size_z * m.size_t) as usize;
            if s.levels.len() != m.levels.len() || s.levels.iter().any(|l| l.len() != planes) {
                return Err(Error::Corrupt(format!("{} series with missing planes", self.format.name)));
            }
        }
        Ok(())
    }
}

impl Backend for Slide {
    fn format(&self) -> &'static FormatInfo {
        self.format
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        Ok(self.series[series].plane_meta.get(index).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let i = self
            .associated_kinds
            .iter()
            .position(|k| k == kind)
            .ok_or_else(|| Error::OutOfRange(format!("no associated image {kind:?}")))?;
        let img = &self.associated[i];
        let region = Region::new(0, 0, img.width as u64, img.height as u64);
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        native_image(region, img.samples_out, img.pixel_type, |sink| img.read_native(&self.tiff, region, samples, sink))
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let p = &self.series[series].levels[level][index];
        match &p.restart {
            Some(r) => r.read(&self.tiff, region, sink),
            None => p.image.read_native(&self.tiff, region, p.samples, sink),
        }
    }
}

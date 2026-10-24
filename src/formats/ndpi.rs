//! # ndpi
//!
//! Hamamatsu NDPI reader.
//!
//! NDPI is a classic little-endian TIFF with 64-bit extensions for files
//! above 4 GiB (see [`TiffFile::open_ndpi`]). Each pyramid level is one
//! JPEG stream; large levels carry restart markers whose offsets are in tag
//! 65426 (low 32 bits) and 65432 (high bits). Every restart interval of
//! one MCU row is decoded on its own as a tile: the stream's header with
//! the frame size patched to the tile, the interval's entropy data and an
//! end marker. Levels without restart markers are decoded whole. Tag 65421
//! (source lens) is -1 for the macro image and -2 for the map of scanned
//! regions; tag 65424 is the focal plane offset (several planes become z).
//!
//! Not supported: NDPIS sets (one file per fluorescence channel).
//!
//! Provenance: openslide.org Hamamatsu format page (prose, CC BY-SA 4.0,
//! `doc/hamamatsu-ndpi-vms/`); ITU-T T.81 (JPEG) for markers and restart
//! intervals; own analysis of sample files. OpenSlide's C code (LGPL) was
//! not consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::slide::restart::RestartJpeg;
use super::slide::{PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::TiffImage;
use super::tiff::{level_meta, original_meta, resolution, tiff_date};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, ObjectiveMeta, OpenOptions, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Ndpi, "Hamamatsu NDPI", "1");

/// NDPI file format version tag, present in every NDPI IFD.
pub(super) const TAG_VERSION: u16 = 65420;
const TAG_SOURCE_LENS: u16 = 65421;
const TAG_X_OFFSET: u16 = 65422;
const TAG_Y_OFFSET: u16 = 65423;
const TAG_Z_OFFSET: u16 = 65424;
const TAG_METADATA: u16 = 65449;

/// Opens an NDPI file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for an invalid structure and
/// [`Error::UnsupportedVariant`] if no level can be decoded.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open_ndpi(path, 4096)?;
    // Levels by focal plane; associated images.
    let mut planes: Vec<(i64, Vec<(usize, TiffImage)>)> = Vec::new();
    let mut associated = Vec::new();
    for (i, ifd) in tiff.ifds.iter().enumerate() {
        let Ok(image) = TiffImage::new(&tiff, ifd) else { continue };
        let lens = tiff.floats(ifd, TAG_SOURCE_LENS)?.and_then(|v| v.first().copied()).unwrap_or(0.0);
        if lens == -1.0 {
            associated.push((AssociatedKind::Macro, image));
            continue;
        }
        if lens == -2.0 {
            associated.push((AssociatedKind::Other("map".into()), image));
            continue;
        }
        let z = tiff.uint(ifd, TAG_Z_OFFSET)?.map(|v| v as u32 as i32 as i64).unwrap_or(0);
        match planes.iter_mut().find(|(pz, _)| *pz == z) {
            Some((_, list)) => list.push((i, image)),
            None => planes.push((z, vec![(i, image)])),
        }
    }
    if planes.is_empty() {
        return Err(Error::UnsupportedVariant("NDPI without readable level".into()));
    }
    // Focal planes in z order; each plane's levels from large to small.
    planes.sort_by_key(|(z, _)| *z);
    for (_, list) in &mut planes {
        list.sort_by_key(|(_, img)| std::cmp::Reverse(img.width));
    }
    let base_z = planes.iter().position(|(z, _)| *z == 0).unwrap_or(0);
    let levels_n = planes.iter().map(|(_, l)| l.len()).min().unwrap_or(0);
    let template = &planes[base_z].1;
    let first_ifd = template[0].0;
    let first = &template[0].1;
    let mut levels: Vec<Vec<PlaneImage>> = (0..levels_n).map(|_| Vec::new()).collect();
    for (_, list) in &planes {
        for (l, (ifd_index, image)) in list.iter().take(levels_n).enumerate() {
            if image.width != template[l].1.width || image.height != template[l].1.height {
                return Err(Error::UnsupportedVariant("NDPI focal planes with different level sizes".into()));
            }
            let restart = RestartJpeg::ndpi(&tiff, &tiff.ifds[*ifd_index], image)?;
            levels[l].push(PlaneImage::whole(image.clone()).with_restart(restart));
        }
    }
    let ifd = &tiff.ifds[first_ifd];
    let lens = tiff.floats(ifd, TAG_SOURCE_LENS)?.and_then(|v| v.first().copied()).filter(|v| *v > 0.0);
    let model = tiff.string(ifd, tag::MODEL)?;
    let level_metas = levels
        .iter()
        .map(|l| {
            let mut m = level_meta(&l[0].image, first.width);
            if let Some(r) = &l[0].restart {
                m.tile = Some((r.tile_w, r.tile_h));
            }
            m
        })
        .collect();
    let meta = SeriesMeta {
        acquisition_date: tiff.string(ifd, tag::DATE_TIME)?.and_then(|d| tiff_date(&d)),
        size_c: 1,
        size_z: planes.len() as u32,
        size_t: 1,
        pixel_type: first.pixel_type,
        samples_per_pixel: first.samples_out,
        levels: level_metas,
        physical_size: resolution(&tiff, ifd)?,
        channels: vec![ChannelMeta::default()],
        objective: lens.map(|m| ObjectiveMeta { magnification: Some(m), ..Default::default() }),
        ..Default::default()
    };
    let mut original = original_meta(&tiff, ifd)?;
    for (t, name) in [(TAG_SOURCE_LENS, "SourceLens"), (TAG_X_OFFSET, "XOffsetFromSlideCentre"), (TAG_Y_OFFSET, "YOffsetFromSlideCentre")] {
        if let Some(v) = tiff.floats(ifd, t)?.and_then(|v| v.first().copied()) {
            let v = if t == TAG_SOURCE_LENS { v } else { v as u32 as i32 as f64 };
            original.push((format!("hamamatsu.{name}"), v.to_string()));
        }
    }
    if let Some(m) = model {
        original.push(("hamamatsu.Model".into(), m));
    }
    if let Some(text) = tiff.string(ifd, TAG_METADATA)? {
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                original.push((format!("hamamatsu.{}", k.trim()), v.trim().to_string()));
            }
        }
    }
    // Planes are indexed by z (one channel).
    let mut slide = Slide::new(&FORMAT, tiff);
    slide.original.pairs = original;
    slide.metas.push(meta);
    slide.series.push(SlideSeries { levels, plane_meta: Vec::new() });
    for (kind, image) in associated {
        slide.add_associated(kind, image);
    }
    slide.validate()?;
    Ok(slide)
}


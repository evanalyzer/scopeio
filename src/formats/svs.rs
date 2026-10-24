//! # svs
//!
//! Aperio SVS reader: a tiled TIFF whose first IFD is the full-resolution
//! image, the second a stripped thumbnail, then tiled pyramid levels, and
//! optionally a label (NewSubfileType 1) and a macro image (9). The
//! ImageDescription holds `key = value` pairs separated by `|`, among them
//! the pixel size (`MPP`, µm) and the objective power (`AppMag`). Tiles
//! may be JPEG 2000 (Aperio compression 33003 YCbCr, 33005 RGB).
//!
//! Provenance: Aperio "Digital Slides and Third-Party Data Interchange"
//! (2008, in `doc/aperio-svs/`); openslide.org Aperio format page (prose,
//! CC BY-SA); own analysis of sample files. No GPL/LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::slide::{PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::TiffImage;
use super::tiff::{level_meta, original_meta};
use crate::{
    AssociatedKind, ChannelMeta, FormatId, FormatInfo, ObjectiveMeta, OpenOptions, PhysicalSize, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Svs, "Aperio SVS", "1");

/// Returns `true` if the first ImageDescription marks an Aperio slide.
pub(super) fn matches(description: &str) -> bool {
    description.starts_with("Aperio")
}

/// `key = value` pairs of an Aperio ImageDescription.
fn pairs(description: &str) -> Vec<(String, String)> {
    description
        .split('|')
        .skip(1)
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// ISO 8601 from `Date = MM/DD/YY` and `Time = HH:MM:SS`.
fn date(pairs: &[(String, String)]) -> Option<String> {
    let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    let d: Vec<u32> = get("Date")?.split('/').filter_map(|v| v.trim().parse().ok()).collect();
    let [month, day, year] = d[..] else { return None };
    let year = if year < 100 { 2000 + year } else { year };
    let time = get("Time").filter(|t| t.len() >= 8 && t.as_bytes()[2] == b':').map(|t| &t[..8]).unwrap_or("00:00:00");
    ((1..=12).contains(&month) && (1..=31).contains(&day)).then(|| format!("{year:04}-{month:02}-{day:02}T{time}"))
}

/// Opens an SVS file.
///
/// # Errors
///
/// Returns [`crate::Error::Corrupt`] if the full-resolution image cannot be
/// decoded.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open(path)?;
    let description = tiff.string(&tiff.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
    let base = TiffImage::new(&tiff, &tiff.ifds[0])?;
    let mut levels = vec![base];
    let mut associated: Vec<(AssociatedKind, TiffImage)> = Vec::new();
    for (i, ifd) in tiff.ifds.iter().enumerate().skip(1) {
        let Ok(image) = TiffImage::new(&tiff, ifd) else { continue };
        let subfile = tiff.uint(ifd, tag::NEW_SUBFILE_TYPE)?.unwrap_or(0);
        let desc = tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default().to_ascii_lowercase();
        let last = levels.last().expect("level 0");
        if image.tiled && subfile == 0 && image.width < last.width && image.spp == last.spp && image.pixel_type == last.pixel_type {
            levels.push(image);
        } else if subfile & 8 != 0 || desc.contains("macro") {
            associated.push((AssociatedKind::Macro, image));
        } else if subfile & 1 != 0 || desc.contains("label") {
            associated.push((AssociatedKind::Label, image));
        } else if i == 1 && !image.tiled {
            associated.push((AssociatedKind::Thumbnail, image));
        }
    }
    let kv = pairs(&description);
    let value = |k: &str| kv.iter().find(|(key, _)| key == k).and_then(|(_, v)| v.parse::<f64>().ok());
    let mpp_nm = value("MPP").filter(|v| *v > 0.0).map(|v| v * 1e3);
    let first = &levels[0];
    let meta = SeriesMeta {
        name: kv.iter().find(|(k, _)| k == "Filename").map(|(_, v)| v.clone()),
        description: Some(description.clone()),
        acquisition_date: date(&kv),
        size_c: 1,
        size_z: 1,
        size_t: 1,
        pixel_type: first.pixel_type,
        samples_per_pixel: first.samples_out,
        levels: levels.iter().map(|l| level_meta(l, first.width)).collect(),
        physical_size: PhysicalSize { x_nm: mpp_nm, y_nm: mpp_nm, z_nm: None },
        channels: vec![ChannelMeta::default()],
        objective: value("AppMag").map(|m| ObjectiveMeta { magnification: Some(m), ..Default::default() }),
        ..Default::default()
    };
    let mut slide = Slide::new(&FORMAT, tiff);
    slide.original.pairs = original_meta(&slide.tiff, &slide.tiff.ifds[0])?;
    slide.original.pairs.extend(kv.into_iter().map(|(k, v)| (format!("aperio.{k}"), v)));
    slide.original.documents.push(("ImageDescription".into(), description));
    slide.metas.push(meta);
    slide.series.push(SlideSeries { levels: levels.into_iter().map(|l| vec![PlaneImage::whole(l)]).collect(), plane_meta: Vec::new() });
    for (kind, image) in associated {
        slide.add_associated(kind, image);
    }
    slide.validate()?;
    Ok(slide)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_description() {
        let d = "Aperio Image Library v10.0.51\r\n46920x33014 [0,100 46000x32914] (256x256) JPEG/RGB Q=30|AppMag = 20|Date = 12/29/09|Time = 09:59:15|MPP = 0.4990";
        let kv = pairs(d);
        assert_eq!(kv[0], ("AppMag".into(), "20".into()));
        assert_eq!(date(&kv).as_deref(), Some("2009-12-29T09:59:15"));
        assert!(matches(d));
    }
}

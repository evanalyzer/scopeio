//! # qptiff
//!
//! Akoya (PerkinElmer) QPTIFF reader: whole-slide scans (`.qptiff`) and
//! inForm component images.
//!
//! Every image directory has an XML ImageDescription
//! (`PerkinElmer-QPI-ImageDescription`) whose `ImageType` classifies it:
//! the `FullResolution` images (one RGB image for brightfield, one gray
//! image per band for fluorescence or unmixed components) form level 0,
//! each run of `ReducedResolution` images with the same size one further
//! level, and `Thumbnail`, `Overview` and `Label` are associated images.
//! Band names and colours come from `Name` and `Color`, the pixel size from
//! the TIFF resolution tags.
//!
//! Provenance: Akoya "QPTIFF Specification", appendix A of the PhenoImager
//! HT user manual (`doc/perkinelmer-qptiff/`); own analysis of sample files.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use roxmltree::Document;

use super::slide::{PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::TiffImage;
use super::tiff::{level_meta, original_meta, resolution, tiff_date};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, ObjectiveMeta, OpenOptions, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Qptiff, "Akoya QPTIFF", "1");

/// Returns `true` for an Akoya / PerkinElmer QPI image description or
/// software tag.
pub(super) fn matches(software: &str, description: &str) -> bool {
    software.starts_with("PerkinElmer-QPI") || description.contains("<PerkinElmer-QPI-ImageDescription")
}

/// Fields of one image description.
#[derive(Debug, Default)]
struct Description {
    image_type: String,
    name: Option<String>,
    color: Option<[u8; 4]>,
    objective: Option<String>,
    exposure_s: Option<f64>,
}

fn parse(xml: &str) -> Description {
    let mut d = Description::default();
    let Ok(doc) = Document::parse_with_options(xml.trim_end_matches('\0'), roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() }) else {
        return d;
    };
    let root = doc.root_element();
    let text = |name: &str| {
        root.children().find(|c| c.has_tag_name(name)).and_then(|c| c.text()).map(str::trim).filter(|s| !s.is_empty()).map(String::from)
    };
    d.image_type = text("ImageType").unwrap_or_default();
    d.name = text("Name");
    d.color = text("Color").and_then(|c| {
        let v: Vec<u8> = c.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        (v.len() == 3).then(|| [v[0], v[1], v[2], 255])
    });
    d.objective = text("Objective");
    d.exposure_s = text("ExposureTime").and_then(|e| e.parse::<f64>().ok()).filter(|e| *e > 0.0).map(|us| us / 1e6);
    d
}

/// Opens a QPTIFF file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] if no full-resolution image can be decoded.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open(path)?;
    let mut full: Vec<(usize, TiffImage, Description)> = Vec::new();
    let mut reduced: Vec<Vec<TiffImage>> = Vec::new();
    let mut associated = Vec::new();
    for (i, ifd) in tiff.ifds.iter().enumerate() {
        let desc = parse(&tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default());
        let Ok(image) = TiffImage::new(&tiff, ifd) else { continue };
        match desc.image_type.as_str() {
            "FullResolution" => full.push((i, image, desc)),
            "ReducedResolution" => match reduced.last_mut() {
                Some(level) if level[0].width == image.width && level[0].height == image.height => level.push(image),
                _ => reduced.push(vec![image]),
            },
            "Thumbnail" => associated.push((AssociatedKind::Thumbnail, image)),
            "Overview" => associated.push((AssociatedKind::Macro, image)),
            "Label" => associated.push((AssociatedKind::Label, image)),
            // Older files without a type: the first images of one size are the bands.
            "" if reduced.is_empty() && full.first().is_none_or(|(_, f, _)| f.width == image.width && f.height == image.height) => {
                full.push((i, image, desc))
            }
            _ => {}
        }
    }
    let Some((first_ifd, first, _)) = full.first() else {
        return Err(Error::Corrupt("QPTIFF without full-resolution image".into()));
    };
    let (first_ifd, first) = (*first_ifd, first.clone());
    if full.iter().any(|(_, f, _)| !f.same_layout(&first) || f.width != first.width || f.height != first.height) {
        return Err(Error::UnsupportedVariant("QPTIFF bands with different layouts".into()));
    }
    let bands = full.len();
    let mut levels = vec![full.iter().map(|(_, img, _)| PlaneImage::whole(img.clone())).collect::<Vec<_>>()];
    let mut level_metas = vec![level_meta(&first, first.width)];
    for level in reduced {
        // Every band must have the level, smaller than the previous one.
        let prev = level_metas.last().expect("level 0").size_x;
        let same_samples = level.iter().all(|i| i.spp == first.spp && i.pixel_type == first.pixel_type && i.samples_out == first.samples_out);
        if level.len() != bands || level[0].width as u64 >= prev || !same_samples {
            break;
        }
        level_metas.push(level_meta(&level[0], first.width));
        levels.push(level.into_iter().map(PlaneImage::whole).collect());
    }
    let channels: Vec<ChannelMeta> = full
        .iter()
        .map(|(_, _, d)| ChannelMeta { name: d.name.clone(), color: d.color, exposure_s: d.exposure_s, ..Default::default() })
        .collect();
    let ifd = &tiff.ifds[first_ifd];
    let objective = full[0].2.objective.clone().map(|o| ObjectiveMeta { model: Some(o), ..Default::default() });
    let meta = SeriesMeta {
        acquisition_date: tiff.string(ifd, tag::DATE_TIME)?.and_then(|d| tiff_date(&d)),
        size_c: bands as u32,
        size_z: 1,
        size_t: 1,
        pixel_type: first.pixel_type,
        samples_per_pixel: first.samples_out,
        levels: level_metas,
        physical_size: resolution(&tiff, ifd)?,
        channels,
        objective,
        ..Default::default()
    };
    let description = tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
    let mut slide = Slide::new(&FORMAT, tiff);
    slide.original.pairs = original_meta(&slide.tiff, &slide.tiff.ifds[first_ifd])?;
    slide.original.documents.push(("ImageDescription".into(), description));
    slide.metas.push(meta);
    slide.series.push(SlideSeries { levels, plane_meta: Vec::new() });
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
        let d = parse(
            "<?xml version=\"1.0\" encoding=\"utf-16\"?><PerkinElmer-QPI-ImageDescription><DescriptionVersion>5</DescriptionVersion>\
             <ImageType>FullResolution</ImageType><ExposureTime>2500</ExposureTime><Name>DAPI</Name><Color>0,0,255</Color>\
             <Objective>20x</Objective></PerkinElmer-QPI-ImageDescription>",
        );
        assert_eq!((d.image_type.as_str(), d.name.as_deref(), d.color), ("FullResolution", Some("DAPI"), Some([0, 0, 255, 255])));
        assert_eq!((d.objective.as_deref(), d.exposure_s), (Some("20x"), Some(0.0025)));
        assert!(matches("PerkinElmer-QPI", ""));
    }
}

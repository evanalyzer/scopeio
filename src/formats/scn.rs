//! # scn
//!
//! Leica SCN reader (SCN400 slide scans).
//!
//! A BigTIFF whose first ImageDescription is an XML document: a
//! `collection` of `image`s, each with `dimension`s (pyramid level `r`,
//! channel `c`, focal plane `z`) naming the TIFF directory that holds it.
//! Every main image is one series; images covering the whole collection at
//! offset 0 are macro images and become associated images (the first
//! `Macro`, further ones `Other("macro N")`). The pixel size is the image's
//! view size in nanometres over its full-resolution size in pixels.
//!
//! Provenance: openslide.org Leica format page (prose, CC BY-SA 4.0) in
//! `doc/leica-scn/`; own analysis of sample files. OpenSlide's C code
//! (LGPL) was not consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use roxmltree::{Document, Node};

use super::slide::{PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::TiffImage;
use super::tiff::{level_meta, original_meta};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, ObjectiveMeta, OpenOptions, PhysicalSize, Position,
    Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Scn, "Leica SCN", "1").extensions(&["scn"]);

/// Returns `true` for the XML document of a Leica SCN file.
pub(super) fn matches(description: &str) -> bool {
    description.contains("http://www.leica-microsystems.com/scn/")
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn num<T: std::str::FromStr>(n: Node, attr: &str) -> Option<T> {
    n.attribute(attr)?.trim().parse().ok()
}

fn text(n: Option<Node>) -> Option<String> {
    n?.text().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// `#rrggbb` as RGBA.
fn color(s: &str) -> Option<[u8; 4]> {
    let v = u32::from_str_radix(s.trim().trim_start_matches('#'), 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255])
}

/// Opens an SCN file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid XML or missing directories.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open(path)?;
    let xml = tiff.string(&tiff.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
    let doc = Document::parse_with_options(&xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
        .map_err(|e| Error::Corrupt(format!("SCN XML: {e}")))?;
    let collection = child(doc.root_element(), "collection").ok_or_else(|| Error::Corrupt("SCN without collection".into()))?;
    let (cx, cy): (u64, u64) = (num(collection, "sizeX").unwrap_or(0), num(collection, "sizeY").unwrap_or(0));
    let (mut metas, mut series, mut associated) = (Vec::new(), Vec::new(), Vec::new());
    let mut macros = 0;
    for image in collection.children().filter(|c| c.tag_name().name() == "image") {
        let Some(pixels) = child(image, "pixels") else { continue };
        let view = child(image, "view");
        let (vx, vy): (u64, u64) = view.map(|v| (num(v, "sizeX").unwrap_or(0), num(v, "sizeY").unwrap_or(0))).unwrap_or((0, 0));
        let (ox, oy): (i64, i64) = view.map(|v| (num(v, "offsetX").unwrap_or(0), num(v, "offsetY").unwrap_or(0))).unwrap_or((0, 0));
        // Dimensions by (level, channel, plane).
        let dims: Vec<(u32, u32, u32, usize)> = pixels
            .children()
            .filter(|c| c.tag_name().name() == "dimension")
            .filter_map(|d| Some((num(d, "r").unwrap_or(0), num(d, "c").unwrap_or(0), num(d, "z").unwrap_or(0), num(d, "ifd")?)))
            .collect();
        if dims.is_empty() {
            continue;
        }
        let levels_n = dims.iter().map(|d| d.0).max().unwrap_or(0) + 1;
        let size_c = dims.iter().map(|d| d.1).max().unwrap_or(0) + 1;
        let size_z = dims.iter().map(|d| d.2).max().unwrap_or(0) + 1;
        let image_at = |ifd: usize| -> Result<TiffImage> {
            let i = tiff.ifds.get(ifd).ok_or_else(|| Error::Corrupt(format!("SCN directory {ifd} missing")))?;
            TiffImage::new(&tiff, i)
        };
        let is_macro = ox == 0 && oy == 0 && vx == cx && vy == cy && cx > 0;
        if is_macro {
            // The full-resolution dimension of channel 0 and plane 0.
            if let Some(d) = dims.iter().find(|d| d.0 == 0 && d.1 == 0 && d.2 == 0) {
                let kind = if macros == 0 { AssociatedKind::Macro } else { AssociatedKind::Other(format!("macro {}", macros + 1)) };
                associated.push((kind, image_at(d.3)?));
                macros += 1;
            }
            continue;
        }
        let mut levels: Vec<Vec<PlaneImage>> = Vec::new();
        let mut level_metas = Vec::new();
        for r in 0..levels_n {
            let mut planes = Vec::with_capacity((size_c * size_z) as usize);
            for z in 0..size_z {
                for c in 0..size_c {
                    let d = dims
                        .iter()
                        .find(|d| d.0 == r && d.1 == c && d.2 == z)
                        .ok_or_else(|| Error::Corrupt(format!("SCN image without dimension r={r} c={c} z={z}")))?;
                    planes.push(PlaneImage::whole(image_at(d.3)?));
                }
            }
            let base_w = levels.first().map(|l: &Vec<PlaneImage>| l[0].image.width).unwrap_or(planes[0].image.width);
            level_metas.push(level_meta(&planes[0].image, base_w));
            levels.push(planes);
        }
        let first = &levels[0][0].image;
        if levels.iter().flatten().any(|p| p.image.samples_out != first.samples_out || p.image.pixel_type != first.pixel_type) {
            return Err(Error::UnsupportedVariant("SCN image with mixed sample layouts".into()));
        }
        let settings = child(image, "scanSettings");
        let objective = settings.and_then(|s| child(s, "objectiveSettings")).and_then(|o| text(child(o, "objective")));
        let illumination = settings.and_then(|s| child(s, "illuminationSettings"));
        let channel_nodes: Vec<Node> = settings
            .and_then(|s| child(s, "channelSettings"))
            .map(|cs| cs.children().filter(|c| c.tag_name().name() == "channel").collect())
            .unwrap_or_default();
        let channels = (0..size_c)
            .map(|c| {
                let node = channel_nodes.iter().find(|n| num::<u32>(**n, "index") == Some(c));
                ChannelMeta {
                    name: node.and_then(|n| n.attribute("name")).map(String::from),
                    color: node.and_then(|n| n.attribute("rgb")).and_then(color),
                    // Microseconds.
                    exposure_s: node.and_then(|n| text(child(*n, "exposureTime"))).and_then(|e| e.parse::<f64>().ok()).map(|us| us / 1e6),
                    contrast_method: illumination.and_then(|i| text(child(i, "illuminationSource"))),
                    ..Default::default()
                }
            })
            .collect();
        let px = |view: u64, pixels: u32| (view > 0 && pixels > 0).then(|| view as f64 / pixels as f64);
        let spacing_z: Option<f64> = view.and_then(|v| num(v, "spacingZ")).filter(|s: &f64| *s > 0.0);
        let meta = SeriesMeta {
            name: image.attribute("name").map(String::from),
            acquisition_date: text(child(image, "creationDate")),
            size_c,
            size_z,
            size_t: 1,
            pixel_type: first.pixel_type,
            samples_per_pixel: first.samples_out,
            levels: level_metas,
            physical_size: PhysicalSize { x_nm: px(vx, first.width), y_nm: px(vy, first.height), z_nm: spacing_z.filter(|_| size_z > 1) },
            channels,
            objective: objective.map(|o| ObjectiveMeta {
                magnification: o.parse().ok(),
                numerical_aperture: illumination.and_then(|i| text(child(i, "numericalAperture"))).and_then(|n| n.parse().ok()),
                ..Default::default()
            }),
            stage_position: Some(Position { x_nm: Some(ox as f64), y_nm: Some(oy as f64), z_nm: None }),
            ..Default::default()
        };
        metas.push(meta);
        series.push(SlideSeries { levels, plane_meta: Vec::new() });
    }
    let mut slide = Slide::new(&FORMAT, tiff);
    slide.metas = metas;
    slide.series = series;
    for (kind, image) in associated {
        slide.add_associated(kind, image);
    }
    slide.original.pairs = original_meta(&slide.tiff, &slide.tiff.ifds[0])?;
    slide.original.documents.push(("SCN-XML".into(), xml));
    slide.validate()?;
    Ok(slide)
}

//! # philips
//!
//! Philips TIFF reader (export format of the Philips IntelliSite pathology
//! solution, "Philips DP").
//!
//! A tiled (Big)TIFF whose first ImageDescription is an XML document: a
//! `DataObject` of type `DPUfsImport` with `Attribute` key-value pairs
//! named after DICOM tags, holding one `DPScannedImage` per image (the
//! whole-slide image `WSI`, and `LABELIMAGE` / `MACROIMAGE` with the image
//! as base64 JPEG in `PIM_DP_IMAGE_DATA`). The tiled TIFF directories are
//! the pyramid levels, largest first. Their sizes in the TIFF tags are the
//! tile size times the tile count, padding included; the true size of a
//! reduced level is the full-resolution size divided by the level's
//! downsample, which is the ratio of its `DICOM_PIXEL_SPACING` (row and
//! column spacing in millimetres) to that of level 0. Tiles outside the
//! scanned regions may be missing (offset and byte count 0); they read as
//! white. Stripped directories whose description starts with `Label` or
//! `Macro` hold the same associated images as TIFF.
//!
//! Provenance: openslide.org Philips format page (prose, CC BY-SA 4.0) in
//! `doc/philips-tiff/`; tifffile (BSD-3-Clause, see
//! THIRD_PARTY_NOTICES.md) for the level size correction; own analysis of
//! the OpenSlide Philips test slides (CC0-1.0). OpenSlide's C code (LGPL)
//! was not consulted. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use roxmltree::{Document, Node};

use super::slide::{Associated, PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::TiffImage;
use super::tiff::original_meta;
use crate::{AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OpenOptions, PhysicalSize, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::PhilipsTiff, "Philips TIFF", "1").extensions(&["tiff", "tif"]);

/// Largest XML document read from the first ImageDescription.
const MAX_XML: usize = 256 << 20;

/// Returns `true` for a Philips TIFF: the Software tag starts with
/// `Philips` and the description is a `DPUfsImport` data object.
pub(super) fn matches(software: &str, description: &str) -> bool {
    software.starts_with("Philips") && description.contains("<DataObject") && description.contains("DPUfsImport")
}

fn attributes<'a, 'i>(object: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    object.children().filter(|c| c.has_tag_name("Attribute"))
}

fn attribute<'a, 'i>(object: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    attributes(object).find(|a| a.attribute("Name") == Some(name))
}

fn text<'a>(object: Node<'a, '_>, name: &str) -> Option<&'a str> {
    attribute(object, name)?.text().map(str::trim)
}

/// Data objects in the `Array` of an `IDataObjectArray` attribute.
fn objects<'a, 'i>(object: Node<'a, 'i>, name: &str) -> Vec<Node<'a, 'i>> {
    attribute(object, name)
        .and_then(|a| a.children().find(|c| c.has_tag_name("Array")))
        .map(|array| array.children().filter(|c| c.has_tag_name("DataObject")).collect())
        .unwrap_or_default()
}

/// Values of a `"a" "b"` string or number array.
fn quoted(s: &str) -> Vec<&str> {
    s.split('"').map(str::trim).filter(|v| !v.is_empty()).collect()
}

/// `DICOM_PIXEL_SPACING` as (row, column) spacing in millimetres.
fn spacing(object: Node) -> Option<(f64, f64)> {
    let v: Vec<f64> = quoted(text(object, "DICOM_PIXEL_SPACING")?).iter().filter_map(|v| v.parse().ok()).collect();
    match v[..] {
        [row, col, ..] if row > 0.0 && col > 0.0 && row.is_finite() && col.is_finite() => Some((row, col)),
        _ => None,
    }
}

/// ISO 8601 from a DICOM date-time `YYYYMMDDHHMMSS[.ffffff]`.
fn dicom_datetime(s: &str) -> Option<String> {
    let d = s.trim().trim_matches('"');
    if d.len() < 8 || !d.is_char_boundary(8) || !d[..8].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut out = format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]);
    if d.len() >= 14 && d.is_char_boundary(14) && d[8..14].bytes().all(|b| b.is_ascii_digit()) {
        out += &format!("T{}:{}:{}", &d[8..10], &d[10..12], &d[12..14]);
    }
    Some(out)
}

/// Decodes standard base64, ignoring whitespace and invalid characters.
fn base64(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Key-value attributes of `object` as `philips.<prefix><name>` pairs;
/// image data and nested arrays are left out.
fn pairs(object: Node, prefix: &str, out: &mut Vec<(String, String)>) {
    for a in attributes(object) {
        let Some(name) = a.attribute("Name") else { continue };
        if name == "PIM_DP_IMAGE_DATA" || a.children().any(|c| c.has_tag_name("Array")) {
            continue;
        }
        let value = a.text().map(str::trim).unwrap_or_default();
        out.push((format!("philips.{prefix}{name}"), value.to_string()));
    }
}

/// True size of a reduced level: the full-resolution size over the
/// downsample, kept within the padded TIFF size (one tile of padding at
/// most).
fn true_size(padded: u32, tile: u32, full: u32, downsample: f64) -> u32 {
    let n = (full as f64 / downsample.round().max(1.0)).floor() as u32;
    let min = padded.saturating_sub(tile.saturating_sub(1)).max(1);
    n.clamp(min, padded)
}

/// Opens a Philips TIFF file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid XML or a file without tiled
/// full-resolution image.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open(path)?;
    let xml = tiff.string(&tiff.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
    if xml.len() > MAX_XML {
        return Err(Error::Corrupt("Philips XML larger than 256 MiB".into()));
    }
    let doc = Document::parse_with_options(&xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
        .map_err(|e| Error::Corrupt(format!("Philips XML: {e}")))?;
    let root = doc.root_element();
    if !root.has_tag_name("DataObject") || root.attribute("ObjectType") != Some("DPUfsImport") {
        return Err(Error::Corrupt("Philips XML without DPUfsImport object".into()));
    }
    let scanned = objects(root, "PIM_DP_SCANNED_IMAGES");
    let image_type = |o: &Node| text(*o, "PIM_DP_IMAGE_TYPE").unwrap_or_default().to_string();
    let wsi = scanned.iter().find(|o| image_type(o) == "WSI").copied();

    // Pixel spacing per level, by representation number.
    let mut spacings: Vec<(u32, (f64, f64))> = wsi
        .map(|w| objects(w, "PIIM_PIXEL_DATA_REPRESENTATION_SEQUENCE"))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| Some((text(r, "PIIM_PIXEL_DATA_REPRESENTATION_NUMBER")?.parse().ok()?, spacing(r)?)))
        .collect();
    spacings.sort_by_key(|s| s.0);

    // Tiled directories are the levels; Label / Macro directories are
    // associated images.
    let mut levels: Vec<TiffImage> = Vec::new();
    let mut tiff_associated: Vec<(AssociatedKind, TiffImage)> = Vec::new();
    for ifd in &tiff.ifds {
        let desc = tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
        let Ok(image) = TiffImage::new(&tiff, ifd) else { continue };
        if desc.starts_with("Label") {
            tiff_associated.push((AssociatedKind::Label, image));
        } else if desc.starts_with("Macro") {
            tiff_associated.push((AssociatedKind::Macro, image));
        } else if image.tiled {
            match levels.last() {
                None => levels.push(image),
                Some(last) if image.width < last.width && image.spp == last.spp && image.pixel_type == last.pixel_type => levels.push(image),
                Some(_) => {}
            }
        }
    }
    if levels.is_empty() {
        return Err(Error::Corrupt("Philips TIFF without tiled image".into()));
    }

    // Level sizes without padding; downsamples from the pixel spacings.
    let (full_w, full_h) = (levels[0].width, levels[0].height);
    let base = spacings.first().map(|s| s.1);
    let mut level_metas = Vec::with_capacity(levels.len());
    for (i, img) in levels.iter_mut().enumerate() {
        img.empty_fill = 255;
        let ratio = match (base, spacings.get(i)) {
            (Some((r0, c0)), Some((_, (r, c)))) => Some((c / c0, r / r0)),
            _ => None,
        };
        // Spacings are rounded to six digits; snap to whole downsamples.
        let snap = |v: f64| if (v - v.round()).abs() < 0.01 * v { v.round() } else { v };
        let (dx, dy) = ratio.map(|(x, y)| (snap(x), snap(y))).unwrap_or((full_w as f64 / img.width as f64, full_h as f64 / img.height as f64));
        if i > 0 && ratio.is_some() {
            img.width = true_size(img.width, img.chunk_w, full_w, dx);
            img.height = true_size(img.height, img.chunk_h, full_h, dy);
        }
        level_metas.push(LevelMeta {
            size_x: img.width as u64,
            size_y: img.height as u64,
            tile: Some((img.chunk_w, img.chunk_h)),
            downsample: if i == 0 { 1.0 } else { dx },
        });
    }

    let px = base.map(|(row, col)| (col * 1e6, row * 1e6));
    let first = &levels[0];
    let date = text(root, "DICOM_ACQUISITION_DATETIME").or_else(|| wsi.and_then(|w| text(w, "DICOM_ACQUISITION_DATETIME"))).and_then(dicom_datetime);
    let barcode = text(root, "PIM_DP_UFS_BARCODE").map(|b| String::from_utf8_lossy(&base64(b)).trim().to_string());
    let meta = SeriesMeta {
        name: barcode.clone().filter(|b| !b.is_empty()),
        acquisition_date: date,
        size_c: 1,
        size_z: 1,
        size_t: 1,
        pixel_type: first.pixel_type,
        samples_per_pixel: first.samples_out,
        levels: level_metas,
        physical_size: PhysicalSize { x_nm: px.map(|p| p.0), y_nm: px.map(|p| p.1), z_nm: None },
        channels: vec![ChannelMeta::default()],
        ..Default::default()
    };

    let mut slide = Slide::new(&FORMAT, tiff);
    slide.original.pairs = original_meta(&slide.tiff, &slide.tiff.ifds[0])?;
    pairs(root, "", &mut slide.original.pairs);
    if let Some(b) = &barcode {
        slide.original.pairs.push(("philips.barcode".into(), b.clone()));
    }
    if let Some(w) = wsi {
        pairs(w, "", &mut slide.original.pairs);
        for (i, r) in objects(w, "PIIM_PIXEL_DATA_REPRESENTATION_SEQUENCE").into_iter().enumerate() {
            pairs(r, &format!("PIIM_PIXEL_DATA_REPRESENTATION_SEQUENCE[{i}]."), &mut slide.original.pairs);
        }
    }
    slide.original.documents.push(("Philips-XML".into(), xml.clone()));
    slide.metas.push(meta);
    slide.series.push(SlideSeries { levels: levels.into_iter().map(|l| vec![PlaneImage::whole(l)]).collect(), plane_meta: Vec::new() });

    // Associated images: the XML JPEGs first, TIFF directories otherwise.
    for o in &scanned {
        let kind = match image_type(o).as_str() {
            "LABELIMAGE" => AssociatedKind::Label,
            "MACROIMAGE" => AssociatedKind::Macro,
            _ => continue,
        };
        if let Some(data) = text(*o, "PIM_DP_IMAGE_DATA").map(base64).filter(|d| d.starts_with(&[0xFF, 0xD8])) {
            slide.add(kind, Associated::Jpeg(data));
        }
    }
    for (kind, image) in tiff_associated {
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
    fn level_sizes_lose_their_padding() {
        // Philips-1 level 4: 3072 x 2560 stored, 45056 x 35840 / 16.
        assert_eq!(true_size(3072, 512, 45056, 16.0), 2816);
        assert_eq!(true_size(2560, 512, 35840, 16.0), 2240);
        // Never below one tile of padding, never above the stored size.
        assert_eq!(true_size(1024, 512, 100, 16.0), 513);
        assert_eq!(true_size(512, 512, 45056, 2.0), 512);
    }

    #[test]
    fn parses_values() {
        assert_eq!(quoted("\"0.000227273\" \"0.000227273\""), ["0.000227273", "0.000227273"]);
        assert_eq!(base64("MzMxMTk0MA=="), b"3311940");
        assert_eq!(dicom_datetime("20151209172929.000000").as_deref(), Some("2015-12-09T17:29:29"));
        assert_eq!(dicom_datetime("2015").as_deref(), None);
        assert!(matches("Philips DP v1.0", "<DataObject ObjectType=\"DPUfsImport\">"));
        assert!(!matches("Aperio", "<DataObject ObjectType=\"DPUfsImport\">"));
    }
}

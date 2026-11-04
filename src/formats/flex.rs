//! # flex
//!
//! PerkinElmer / Evotec Opera Flex reader (`.flex`).
//!
//! A Flex file is a TIFF holding one well of a plate: one page per image and an XML
//! document in tag 65200 (`Root` with `Arrays` and `FLEX`). `FLEX.Well`
//! gives the well coordinate and an `Images` list: per image the page
//! (`BufferNo`), `CameraRef`, `ExposureNo`, the field (`Sublayout`), the
//! plane (`Stack`), positions (`PositionX/Y/Z` in metres), the time offset
//! (`MeasurementTimeOffset`), the exposure time and the pixel size
//! (`ImageResolutionX/Y`). The `.flex` files of one folder are the wells of
//! a plate: opening any of them opens all, one series per well and field
//! in row, column and field order; channels are the distinct (exposure,
//! camera) pairs in order of appearance, z the planes. Files are opened for
//! each read, so large plates keep no handles.
//! Images with a `CompressionType` (LuraWave) are not supported.
//!
//! Provenance: own analysis of the Flex files of IDR studies on the OME
//! sample server (`Flex/idr0007`, CC BY-NC-SA 3.0, used locally only);
//! checked against Bio-Formats as a black box. No public specification
//! exists. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::BTreeSet;
use std::path::Path;

use roxmltree::{Document, Node};

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta,
    PlateMeta, Position, Region, Result, SeriesMeta, WellSample,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Flex, "Opera Flex", "1").extensions(&["flex"]);

/// TIFF tag holding the Flex XML.
const TAG_FLEX_XML: u16 = 65200;

/// Returns `true` for a TIFF file with the Flex XML tag.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    TiffFile::matches(head) && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("flex")) && TiffFile::open_first(path).is_ok_and(|t| t.ifds[0].has(TAG_FLEX_XML))
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn descendant<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.descendants().find(|c| c.tag_name().name() == name)
}

fn text(n: Node, name: &str) -> Option<String> {
    child(n, name).and_then(|c| c.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn num(n: Node, name: &str) -> Option<f64> {
    text(n, name)?.parse().ok().filter(|v: &f64| v.is_finite())
}

/// A value in metres (attribute `Unit`, default metres) as nanometres.
fn metres_nm(n: Node, name: &str) -> Option<f64> {
    let c = child(n, name)?;
    let v: f64 = c.text()?.trim().parse().ok()?;
    let f = match c.attribute("Unit").unwrap_or("m") {
        "m" => 1e9,
        "mm" => 1e6,
        "um" | "µm" => 1e3,
        "nm" => 1.0,
        _ => return None,
    };
    v.is_finite().then_some(v * f)
}

/// Opera Flex well.
#[derive(Debug)]
pub(super) struct Flex {
    /// Well file of each series.
    files: Vec<std::path::PathBuf>,
    /// Page of each plane by `c + size_c * z`, per series.
    pages: Vec<Vec<Option<usize>>>,
    plane_meta: Vec<Vec<PlaneMeta>>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    original: OriginalMeta,
}

impl Flex {
    /// Opens the plate of a Flex file: all `.flex` files of its folder.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML and
    /// [`Error::UnsupportedVariant`] for compressed (LuraWave) images.
    pub fn open(path: &Path) -> Result<Self> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("flex")) && p.is_file()).collect())
            .unwrap_or_default();
        if !paths.iter().any(|p| p == path) {
            paths.push(path.to_path_buf());
        }
        let mut wells: Vec<((u32, u32), Flex)> = Vec::new();
        for p in &paths {
            match Self::open_well(p) {
                Ok(w) => wells.push(w),
                // Other wells may be unreadable; the opened one may not.
                Err(e) if p == path => return Err(e),
                Err(_) => {}
            }
        }
        wells.sort_by_key(|(rc, _)| *rc);
        let mut plate = Flex { files: Vec::new(), pages: Vec::new(), plane_meta: Vec::new(), metas: Vec::new(), plates: Vec::new(), original: OriginalMeta::default() };
        let mut samples = Vec::new();
        for (_, w) in wells {
            let base = plate.metas.len();
            for mut s in w.plates.into_iter().flat_map(|p| p.wells) {
                s.series += base;
                samples.push(s);
            }
            if plate.original.documents.is_empty() {
                plate.original = w.original;
                plate.plates = vec![PlateMeta { name: None, rows: 0, cols: 0, wells: Vec::new() }];
            }
            plate.files.extend(w.files);
            plate.pages.extend(w.pages);
            plate.plane_meta.extend(w.plane_meta);
            plate.metas.extend(w.metas);
        }
        if let Some(p) = plate.plates.first_mut() {
            p.wells = samples;
        }
        Ok(plate)
    }

    /// One well file: its coordinate (row, column) and series.
    fn open_well(path: &Path) -> Result<((u32, u32), Self)> {
        let tiff = TiffFile::open(path)?;
        let xml = tiff.string(&tiff.ifds[0], TAG_FLEX_XML)?.ok_or_else(|| Error::Corrupt("Flex file without XML".into()))?;
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("Flex XML: {e}")))?;
        let root = doc.root_element();
        let well = descendant(root, "Well").ok_or_else(|| Error::Corrupt("Flex XML without Well".into()))?;
        let images: Vec<Node> = child(well, "Images").map(|i| i.children().filter(|n| n.tag_name().name() == "Image").collect()).unwrap_or_default();
        if images.is_empty() {
            return Err(Error::Corrupt("Flex XML without images".into()));
        }
        // Per image: page, channel key, field, plane.
        struct Entry {
            page: usize,
            channel: (String, String),
            field: u32,
            plane: u32,
            meta: PlaneMeta,
        }
        let mut entries = Vec::new();
        for (k, im) in images.iter().enumerate() {
            if text(*im, "CompressionType").is_some() {
                return Err(Error::UnsupportedVariant("Flex images with LuraWave compression".into()));
            }
            let page = im.attribute("BufferNo").and_then(|v| v.parse().ok()).unwrap_or(k);
            let position = Position { x_nm: metres_nm(*im, "PositionX"), y_nm: metres_nm(*im, "PositionY"), z_nm: metres_nm(*im, "PositionZ") };
            entries.push(Entry {
                page,
                channel: (text(*im, "ExposureNo").unwrap_or_default(), text(*im, "CameraRef").unwrap_or_default()),
                field: num(*im, "Sublayout").map_or(1, |v| v.max(1.0) as u32),
                plane: num(*im, "Stack").map_or(1, |v| v.max(1.0) as u32),
                meta: PlaneMeta {
                    delta_t_s: num(*im, "MeasurementTimeOffset"),
                    exposure_s: num(*im, "CameraExposureTime").filter(|v| *v > 0.0),
                    position: (position != Position::default()).then_some(position),
                },
            });
        }
        let mut channels: Vec<(String, String)> = Vec::new();
        for e in &entries {
            if !channels.contains(&e.channel) {
                channels.push(e.channel.clone());
            }
        }
        let fields: Vec<u32> = entries.iter().map(|e| e.field).collect::<BTreeSet<_>>().into_iter().collect();
        let planes: Vec<u32> = entries.iter().map(|e| e.plane).collect::<BTreeSet<_>>().into_iter().collect();
        let (size_c, size_z) = (channels.len(), planes.len());
        // Geometry from the first image's page.
        let first = tiff.ifds.get(entries[0].page).ok_or_else(|| Error::Corrupt("Flex image page missing".into()))?;
        let img = TiffImage::new(&tiff, first)?;
        let (width, height, pixel_type) = (img.width as u64, img.height as u64, img.pixel_type);
        let first_image = images[0];
        let objective = text(first_image, "ObjectiveRef").and_then(|id| root.descendants().find(|o| o.tag_name().name() == "Objective" && o.attribute("ID") == Some(id.as_str()))).map(|o| ObjectiveMeta {
            magnification: num(o, "Magnification").filter(|v| *v > 0.0),
            numerical_aperture: num(o, "NumAperture").filter(|v| *v > 0.0),
            ..Default::default()
        });
        let (row, col) = child(well, "WellCoordinate").map_or((1, 1), |w| (w.attribute("Row").and_then(|v| v.parse().ok()).unwrap_or(1u32), w.attribute("Col").and_then(|v| v.parse().ok()).unwrap_or(1u32)));
        let well_name = format!("{}{:02}", (b'A' + (row.saturating_sub(1) % 26) as u8) as char, col);
        let mut pages = Vec::new();
        let mut plane_meta = Vec::new();
        let mut metas = Vec::new();
        let mut wells = Vec::new();
        for &f in &fields {
            let mut p = vec![None; size_c * size_z];
            let mut pm = vec![PlaneMeta::default(); size_c * size_z];
            let mut ch_meta = vec![ChannelMeta::default(); size_c];
            for e in entries.iter().filter(|e| e.field == f) {
                let c = channels.iter().position(|k| *k == e.channel).expect("listed channel");
                let z = planes.binary_search(&e.plane).expect("listed plane");
                p[c + size_c * z] = Some(e.page);
                pm[c + size_c * z] = e.meta.clone();
                ch_meta[c].exposure_s = ch_meta[c].exposure_s.or(e.meta.exposure_s);
                ch_meta[c].name = Some(format!("{} {}", e.channel.1, e.channel.0).trim().to_string()).filter(|s| !s.is_empty());
            }
            let first = entries.iter().find(|e| e.field == f);
            wells.push(WellSample { row: row.saturating_sub(1), col: col.saturating_sub(1), field: metas.len() as u32, series: metas.len(), position: first.and_then(|e| e.meta.position) });
            metas.push(SeriesMeta {
                name: Some(format!("well {well_name} field {f}")),
                acquisition_date: descendant(root, "StartTime").and_then(|n| n.text()).map(|t| t.trim().to_string()),
                size_c: size_c as u32,
                size_z: size_z as u32,
                size_t: 1,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: metres_nm(first_image, "ImageResolutionX"), y_nm: metres_nm(first_image, "ImageResolutionY"), z_nm: None },
                channels: ch_meta,
                objective: objective.clone(),
                stage_position: first.and_then(|e| e.meta.position),
                ..Default::default()
            });
            pages.push(p);
            plane_meta.push(pm);
        }
        let plate = descendant(root, "Plate");
        let plates = vec![PlateMeta { name: plate.and_then(|p| text(p, "Barcode").or_else(|| text(p, "PlateName"))), rows: 0, cols: 0, wells }];
        let mut original = OriginalMeta::default();
        original.documents.push(("Flex XML".into(), xml.clone()));
        let files = vec![path.to_path_buf(); metas.len()];
        Ok(((row, col), Flex { files, pages, plane_meta, metas, plates, original }))
    }
}

impl Backend for Flex {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let i = (plane.c + self.metas[series].size_c * plane.z) as usize;
        Ok(self.plane_meta[series].get(i).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * plane.z) as usize;
        let Some(page) = self.pages[series][i] else {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let tiff = TiffFile::open(&self.files[series])?;
        let ifd = tiff.ifds.get(page).ok_or_else(|| Error::Corrupt(format!("Flex page {page} missing")))?;
        let img = TiffImage::new(&tiff, ifd)?;
        if (img.width as u64, img.height as u64, img.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt(format!("Flex page {page} differs from the first image")));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

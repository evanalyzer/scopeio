//! # ome
//!
//! OME-TIFF: series, dimensions and metadata from the OME-XML in the first
//! IFD, with planes spread over one or more TIFF files.
//!
//! Supports `TiffData` mapping with `IFD`, `First*` and `PlaneCount`,
//! multi-file sets referenced by `UUID`/`FileName`, partial metadata with
//! `BinaryOnly` pointing to a master file, companion `.companion.ome` files
//! and sub-resolutions stored as SubIFDs.
//!
//! Reference: OME-TIFF specification and the OME 2016-06 schema (see
//! `doc/ome-tiff/`, `doc/ome-xml/`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use roxmltree::{Document, Node};

use super::ifd::TiffFile;
use super::image::Samples;
use super::{PlaneRef, Series, Slot, level_meta};
use crate::{
    ChannelMeta, Error, Geometry, ObjectiveMeta, OriginalMeta, PhysicalSize, PixelType, PlaneMeta, PlateMeta,
    Position, Result, Roi, SeriesMeta, Shape, WellSample,
};

/// A TIFF file opened on first use.
#[derive(Debug)]
pub(super) struct LazyFile {
    path: PathBuf,
    file: OnceLock<std::result::Result<TiffFile, String>>,
}

impl LazyFile {
    pub fn ready(tiff: TiffFile) -> Self {
        let file = OnceLock::new();
        let _ = file.set(Ok(tiff));
        Self { path: PathBuf::new(), file }
    }

    pub fn lazy(path: PathBuf) -> Self {
        Self { path, file: OnceLock::new() }
    }

    pub fn get(&self) -> Result<&TiffFile> {
        self.file
            .get_or_init(|| TiffFile::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display())))
            .as_ref()
            .map_err(|e| Error::Corrupt(e.clone()))
    }

    fn into_tiff(self) -> Option<TiffFile> {
        self.file.into_inner().and_then(|r| r.ok())
    }
}

/// Where the OME-XML of an opened set comes from, to read it again for
/// [`original`].
#[derive(Debug)]
pub(super) struct Source {
    dir: PathBuf,
    /// The companion file, if the set was opened from one; otherwise the
    /// XML is in the first IFD of file 0.
    companion: Option<PathBuf>,
}

/// Files, series, plates and metadata source of an opened set.
pub(super) type Opened = (Vec<LazyFile>, Vec<Series>, Vec<PlateMeta>, Source);

/// Opens an OME-TIFF from its first file. On failure the TIFF is handed
/// back so that it can be read as a plain TIFF.
pub(super) fn open(path: &Path, tiff: TiffFile, xml: &str) -> std::result::Result<Opened, (TiffFile, Error)> {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut files = vec![LazyFile::ready(tiff)];
    // The opened file's own UUID, from its (possibly partial) metadata.
    let own_uuid = parse(xml).ok().and_then(|d| d.root_element().attribute("UUID").map(String::from));
    let result = master_xml(&dir, xml)
        .and_then(|master| build(&dir, Some((&name, own_uuid.as_deref())), &master, &mut files));
    match result {
        Ok((series, plates)) => Ok((files, series, plates, Source { dir, companion: None })),
        Err(e) => {
            let tiff = files.swap_remove(0).into_tiff().expect("first file is open");
            Err((tiff, e))
        }
    }
}

/// Opens an OME-XML companion file (`.companion.ome`) of a multi-file set.
pub(super) fn open_companion(path: &Path) -> Result<Opened> {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let xml = std::fs::read_to_string(path)?;
    let mut files = Vec::new();
    let (series, plates) = build(&dir, None, &xml, &mut files)?;
    Ok((files, series, plates, Source { dir, companion: Some(path.to_path_buf()) }))
}

/// Reads the OME-XML again and adds it with selected values of `series`.
pub(super) fn original(source: &Source, files: &[LazyFile], series: usize, meta: &mut OriginalMeta) -> Result<()> {
    let xml = match &source.companion {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let tiff = files[0].get()?;
            let own = tiff.string(&tiff.ifds[0], super::ifd::tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
            master_xml(&source.dir, &own)?
        }
    };
    {
        let doc = parse(&xml)?;
        let root = doc.root_element();
        if let Some(creator) = root.attribute("Creator") {
            meta.pairs.push(("OME Creator".into(), creator.into()));
        }
        if let Some(image) = children(root, "Image").nth(series) {
            for a in ["ID", "Name"] {
                if let Some(v) = image.attribute(a) {
                    meta.pairs.push((format!("OME Image {a}"), v.into()));
                }
            }
        }
    }
    meta.documents.push(("OME-XML".into(), xml));
    Ok(())
}

/// Returns `true` if `head` looks like the start of an OME-XML document.
pub(super) fn is_ome_xml(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start_matches('\u{feff}').trim_start();
    t.starts_with("<?xml") && text.contains("<OME") || t.starts_with("<OME")
}

/// The full metadata: `xml` itself, or the master file it points to with
/// `BinaryOnly`.
fn master_xml(dir: &Path, xml: &str) -> Result<String> {
    let doc = parse(xml)?;
    match child(doc.root_element(), "BinaryOnly").and_then(|b| b.attribute("MetadataFile")) {
        Some(file) => {
            let path = dir.join(file);
            if path.extension().is_some_and(|e| e == "ome" || e == "xml") {
                Ok(std::fs::read_to_string(&path)?)
            } else {
                let tiff = TiffFile::open(&path)?;
                tiff.string(&tiff.ifds[0], super::ifd::tag::IMAGE_DESCRIPTION)?
                    .ok_or_else(|| Error::Corrupt(format!("{}: no OME-XML", path.display())))
            }
        }
        None => Ok(xml.to_string()),
    }
}

fn parse(xml: &str) -> Result<Document<'_>> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    Document::parse_with_options(xml, options).map_err(|e| Error::Corrupt(format!("OME-XML: {e}")))
}

/// Registry of the files of a set; index 0 is the opened file, if any.
struct Files<'a> {
    dir: &'a Path,
    by_name: HashMap<String, usize>,
    by_uuid: HashMap<String, usize>,
    list: &'a mut Vec<LazyFile>,
}

impl Files<'_> {
    /// Index of the file a `TiffData` element refers to; `None` if unknown.
    fn resolve(&mut self, td: Node) -> Option<usize> {
        let Some(uuid) = child(td, "UUID") else {
            // Without UUID the data is in the file holding the metadata.
            return if self.list.is_empty() { None } else { Some(0) };
        };
        let id = uuid.text().unwrap_or("").trim().to_string();
        // The UUID identifies the file; file names may be outdated after renaming.
        if let Some(&i) = self.by_uuid.get(&id) {
            return Some(i);
        }
        if let Some(name) = uuid.attribute("FileName") {
            if let Some(&i) = self.by_name.get(name) {
                return Some(i);
            }
            let path = self.dir.join(name);
            if !path.exists() {
                return None;
            }
            self.list.push(LazyFile::lazy(path));
            let i = self.list.len() - 1;
            self.by_name.insert(name.to_string(), i);
            self.by_uuid.entry(id).or_insert(i);
            return Some(i);
        }
        self.by_uuid.get(&id).copied()
    }
}

/// Builds all series and plates from the master OME-XML.
///
/// `opened` holds the file name and UUID of the TIFF file that was opened
/// (file 0), if any.
fn build(
    dir: &Path,
    opened: Option<(&str, Option<&str>)>,
    xml: &str,
    list: &mut Vec<LazyFile>,
) -> Result<(Vec<Series>, Vec<PlateMeta>)> {
    let doc = parse(xml)?;
    let root = doc.root_element();
    let mut files = Files { dir, by_name: HashMap::new(), by_uuid: HashMap::new(), list };
    if let Some((name, uuid)) = opened {
        files.by_name.insert(name.to_string(), 0);
        if let Some(uuid) = uuid {
            files.by_uuid.insert(uuid.to_string(), 0);
        }
    }
    let objectives: HashMap<&str, ObjectiveMeta> = descendants(root, "Objective")
        .filter_map(|o| Some((o.attribute("ID")?, objective(o))))
        .collect();
    let rois: HashMap<&str, Roi> = children(root, "ROI").filter_map(|r| Some((r.attribute("ID")?, roi(r)))).collect();

    let mut series = Vec::new();
    let mut image_ids = HashMap::new();
    for (i, image) in children(root, "Image").enumerate() {
        if let Some(id) = image.attribute("ID") {
            image_ids.insert(id.to_string(), i);
        }
        series.push(image_series(image, &mut files, &objectives, &rois)?);
    }
    if series.is_empty() {
        return Err(Error::Corrupt("OME-XML without Image".into()));
    }
    let plates = children(root, "Plate").map(|p| plate(p, &image_ids)).collect();
    Ok((series, plates))
}

/// Builds the series of one `Image` element.
fn image_series(
    image: Node,
    files: &mut Files,
    objectives: &HashMap<&str, ObjectiveMeta>,
    rois: &HashMap<&str, Roi>,
) -> Result<Series> {
    // Schema 2008-09 allows several Pixels per Image, selected by DefaultPixels.
    let default = image.attribute("DefaultPixels");
    let pixels = children(image, "Pixels")
        .find(|p| default.is_some_and(|d| p.attribute("ID") == Some(d)))
        .or_else(|| child(image, "Pixels"))
        .ok_or_else(|| Error::Corrupt("Image without Pixels".into()))?;
    let size = |a: &str| pixels.attribute(a).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).max(1);
    let (sx, sy, sz, sc, st) = (size("SizeX"), size("SizeY"), size("SizeZ"), size("SizeC"), size("SizeT"));
    // "PixelType" is the name used by schema 2008-09.
    let pixel_type = match pixels.attribute("Type").or(pixels.attribute("PixelType")).unwrap_or("uint8") {
        "int8" => PixelType::I8,
        "uint8" | "bit" => PixelType::U8,
        "int16" => PixelType::I16,
        "uint16" => PixelType::U16,
        "int32" => PixelType::I32,
        "uint32" => PixelType::U32,
        "float" => PixelType::F32,
        "double" => PixelType::F64,
        t => return Err(Error::UnsupportedVariant(format!("OME pixel type {t}"))),
    };
    let channel_nodes: Vec<Node> = children(pixels, "Channel").collect();
    let spp = channel_nodes
        .first()
        .and_then(|c| c.attribute("SamplesPerPixel"))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, sc);
    let size_c = (sc / spp).max(1);
    let samples_out: u8 = match spp {
        3 | 4 => spp as u8,
        _ => 1,
    };
    let order = pixels.attribute("DimensionOrder").unwrap_or("XYZCT");
    let total = (size_c * sz * st) as usize;

    // Map planes to (file, IFD).
    let mut mapping: Vec<Option<(usize, usize)>> = vec![None; total];
    for td in children(pixels, "TiffData") {
        let Some(file) = files.resolve(td) else { continue };
        let num = |a: &str| td.attribute(a).and_then(|v| v.parse::<usize>().ok());
        let ifd = num("IFD").unwrap_or(0);
        // "NumPlanes" is the name used by schema 2008-09.
        let count = match num("PlaneCount").or(num("NumPlanes")) {
            Some(n) => n,
            None if td.attribute("IFD").is_some() => 1,
            None => files.list[file].get()?.ifds.len().saturating_sub(ifd),
        };
        let first = (num("FirstZ").unwrap_or(0), num("FirstC").unwrap_or(0), num("FirstT").unwrap_or(0));
        let start = rasterize(order, first, (sz as usize, size_c as usize, st as usize));
        for k in 0..count {
            let Some(pos) = start.map(|s| s + k).filter(|&p| p < total) else { break };
            let (z, c, t) = unrasterize(order, pos, (sz as usize, size_c as usize, st as usize));
            mapping[c + size_c as usize * (z + sz as usize * t)] = Some((file, ifd + k));
        }
    }
    let (first_file, first_ifd) = mapping
        .iter()
        .flatten()
        .copied()
        .next()
        .ok_or_else(|| Error::Corrupt("OME-XML maps no plane to a TIFF IFD".into()))?;

    // Validate the layout with the first plane and find its sub-resolutions.
    let tiff = files.list[first_file].get()?;
    let ifd = tiff.ifds.get(first_ifd).ok_or_else(|| Error::Corrupt(format!("IFD {first_ifd} missing")))?;
    let first = super::image::TiffImage::new(tiff, ifd)?;
    // Some files declare a signed type in OME-XML for unsigned TIFF data (or the
    // reverse); the TIFF sample format describes the stored bytes, so it wins.
    let type_ok = first.pixel_type == pixel_type
        || first.pixel_type.bytes() == pixel_type.bytes() && !matches!(pixel_type, PixelType::F32 | PixelType::F64);
    let ome_type = pixel_type;
    let pixel_type = first.pixel_type;
    if first.width != sx
        || first.height != sy
        || !type_ok
        || first.samples_out != samples_out
        || first.channels() != 1 && spp == 1
    {
        return Err(Error::Corrupt(format!(
            "OME-XML ({sx}x{sy} {ome_type:?} {samples_out} samples) does not match TIFF ({}x{} {:?} {} samples)",
            first.width, first.height, first.pixel_type, first.samples_out
        )));
    }
    let mut levels_meta = vec![level_meta(&first, sx)];
    for sub in tiff.sub_ifds(ifd).unwrap_or_default() {
        match super::image::TiffImage::new(tiff, &sub) {
            Ok(img)
                if (img.width as u64) < levels_meta.last().map_or(0, |l| l.size_x)
                    && img.pixel_type.bytes() == pixel_type.bytes() =>
            {
                levels_meta.push(level_meta(&img, sx));
            }
            _ => break,
        }
    }

    let samples = if samples_out == 1 { Samples::One(0) } else { Samples::All };
    let planes = (0..total).map(|p| mapping[p].map(|_| PlaneRef { page: p, samples })).collect();
    let levels = (0..levels_meta.len())
        .map(|level| mapping.iter().map(|m| m.map(|(file, ifd)| Slot::lazy(file, ifd, level))).collect())
        .collect();

    let unit = |a: &str| pixels.attribute(format!("{a}Unit").as_str()).unwrap_or("µm");
    let length = |a: &str| pixels.attribute(a).and_then(|v| v.parse::<f64>().ok()).and_then(|v| to_nm(v, unit(a)));
    let channels: Vec<ChannelMeta> = (0..size_c as usize).map(|i| channel_nodes.get(i).map(|&c| channel(c)).unwrap_or_default()).collect();
    let objective = child(image, "ObjectiveSettings")
        .and_then(|o| o.attribute("ID"))
        .and_then(|id| objectives.get(id).cloned());
    let image_rois = children(image, "ROIRef").filter_map(|r| r.attribute("ID")).filter_map(|id| rois.get(id).cloned()).collect();
    let stage_position = child(image, "StageLabel").and_then(position);
    let significant_bits = pixels
        .attribute("SignificantBits")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|&b| b as usize != pixel_type.bytes() * 8);

    let meta = SeriesMeta {
        name: image.attribute("Name").map(String::from),
        description: child(image, "Description").and_then(|d| d.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty()),
        acquisition_date: child(image, "AcquisitionDate")
            .or_else(|| child(image, "AcquiredDate"))
            .and_then(|d| d.text())
            .map(|t| t.trim().to_string()),
        size_c,
        size_z: sz,
        size_t: st,
        pixel_type,
        significant_bits,
        samples_per_pixel: samples_out,
        levels: levels_meta,
        physical_size: PhysicalSize { x_nm: length("PhysicalSizeX"), y_nm: length("PhysicalSizeY"), z_nm: length("PhysicalSizeZ") },
        time_increment_s: pixels.attribute("TimeIncrement").and_then(|v| v.parse().ok()).and_then(|v| to_s(v, pixels.attribute("TimeIncrementUnit").unwrap_or("s"))),
        channels,
        objective,
        stage_position,
        rois: image_rois,
    };
    let mut plane_meta = vec![PlaneMeta::default(); total];
    for p in children(pixels, "Plane") {
        let at = |a: &str| p.attribute(a).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let (c, z, t) = (at("TheC"), at("TheZ"), at("TheT"));
        if c >= size_c as usize || z >= sz as usize || t >= st as usize {
            continue;
        }
        let time = |a: &str| p.attribute(a).and_then(|v| v.parse::<f64>().ok()).and_then(|v| to_s(v, p.attribute(format!("{a}Unit").as_str()).unwrap_or("s")));
        plane_meta[c + size_c as usize * (z + sz as usize * t)] =
            PlaneMeta { delta_t_s: time("DeltaT"), exposure_s: time("ExposureTime"), position: position(p) };
    }
    Ok(Series { meta, levels, planes, plane_meta, first_ifd: (first_file, first_ifd) })
}

/// Index of plane (z, c, t) in the order given by `DimensionOrder`.
fn rasterize(order: &str, (z, c, t): (usize, usize, usize), (sz, sc, st): (usize, usize, usize)) -> Option<usize> {
    if z >= sz || c >= sc || t >= st {
        return None;
    }
    let mut index = 0;
    let mut stride = 1;
    for axis in order.chars().skip(2) {
        let (v, n) = match axis {
            'Z' => (z, sz),
            'C' => (c, sc),
            'T' => (t, st),
            _ => continue,
        };
        index += v * stride;
        stride *= n;
    }
    Some(index)
}

/// Inverse of [`rasterize`].
fn unrasterize(order: &str, mut index: usize, (sz, sc, st): (usize, usize, usize)) -> (usize, usize, usize) {
    let (mut z, mut c, mut t) = (0, 0, 0);
    for axis in order.chars().skip(2) {
        match axis {
            'Z' => {
                z = index % sz;
                index /= sz;
            }
            'C' => {
                c = index % sc;
                index /= sc;
            }
            'T' => {
                t = index % st;
                index /= st;
            }
            _ => {}
        }
    }
    (z, c, t)
}

fn channel(c: Node) -> ChannelMeta {
    let f = |a: &str| c.attribute(a).and_then(|v| v.parse::<f64>().ok());
    let s = |a: &str| c.attribute(a).map(String::from);
    let wavelength = |a: &str| f(a).and_then(|v| to_nm(v, c.attribute(format!("{a}Unit").as_str()).unwrap_or("nm")));
    let settings = child(c, "DetectorSettings");
    let ds = |a: &str| settings.and_then(|d| d.attribute(a)).and_then(|v| v.parse::<f64>().ok());
    let binning = settings.and_then(|d| d.attribute("Binning")).and_then(|b| {
        let (x, y) = b.split_once('x')?;
        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
    });
    ChannelMeta {
        name: s("Name"),
        color: c.attribute("Color").and_then(|v| v.parse::<i64>().ok()).map(|v| (v as u32).to_be_bytes()),
        excitation_wavelength_nm: wavelength("ExcitationWavelength"),
        emission_wavelength_nm: wavelength("EmissionWavelength"),
        fluor: s("Fluor"),
        contrast_method: s("ContrastMethod"),
        pinhole_size_nm: f("PinholeSize").and_then(|v| to_nm(v, c.attribute("PinholeSizeUnit").unwrap_or("µm"))),
        detector_gain: ds("Gain"),
        detector_offset: ds("Offset"),
        binning,
        ..Default::default()
    }
}

fn objective(o: Node) -> ObjectiveMeta {
    ObjectiveMeta {
        manufacturer: o.attribute("Manufacturer").map(String::from),
        model: o.attribute("Model").map(String::from),
        magnification: o.attribute("NominalMagnification").and_then(|v| v.parse().ok()),
        numerical_aperture: o.attribute("LensNA").and_then(|v| v.parse().ok()),
        immersion: o.attribute("Immersion").map(String::from),
    }
}

/// Stage position from `PositionX/Y/Z` attributes. Positions in the
/// unitless "reference frame" (the schema default) are not converted.
fn position(n: Node) -> Option<Position> {
    let get = |a: &str| {
        let v = n.attribute(a)?.parse::<f64>().ok()?;
        to_nm(v, n.attribute(format!("{a}Unit").as_str()).unwrap_or("reference frame"))
    };
    let p = Position { x_nm: get("PositionX").or(get("X")), y_nm: get("PositionY").or(get("Y")), z_nm: get("PositionZ").or(get("Z")) };
    (p.x_nm.is_some() || p.y_nm.is_some() || p.z_nm.is_some()).then_some(p)
}

fn roi(r: Node) -> Roi {
    let mut shapes = Vec::new();
    for s in descendants(r, "Union").flat_map(|u| u.children().filter(Node::is_element)) {
        let f = |a: &str| s.attribute(a).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        let points = || -> Vec<[f64; 2]> {
            s.attribute("Points")
                .unwrap_or("")
                .split_whitespace()
                .filter_map(|p| {
                    let (x, y) = p.split_once(',')?;
                    Some([x.parse().ok()?, y.parse().ok()?])
                })
                .collect()
        };
        let geometry = match s.tag_name().name() {
            "Point" => Geometry::Point([f("X"), f("Y")]),
            "Line" => Geometry::Line([f("X1"), f("Y1")], [f("X2"), f("Y2")]),
            "Rectangle" => Geometry::Rectangle { x: f("X"), y: f("Y"), width: f("Width"), height: f("Height") },
            "Ellipse" => Geometry::Ellipse { center: [f("X"), f("Y")], radius_x: f("RadiusX"), radius_y: f("RadiusY") },
            "Polygon" => Geometry::Polygon(points()),
            "Polyline" => Geometry::Polyline(points()),
            _ => continue,
        };
        let at = |a: &str| s.attribute(a).and_then(|v| v.parse().ok());
        shapes.push(Shape { geometry, label: s.attribute("Text").map(String::from), c: at("TheC"), z: at("TheZ"), t: at("TheT") });
    }
    Roi { name: r.attribute("Name").map(String::from), shapes }
}

fn plate(p: Node, image_ids: &HashMap<String, usize>) -> PlateMeta {
    let num = |n: Node, a: &str| n.attribute(a).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let mut wells = Vec::new();
    for w in children(p, "Well") {
        for (field, ws) in children(w, "WellSample").enumerate() {
            let Some(series) = child(ws, "ImageRef").and_then(|r| r.attribute("ID")).and_then(|id| image_ids.get(id)) else {
                continue;
            };
            wells.push(WellSample {
                row: num(w, "Row"),
                col: num(w, "Column"),
                field: field as u32,
                series: *series,
                position: position(ws),
            });
        }
    }
    PlateMeta { name: p.attribute("Name").map(String::from), rows: num(p, "Rows"), cols: num(p, "Columns"), wells }
}

/// Converts a length to nanometres; `None` for unknown or relative units.
fn to_nm(v: f64, unit: &str) -> Option<f64> {
    let factor = match unit {
        "m" => 1e9,
        "dm" => 1e8,
        "cm" => 1e7,
        "mm" => 1e6,
        "µm" | "um" | "μm" | "micron" => 1e3,
        "nm" => 1.0,
        "pm" => 1e-3,
        "Å" => 0.1,
        "in" => 25.4e6,
        "ft" => 304.8e6,
        _ => return None,
    };
    v.is_finite().then_some(v * factor)
}

/// Converts a time to seconds; `None` for unknown units.
fn to_s(v: f64, unit: &str) -> Option<f64> {
    let factor = match unit {
        "s" => 1.0,
        "ms" => 1e-3,
        "µs" | "us" | "μs" => 1e-6,
        "ns" => 1e-9,
        "min" => 60.0,
        "h" => 3600.0,
        "d" => 86400.0,
        _ => return None,
    };
    v.is_finite().then_some(v * factor)
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn descendants<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.descendants().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterization_follows_dimension_order() {
        // Fragment 2 of the OME-TIFF specification: XYCTZ, Z=4, T=3, C=2.
        let dims = (4, 2, 3);
        assert_eq!(unrasterize("XYCTZ", 1, dims), (0, 1, 0));
        assert_eq!(unrasterize("XYCTZ", 2, dims), (0, 0, 1));
        assert_eq!(unrasterize("XYCTZ", 6, dims), (1, 0, 0));
        for i in 0..24 {
            assert_eq!(rasterize("XYCTZ", unrasterize("XYCTZ", i, dims), dims), Some(i));
        }
        assert_eq!(rasterize("XYZTC", (0, 1, 0), (3, 2, 2)), Some(6));
    }

    #[test]
    fn unit_conversion() {
        assert_eq!(to_nm(0.5, "µm"), Some(500.0));
        assert_eq!(to_nm(1.0, "reference frame"), None);
        assert_eq!(to_s(20.0, "ms"), Some(0.02));
    }
}

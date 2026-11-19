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
    ChannelMeta, DetectorMeta, DichroicMeta, EnvironmentMeta, Error, ExperimenterMeta, FilterMeta, Geometry, LightSourceMeta, MicroscopeMeta,
    ObjectiveMeta, OriginalMeta, PhysicalSize, PixelType, PlaneMeta, PlateMeta, Position, Result, Roi, SeriesMeta, Shape, WellSample,
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
    let result = master_xml(&dir, xml).and_then(|master| build(&dir, Some((&name, own_uuid.as_deref())), &master, &mut files));
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

pub(super) fn parse(xml: &str) -> Result<Document<'_>> {
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
fn build(dir: &Path, opened: Option<(&str, Option<&str>)>, xml: &str, list: &mut Vec<LazyFile>) -> Result<(Vec<Series>, Vec<PlateMeta>)> {
    let doc = parse(xml)?;
    let root = doc.root_element();
    let mut files = Files { dir, by_name: HashMap::new(), by_uuid: HashMap::new(), list };
    if let Some((name, uuid)) = opened {
        files.by_name.insert(name.to_string(), 0);
        if let Some(uuid) = uuid {
            files.by_uuid.insert(uuid.to_string(), 0);
        }
    }
    let refs = DocRefs::new(root);

    let mut series = Vec::new();
    let mut image_ids = HashMap::new();
    for (i, image) in children(root, "Image").enumerate() {
        if let Some(id) = image.attribute("ID") {
            image_ids.insert(id.to_string(), i);
        }
        series.push(image_series(image, &mut files, &refs)?);
    }
    if series.is_empty() {
        return Err(Error::Corrupt("OME-XML without Image".into()));
    }
    let plates = children(root, "Plate").map(|p| plate(p, &image_ids)).collect();
    Ok((series, plates))
}

/// Document-level elements that images refer to by ID, for [`describe`].
pub(super) struct DocRefs<'a, 'i> {
    root: Node<'a, 'i>,
    /// Every element with an `ID`, keyed by (tag name, ID).
    by_id: HashMap<(&'a str, &'a str), Node<'a, 'i>>,
    objectives: HashMap<&'a str, ObjectiveMeta>,
    rois: HashMap<&'a str, Roi>,
}

impl<'a, 'i> DocRefs<'a, 'i> {
    pub fn new(root: Node<'a, 'i>) -> Self {
        let by_id = root.descendants().filter(Node::is_element).filter_map(|n| Some(((n.tag_name().name(), n.attribute("ID")?), n))).collect();
        let objectives = descendants(root, "Objective").filter_map(|o| Some((o.attribute("ID")?, objective(o)))).collect();
        let rois = children(root, "ROI").filter_map(|r| Some((r.attribute("ID")?, roi(r)))).collect();
        Self { root, by_id, objectives, rois }
    }

    /// The `tag` element whose ID is given by the `ID` attribute of `reference`.
    fn get(&self, tag: &str, reference: Option<Node>) -> Option<Node<'a, 'i>> {
        self.by_id.get(&(tag, reference?.attribute("ID")?)).copied()
    }

    /// The element referenced by child `ref_tag` of `n`, or the only `tag`
    /// element of the document if `n` has no such reference.
    fn get_or_only(&self, n: Node, ref_tag: &str, tag: &'a str) -> Option<Node<'a, 'i>> {
        match child(n, ref_tag) {
            Some(r) => self.get(tag, Some(r)),
            None => {
                let mut all = children(self.root, tag);
                all.next().filter(|_| all.next().is_none())
            }
        }
    }
}

/// The `Pixels` element of an `Image` (schema 2008-09 allows several,
/// selected by `DefaultPixels`).
pub(super) fn pixels_of<'a, 'i>(image: Node<'a, 'i>) -> Result<Node<'a, 'i>> {
    let default = image.attribute("DefaultPixels");
    children(image, "Pixels")
        .find(|p| default.is_some_and(|d| p.attribute("ID") == Some(d)))
        .or_else(|| child(image, "Pixels"))
        .ok_or_else(|| Error::Corrupt("Image without Pixels".into()))
}

/// The pixel type of a `Pixels` element (`PixelType` in schema 2008-09).
pub(super) fn pixel_type_of(pixels: Node) -> Result<PixelType> {
    Ok(match pixels.attribute("Type").or(pixels.attribute("PixelType")).unwrap_or("uint8") {
        "int8" => PixelType::I8,
        "uint8" | "bit" => PixelType::U8,
        "int16" => PixelType::I16,
        "uint16" => PixelType::U16,
        "int32" => PixelType::I32,
        "uint32" => PixelType::U32,
        "float" => PixelType::F32,
        "double" => PixelType::F64,
        t => return Err(Error::UnsupportedVariant(format!("OME pixel type {t}"))),
    })
}

/// Sizes of a `Pixels` element: (x, y, z, c, t), each at least 1, `c`
/// counting channels (not samples); samples per channel and output samples
/// (1, 3 or 4).
pub(super) fn sizes_of(pixels: Node) -> ((u32, u32, u32, u32, u32), u32, u8) {
    let size = |a: &str| pixels.attribute(a).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).max(1);
    let (sx, sy, sz, sc, st) = (size("SizeX"), size("SizeY"), size("SizeZ"), size("SizeC"), size("SizeT"));
    let spp =
        children(pixels, "Channel").next().and_then(|c| c.attribute("SamplesPerPixel")).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).clamp(1, sc);
    let samples_out = match spp {
        3 | 4 => spp as u8,
        _ => 1,
    };
    ((sx, sy, sz, (sc / spp).max(1), st), spp, samples_out)
}

/// Series metadata and plane metadata of an `Image` whose pixel layout is
/// known.
#[allow(clippy::too_many_arguments)]
pub(super) fn describe(
    image: Node,
    pixels: Node,
    (size_c, sz, st): (u32, u32, u32),
    pixel_type: PixelType,
    samples_out: u8,
    levels: Vec<crate::LevelMeta>,
    refs: &DocRefs,
) -> (SeriesMeta, Vec<PlaneMeta>) {
    let total = (size_c * sz * st) as usize;
    let channel_nodes: Vec<Node> = children(pixels, "Channel").collect();
    let unit = |a: &str| pixels.attribute(format!("{a}Unit").as_str()).unwrap_or("µm");
    let length = |a: &str| pixels.attribute(a).and_then(|v| v.parse::<f64>().ok()).and_then(|v| to_nm(v, unit(a)));
    let channels: Vec<ChannelMeta> = (0..size_c as usize).map(|i| channel_nodes.get(i).map(|&c| channel(c, refs)).unwrap_or_default()).collect();
    let settings = child(image, "ObjectiveSettings");
    let objective = settings
        .and_then(|o| o.attribute("ID"))
        .and_then(|id| refs.objectives.get(id).cloned())
        .map(|o| ObjectiveMeta { refractive_index: settings.and_then(|s| s.attribute("RefractiveIndex")).and_then(|v| v.parse().ok()), ..o });
    let image_rois = children(image, "ROIRef").filter_map(|r| r.attribute("ID")).filter_map(|id| refs.rois.get(id).cloned()).collect();
    let microscope = refs.get_or_only(image, "InstrumentRef", "Instrument").and_then(|i| child(i, "Microscope")).map(|m| MicroscopeMeta {
        manufacturer: text_attr(m, "Manufacturer"),
        model: text_attr(m, "Model"),
        serial_number: text_attr(m, "SerialNumber"),
        kind: text_attr(m, "Type"),
    });
    let experimenter = refs.get_or_only(image, "ExperimenterRef", "Experimenter").map(|e| ExperimenterMeta {
        first_name: text_attr(e, "FirstName"),
        last_name: text_attr(e, "LastName"),
        email: text_attr(e, "Email"),
        institution: text_attr(e, "Institution"),
        user_name: text_attr(e, "UserName"),
    });
    let (dataset, project) = membership(image, refs);
    let stage_position = child(image, "StageLabel").and_then(position);
    let significant_bits = pixels.attribute("SignificantBits").and_then(|v| v.parse::<u8>().ok()).filter(|&b| b as usize != pixel_type.bytes() * 8);
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
        levels,
        physical_size: PhysicalSize { x_nm: length("PhysicalSizeX"), y_nm: length("PhysicalSizeY"), z_nm: length("PhysicalSizeZ") },
        time_increment_s: pixels
            .attribute("TimeIncrement")
            .and_then(|v| v.parse().ok())
            .and_then(|v| to_s(v, pixels.attribute("TimeIncrementUnit").unwrap_or("s"))),
        channels,
        objective,
        microscope,
        environment: child(image, "ImagingEnvironment").map(environment),
        experimenter,
        project: project.and_then(|p| text_attr(p, "Name")),
        dataset: dataset.and_then(|d| text_attr(d, "Name")),
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
        let time = |a: &str| {
            p.attribute(a).and_then(|v| v.parse::<f64>().ok()).and_then(|v| to_s(v, p.attribute(format!("{a}Unit").as_str()).unwrap_or("s")))
        };
        plane_meta[c + size_c as usize * (z + sz as usize * t)] =
            PlaneMeta { delta_t_s: time("DeltaT"), exposure_s: time("ExposureTime"), position: position(p) };
    }
    (meta, plane_meta)
}

/// Builds the series of one `Image` element.
fn image_series(image: Node, files: &mut Files, refs: &DocRefs) -> Result<Series> {
    let pixels = pixels_of(image)?;
    let ((sx, sy, sz, size_c, st), spp, samples_out) = sizes_of(pixels);
    let pixel_type = pixel_type_of(pixels)?;
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
    let (first_file, first_ifd) =
        mapping.iter().flatten().copied().next().ok_or_else(|| Error::Corrupt("OME-XML maps no plane to a TIFF IFD".into()))?;

    // Validate the layout with the first plane and find its sub-resolutions.
    let tiff = files.list[first_file].get()?;
    let ifd = tiff.ifds.get(first_ifd).ok_or_else(|| Error::Corrupt(format!("IFD {first_ifd} missing")))?;
    let first = super::image::TiffImage::new(tiff, ifd)?;
    // Some files declare a signed type in OME-XML for unsigned TIFF data (or the
    // reverse); the TIFF sample format describes the stored bytes, so it wins.
    let type_ok =
        first.pixel_type == pixel_type || first.pixel_type.bytes() == pixel_type.bytes() && !matches!(pixel_type, PixelType::F32 | PixelType::F64);
    let ome_type = pixel_type;
    let pixel_type = first.pixel_type;
    if first.width != sx || first.height != sy || !type_ok || first.samples_out != samples_out || first.channels() != 1 && spp == 1 {
        return Err(Error::Corrupt(format!(
            "OME-XML ({sx}x{sy} {ome_type:?} {samples_out} samples) does not match TIFF ({}x{} {:?} {} samples)",
            first.width, first.height, first.pixel_type, first.samples_out
        )));
    }
    let mut levels_meta = vec![level_meta(&first, sx)];
    for sub in tiff.sub_ifds(ifd).unwrap_or_default() {
        match super::image::TiffImage::new(tiff, &sub) {
            Ok(img) if (img.width as u64) < levels_meta.last().map_or(0, |l| l.size_x) && img.pixel_type.bytes() == pixel_type.bytes() => {
                levels_meta.push(level_meta(&img, sx));
            }
            _ => break,
        }
    }

    let samples = if samples_out == 1 { Samples::One(0) } else { Samples::All };
    let planes = (0..total).map(|p| mapping[p].map(|_| PlaneRef { page: p, samples })).collect();
    let levels = (0..levels_meta.len()).map(|level| mapping.iter().map(|m| m.map(|(file, ifd)| Slot::lazy(file, ifd, level))).collect()).collect();

    let (meta, plane_meta) = describe(image, pixels, (size_c, sz, st), pixel_type, samples_out, levels_meta, refs);
    Ok(Series { meta, levels, planes, plane_meta, first_ifd: (first_file, first_ifd), original: Vec::new() })
}

/// Index of plane (z, c, t) in the order given by `DimensionOrder`.
pub(super) fn rasterize(order: &str, (z, c, t): (usize, usize, usize), (sz, sc, st): (usize, usize, usize)) -> Option<usize> {
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
pub(super) fn unrasterize(order: &str, mut index: usize, (sz, sc, st): (usize, usize, usize)) -> (usize, usize, usize) {
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

fn channel(c: Node, refs: &DocRefs) -> ChannelMeta {
    let f = |a: &str| c.attribute(a).and_then(|v| v.parse::<f64>().ok());
    let s = |a: &str| c.attribute(a).map(String::from);
    let wavelength = |a: &str| f(a).and_then(|v| to_nm(v, c.attribute(format!("{a}Unit").as_str()).unwrap_or("nm")));
    let settings = child(c, "DetectorSettings");
    let detector = refs.get("Detector", settings);
    // Settings override the defaults stored with the detector.
    let ds = |a: &str| settings.and_then(|d| d.attribute(a)).or_else(|| detector.and_then(|d| d.attribute(a))).and_then(|v| v.parse::<f64>().ok());
    let (excitation, emission, dichroic) = light_path(c, refs);
    let (excitation_filters, emission_filters) = (excitation.into_iter().map(filter).collect(), emission.into_iter().map(filter).collect());
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
        zoom: ds("Zoom"),
        detector: detector.map(|d| DetectorMeta {
            manufacturer: text_attr(d, "Manufacturer"),
            model: text_attr(d, "Model"),
            serial_number: text_attr(d, "SerialNumber"),
            kind: text_attr(d, "Type"),
        }),
        light_source: light_source(child(c, "LightSourceSettings"), refs),
        excitation_filters,
        emission_filters,
        dichroic: dichroic.map(|d| DichroicMeta { manufacturer: text_attr(d, "Manufacturer"), model: text_attr(d, "Model") }),
        ..Default::default()
    }
}

/// Light source of a channel with its `LightSourceSettings`.
fn light_source(settings: Option<Node>, refs: &DocRefs) -> Option<LightSourceMeta> {
    const KINDS: [&str; 5] = ["Laser", "Arc", "Filament", "LightEmittingDiode", "GenericExcitationSource"];
    let source = KINDS.iter().find_map(|k| refs.get(k, settings));
    if settings.is_none() && source.is_none() {
        return None;
    }
    let num = |n: Option<Node>, a: &str| n.and_then(|n| n.attribute(a)).and_then(|v| v.parse::<f64>().ok());
    let wavelength = |n: Option<Node>| num(n, "Wavelength").and_then(|v| to_nm(v, n.and_then(|n| n.attribute("WavelengthUnit")).unwrap_or("nm")));
    let laser = source.filter(|s| s.tag_name().name() == "Laser");
    let s = |a: &str| source.and_then(|n| text_attr(n, a));
    Some(LightSourceMeta {
        manufacturer: s("Manufacturer"),
        model: s("Model"),
        serial_number: s("SerialNumber"),
        kind: source.map(|n| n.tag_name().name().to_string()),
        subtype: s("Type"),
        laser_medium: laser.and_then(|l| text_attr(l, "LaserMedium")),
        wavelength_nm: wavelength(settings).or_else(|| wavelength(laser)),
        power_mw: num(source, "Power").and_then(|v| to_mw(v, source.and_then(|n| n.attribute("PowerUnit")).unwrap_or("mW"))),
        attenuation: num(settings, "Attenuation"),
    })
}

/// Excitation filters, emission filters and dichroic of a channel, from
/// its `LightPath` or else its `FilterSet`.
fn light_path<'a, 'i>(c: Node, refs: &DocRefs<'a, 'i>) -> (Vec<Node<'a, 'i>>, Vec<Node<'a, 'i>>, Option<Node<'a, 'i>>) {
    let Some(path) = child(c, "LightPath").or_else(|| refs.get("FilterSet", child(c, "FilterSetRef"))) else {
        return (Vec::new(), Vec::new(), None);
    };
    let filters =
        |tag: &str| path.children().filter(|n| n.is_element() && n.tag_name().name() == tag).filter_map(|r| refs.get("Filter", Some(r))).collect();
    (filters("ExcitationFilterRef"), filters("EmissionFilterRef"), refs.get("Dichroic", child(path, "DichroicRef")))
}

fn filter(f: Node) -> FilterMeta {
    let range = child(f, "TransmittanceRange");
    let num = |a: &str| range.and_then(|r| r.attribute(a)).and_then(|v| v.parse::<f64>().ok());
    let edge = |a: &str| num(a).and_then(|v| to_nm(v, range.and_then(|r| r.attribute(format!("{a}Unit").as_str())).unwrap_or("nm")));
    FilterMeta {
        manufacturer: text_attr(f, "Manufacturer"),
        model: text_attr(f, "Model"),
        kind: text_attr(f, "Type"),
        filter_wheel: text_attr(f, "FilterWheel"),
        cut_in_nm: edge("CutIn"),
        cut_out_nm: edge("CutOut"),
        transmittance: num("Transmittance"),
    }
}

fn environment(e: Node) -> EnvironmentMeta {
    let num = |a: &str| e.attribute(a).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite());
    EnvironmentMeta {
        temperature_c: num("Temperature").and_then(|v| to_celsius(v, e.attribute("TemperatureUnit").unwrap_or("°C"))),
        co2_fraction: num("CO2Percent"),
        humidity_fraction: num("Humidity"),
        air_pressure_mbar: num("AirPressure").and_then(|v| to_mbar(v, e.attribute("AirPressureUnit").unwrap_or("mbar"))),
    }
}

/// Dataset and project an image belongs to. Schema 2016-06 lists images in
/// the dataset (`ImageRef`) and datasets in the project (`DatasetRef`);
/// older schemas point upwards (`Image/DatasetRef`, `Dataset/ProjectRef`).
fn membership<'a, 'i>(image: Node, refs: &DocRefs<'a, 'i>) -> (Option<Node<'a, 'i>>, Option<Node<'a, 'i>>) {
    let lists = |parent: Node, tag: &str, id: Option<&str>| id.is_some() && children(parent, tag).any(|r| r.attribute("ID") == id);
    let dataset = refs
        .get("Dataset", child(image, "DatasetRef"))
        .or_else(|| children(refs.root, "Dataset").find(|d| lists(*d, "ImageRef", image.attribute("ID"))));
    let project = dataset.and_then(|d| {
        refs.get("Project", child(d, "ProjectRef")).or_else(|| children(refs.root, "Project").find(|p| lists(*p, "DatasetRef", d.attribute("ID"))))
    });
    (dataset, project)
}

/// A non-empty, trimmed attribute value.
fn text_attr(n: Node, a: &str) -> Option<String> {
    n.attribute(a).map(str::trim).filter(|v| !v.is_empty()).map(String::from)
}

fn objective(o: Node) -> ObjectiveMeta {
    ObjectiveMeta {
        manufacturer: o.attribute("Manufacturer").map(String::from),
        model: o.attribute("Model").map(String::from),
        magnification: o.attribute("NominalMagnification").and_then(|v| v.parse().ok()),
        numerical_aperture: o.attribute("LensNA").and_then(|v| v.parse().ok()),
        immersion: o.attribute("Immersion").map(String::from),
        correction: text_attr(o, "Correction"),
        working_distance_nm: o
            .attribute("WorkingDistance")
            .and_then(|v| v.parse().ok())
            .and_then(|v| to_nm(v, o.attribute("WorkingDistanceUnit").unwrap_or("µm"))),
        refractive_index: None,
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

pub(super) fn plate(p: Node, image_ids: &HashMap<String, usize>) -> PlateMeta {
    let num = |n: Node, a: &str| n.attribute(a).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let mut wells = Vec::new();
    for w in children(p, "Well") {
        for (field, ws) in children(w, "WellSample").enumerate() {
            let Some(series) = child(ws, "ImageRef").and_then(|r| r.attribute("ID")).and_then(|id| image_ids.get(id)) else {
                continue;
            };
            wells.push(WellSample { row: num(w, "Row"), col: num(w, "Column"), field: field as u32, series: *series, position: position(ws) });
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

/// Converts a power to milliwatts; `None` for unknown units.
fn to_mw(v: f64, unit: &str) -> Option<f64> {
    let factor = match unit {
        "MW" => 1e9,
        "kW" => 1e6,
        "W" => 1e3,
        "mW" => 1.0,
        "µW" | "uW" | "μW" => 1e-3,
        "nW" => 1e-6,
        "pW" => 1e-9,
        _ => return None,
    };
    v.is_finite().then_some(v * factor)
}

/// Converts a temperature to degrees Celsius; `None` for unknown units.
fn to_celsius(v: f64, unit: &str) -> Option<f64> {
    let c = match unit {
        "°C" | "C" => v,
        "K" => v - 273.15,
        "°F" | "F" => (v - 32.0) * 5.0 / 9.0,
        "°R" | "R" => (v - 491.67) * 5.0 / 9.0,
        _ => return None,
    };
    c.is_finite().then_some(c)
}

/// Converts a pressure to millibar; `None` for unknown units.
fn to_mbar(v: f64, unit: &str) -> Option<f64> {
    let factor = match unit {
        "bar" => 1e3,
        "mbar" | "hPa" => 1.0,
        "Pa" => 1e-2,
        "kPa" => 10.0,
        "MPa" => 1e4,
        "atm" => 1013.25,
        "Torr" | "mm Hg" | "mmHg" => 1.333_223_684,
        "mTorr" => 1.333_223_684e-3,
        "psi" => 68.947_572_93,
        _ => return None,
    };
    v.is_finite().then_some(v * factor)
}

pub(super) fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

pub(super) fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
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
    fn instrument_environment_and_owner() {
        let xml = r#"<OME xmlns="http://www.openmicroscopy.org/Schemas/OME/2016-06">
          <Project ID="Project:0" Name="Organoids"><DatasetRef ID="Dataset:0"/></Project>
          <Dataset ID="Dataset:0" Name="Day 3"><ImageRef ID="Image:0"/></Dataset>
          <Experimenter ID="Experimenter:0" FirstName="Ada" LastName="Lovelace" Email="ada@example.org"/>
          <Instrument ID="Instrument:0">
            <Microscope Manufacturer="Zeiss" Model="LSM 980" Type="Inverted"/>
            <Laser ID="LightSource:0" Type="SolidState" LaserMedium="Other" Wavelength="488" Power="0.03" PowerUnit="W"/>
            <Detector ID="Detector:0" Model="GaAsP" Type="PMT" Zoom="1"/>
            <Objective ID="Objective:0" Model="Plan-Apochromat 63x/1.40 Oil" Correction="PlanApo" Immersion="Oil"
                       LensNA="1.4" NominalMagnification="63" WorkingDistance="0.19" WorkingDistanceUnit="mm"/>
            <FilterSet ID="FilterSet:0"><ExcitationFilterRef ID="Filter:0"/><DichroicRef ID="Dichroic:0"/><EmissionFilterRef ID="Filter:1"/></FilterSet>
            <Filter ID="Filter:0" Type="BandPass"><TransmittanceRange CutIn="470" CutOut="495"/></Filter>
            <Filter ID="Filter:1" Type="BandPass"><TransmittanceRange CutIn="0.5" CutInUnit="µm" CutOut="550" Transmittance="0.9"/></Filter>
            <Dichroic ID="Dichroic:0" Model="MBS 488"/>
          </Instrument>
          <Image ID="Image:0" Name="cells">
            <InstrumentRef ID="Instrument:0"/>
            <ObjectiveSettings ID="Objective:0" RefractiveIndex="1.518"/>
            <ImagingEnvironment Temperature="310.15" TemperatureUnit="K" CO2Percent="0.05" Humidity="0.9"/>
            <Pixels ID="Pixels:0" DimensionOrder="XYZCT" Type="uint8" SizeX="4" SizeY="4" SizeZ="1" SizeC="1" SizeT="1">
              <Channel ID="Channel:0:0" FilterSetRef="ignored">
                <LightSourceSettings ID="LightSource:0" Wavelength="488" Attenuation="0.8"/>
                <DetectorSettings ID="Detector:0" Gain="650" Zoom="2.5"/>
                <FilterSetRef ID="FilterSet:0"/>
              </Channel>
            </Pixels>
          </Image>
        </OME>"#;
        let doc = parse(xml).unwrap();
        let root = doc.root_element();
        let refs = DocRefs::new(root);
        let image = child(root, "Image").unwrap();
        let pixels = pixels_of(image).unwrap();
        let (meta, _) = describe(image, pixels, (1, 1, 1), PixelType::U8, 1, Vec::new(), &refs);

        let m = meta.microscope.unwrap();
        assert_eq!((m.manufacturer.as_deref(), m.model.as_deref(), m.kind.as_deref()), (Some("Zeiss"), Some("LSM 980"), Some("Inverted")));
        let o = meta.objective.unwrap();
        assert_eq!(o.correction.as_deref(), Some("PlanApo"));
        assert_eq!(o.working_distance_nm, Some(190_000.0));
        assert_eq!(o.refractive_index, Some(1.518));
        let e = meta.environment.unwrap();
        assert!((e.temperature_c.unwrap() - 37.0).abs() < 1e-9);
        assert_eq!((e.co2_fraction, e.humidity_fraction), (Some(0.05), Some(0.9)));
        let x = meta.experimenter.unwrap();
        assert_eq!((x.first_name.as_deref(), x.last_name.as_deref()), (Some("Ada"), Some("Lovelace")));
        assert_eq!((meta.project.as_deref(), meta.dataset.as_deref()), (Some("Organoids"), Some("Day 3")));

        let c = &meta.channels[0];
        assert_eq!((c.detector_gain, c.zoom), (Some(650.0), Some(2.5)));
        let d = c.detector.as_ref().unwrap();
        assert_eq!((d.model.as_deref(), d.kind.as_deref()), (Some("GaAsP"), Some("PMT")));
        let l = c.light_source.as_ref().unwrap();
        assert_eq!((l.kind.as_deref(), l.subtype.as_deref()), (Some("Laser"), Some("SolidState")));
        assert_eq!((l.wavelength_nm, l.power_mw, l.attenuation), (Some(488.0), Some(30.0), Some(0.8)));
        assert_eq!(c.excitation_filters.len(), 1);
        assert_eq!((c.excitation_filters[0].cut_in_nm, c.excitation_filters[0].cut_out_nm), (Some(470.0), Some(495.0)));
        assert_eq!((c.emission_filters[0].cut_in_nm, c.emission_filters[0].transmittance), (Some(500.0), Some(0.9)));
        assert_eq!(c.dichroic.as_ref().unwrap().model.as_deref(), Some("MBS 488"));
    }

    #[test]
    fn no_instrument_leaves_fields_empty() {
        let xml = r#"<OME><Image ID="Image:0"><Pixels Type="uint8" SizeX="1" SizeY="1"><Channel ID="Channel:0:0"/></Pixels></Image></OME>"#;
        let doc = parse(xml).unwrap();
        let root = doc.root_element();
        let image = child(root, "Image").unwrap();
        let (meta, _) = describe(image, pixels_of(image).unwrap(), (1, 1, 1), PixelType::U8, 1, Vec::new(), &DocRefs::new(root));
        assert!(meta.microscope.is_none() && meta.environment.is_none() && meta.experimenter.is_none() && meta.project.is_none());
        let c = &meta.channels[0];
        assert!(c.detector.is_none() && c.light_source.is_none() && c.excitation_filters.is_empty() && c.dichroic.is_none());
    }

    #[test]
    fn unit_conversion() {
        assert_eq!(to_nm(0.5, "µm"), Some(500.0));
        assert_eq!(to_nm(1.0, "reference frame"), None);
        assert_eq!(to_s(20.0, "ms"), Some(0.02));
    }
}

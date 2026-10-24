//! # tiff
//!
//! TIFF and BigTIFF reader, including ImageJ hyperstacks, pyramids stored as
//! SubIFDs or reduced-resolution IFDs, and OME-TIFF (see [`ome`]).
//!
//! Series are formed from consecutive IFDs with the same pixel layout. Plain
//! multi-page files map pages to time points; samples of non-colour images
//! (e.g. two-sample gray data) become channels. Palette images whose colour
//! map is a gray or single-colour ramp return the stored indices as
//! intensities, with the map in [`ChannelMeta::lut`] and the ramp colour as
//! display colour; any other palette image is expanded to RGB.
//!
//! Provenance: TIFF 6.0 specification, TIFF Technical Note 2, Adobe
//! floating-point predictor note, BigTIFF specification, OME-TIFF
//! specification and OME 2016-06 schema (`doc/tiff/`, `doc/bigtiff/`,
//! `doc/ome-tiff/`, `doc/ome-xml/`); own analysis of sample files.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

pub(super) mod ifd;
pub(super) mod image;
mod imagej;
mod ome;

use std::path::Path;
use std::sync::OnceLock;

use ifd::{Ifd, TiffFile, tag};
use image::{Samples, TiffImage};

use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OpenOptions, OriginalMeta, PhysicalSize, Plane, PlaneMeta,
    PlateMeta, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Tiff, "TIFF", "1");
pub(super) static OME_FORMAT: FormatInfo = FormatInfo::new(FormatId::OmeTiff, "OME-TIFF", "1");

/// Detects TIFF, OME-TIFF and OME-XML companion files.
pub(super) fn probe(path: &Path, head: &[u8]) -> Result<Option<FormatId>> {
    if ome::is_ome_xml(head) {
        return Ok(Some(FormatId::OmeTiff));
    }
    if !TiffFile::matches(head) {
        return Ok(None);
    }
    #[cfg(feature = "ndpi")]
    {
        // NDPI first IFDs may lie beyond 4 GiB; read them the NDPI way.
        if head.starts_with(b"II*\0") {
            if let Ok(t) = TiffFile::open_ndpi(path, 1) {
                if t.ifds[0].has(super::ndpi::TAG_VERSION) {
                    return Ok(Some(FormatId::Ndpi));
                }
            }
        }
    }
    let first = TiffFile::open_first(path)?;
    let description = first.string(&first.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
    #[cfg(feature = "lsm")]
    if first.ifds[0].has(super::lsm::TAG_LSMINFO) {
        return Ok(Some(FormatId::Lsm));
    }
    #[cfg(feature = "svs")]
    if super::svs::matches(&description) {
        return Ok(Some(FormatId::Svs));
    }
    Ok(Some(if description.contains("<OME") { FormatId::OmeTiff } else { FormatId::Tiff }))
}

/// Reader for TIFF, BigTIFF, ImageJ TIFF and OME-TIFF files.
#[derive(Debug)]
pub(super) struct Tiff {
    files: Vec<ome::LazyFile>,
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    /// Where the OME-XML comes from; `None` for plain TIFF.
    ome: Option<ome::Source>,
    /// Why OME metadata was ignored, if it was.
    ome_error: Option<String>,
}

/// Image of one plane at one level: file index and image.
#[derive(Debug)]
struct ImageRef {
    file: usize,
    image: TiffImage,
}

/// Image of one plane at one level, possibly in a file not opened yet.
#[derive(Debug)]
enum Slot {
    Ready(ImageRef),
    /// IFD `ifd` of file `file`; `level` 0 is the IFD itself, higher levels
    /// its SubIFDs. Parsed on first use.
    Lazy { file: usize, ifd: usize, level: usize, image: OnceLock<std::result::Result<TiffImage, String>> },
}

impl Slot {
    fn ready(file: usize, image: TiffImage) -> Self {
        Slot::Ready(ImageRef { file, image })
    }

    fn lazy(file: usize, ifd: usize, level: usize) -> Self {
        Slot::Lazy { file, ifd, level, image: OnceLock::new() }
    }

    /// File index and image, parsing a lazy image if needed.
    fn get<'a>(&'a self, files: &[ome::LazyFile]) -> Result<(usize, &'a TiffImage)> {
        match self {
            Slot::Ready(r) => Ok((r.file, &r.image)),
            Slot::Lazy { file, ifd, level, image } => {
                let image = image.get_or_init(|| {
                    let load = || -> Result<TiffImage> {
                        let tiff = files[*file].get()?;
                        let main = tiff
                            .ifds
                            .get(*ifd)
                            .ok_or_else(|| Error::Corrupt(format!("IFD {ifd} missing in file {file}")))?;
                        if *level == 0 {
                            return TiffImage::new(tiff, main);
                        }
                        let subs = tiff.sub_ifds(main)?;
                        let sub = subs
                            .get(level - 1)
                            .ok_or_else(|| Error::Corrupt(format!("SubIFD {} missing in IFD {ifd}", level - 1)))?;
                        TiffImage::new(tiff, sub)
                    };
                    load().map_err(|e| e.to_string())
                });
                image.as_ref().map(|i| (*file, i)).map_err(|e| Error::Corrupt(e.clone()))
            }
        }
    }
}

/// Where one plane is stored.
#[derive(Debug, Clone, Copy)]
struct PlaneRef {
    /// Page (index into each level's image list).
    page: usize,
    samples: Samples,
}

#[derive(Debug)]
struct Series {
    /// Moved to [`Tiff::metas`] once the series is complete.
    meta: SeriesMeta,
    /// Images per level and page; `None` for planes missing from the file.
    levels: Vec<Vec<Option<Slot>>>,
    /// Indexed by `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<PlaneRef>>,
    plane_meta: Vec<PlaneMeta>,
    /// File and main-chain IFD whose tags describe the series.
    first_ifd: (usize, usize),
}

impl Tiff {
    /// Opens a TIFF file or an OME-XML companion file. OME-TIFF metadata is
    /// used if present and valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read, [`Error::Corrupt`]
    /// for invalid structure and [`Error::UnsupportedVariant`] if no image of
    /// the file can be decoded.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let mut head = [0u8; 512];
        let n = std::io::Read::read(&mut std::fs::File::open(path)?, &mut head)?;
        if ome::is_ome_xml(&head[..n]) {
            let (files, series, plates, source) = ome::open_companion(path)?;
            return Ok(Self::finish(files, series, plates, Some(source), None));
        }
        let tiff = TiffFile::open(path)?;
        let description = tiff.string(&tiff.ifds[0], tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
        if description.contains("<OME") || path.to_string_lossy().contains(".ome.tif") {
            match ome::open(path, tiff, &description) {
                Ok((files, series, plates, source)) => return Ok(Self::finish(files, series, plates, Some(source), None)),
                // Fall back to plain TIFF if the OME metadata cannot be used.
                Err((tiff, e)) => return Self::open_plain(tiff, &description, Some(e.to_string())),
            }
        }
        Self::open_plain(tiff, &description, None)
    }

    fn open_plain(tiff: TiffFile, description: &str, ome_error: Option<String>) -> Result<Self> {
        let mut series = plain_series(&tiff, 0)?;
        if let Some(ij) = imagej::parse(description) {
            if let [s] = series.as_mut_slice() {
                imagej::apply(&ij, s, &tiff)?;
            }
        }
        Ok(Self::finish(vec![ome::LazyFile::ready(tiff)], series, Vec::new(), None, ome_error))
    }

    fn finish(
        files: Vec<ome::LazyFile>,
        mut series: Vec<Series>,
        plates: Vec<PlateMeta>,
        ome: Option<ome::Source>,
        ome_error: Option<String>,
    ) -> Self {
        let metas = series.iter_mut().map(|s| std::mem::take(&mut s.meta)).collect();
        Self { files, series, metas, plates, ome, ome_error }
    }
}

impl Backend for Tiff {
    fn format(&self) -> &'static FormatInfo {
        if self.ome.is_some() { &OME_FORMAT } else { &FORMAT }
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let index = plane_index(&self.metas[series], plane);
        Ok(self.series[series].plane_meta.get(index).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let (file, ifd) = self.series[series].first_ifd;
        let mut meta = OriginalMeta::default();
        if let Ok(tiff) = self.files[file].get() {
            if let Some(ifd) = tiff.ifds.get(ifd) {
                meta.pairs = original_meta(tiff, ifd)?;
                let description = tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
                if let Some(ij) = imagej::parse(&description) {
                    meta.pairs.extend(ij.pairs);
                } else if !description.is_empty() && self.ome.is_none() {
                    meta.documents.push(("ImageDescription".into(), description));
                }
            }
        }
        if let Some(e) = &self.ome_error {
            meta.pairs.push(("OME-XML error".into(), e.clone()));
        }
        if let Some(source) = &self.ome {
            ome::original(source, &self.files, series, &mut meta)?;
        }
        Ok(meta)
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let s = &self.series[series];
        let slot = s.planes[plane_index(&self.metas[series], plane)].and_then(|p| s.levels[level][p.page].as_ref().map(|slot| (p, slot)));
        let Some((p, slot)) = slot else {
            // Planes missing from the file read as zeros.
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let (file, image) = slot.get(&self.files)?;
        image.read_native(self.files[file].get()?, region, p.samples, sink)
    }
}

fn plane_index(meta: &SeriesMeta, p: Plane) -> usize {
    p.c as usize + meta.size_c as usize * (p.z as usize + meta.size_z as usize * p.t as usize)
}

/// Bit 0 of NewSubfileType: reduced-resolution version of another image.
const REDUCED: u64 = 1;
/// Bit 2 of NewSubfileType: transparency mask.
const MASK: u64 = 4;

/// Builds series from the main IFD chain of a plain TIFF file.
fn plain_series(tiff: &TiffFile, file: usize) -> Result<Vec<Series>> {
    // Pages with their main-chain IFD index.
    let mut groups: Vec<(Vec<TiffImage>, Vec<usize>)> = Vec::new();
    // Reduced-resolution IFDs following a single-page group, as levels.
    let mut reduced: Vec<Vec<TiffImage>> = Vec::new();
    let mut first_error = None;
    for (index, ifd) in tiff.ifds.iter().enumerate() {
        let subfile = tiff.uint(ifd, tag::NEW_SUBFILE_TYPE)?.unwrap_or(0);
        if subfile & MASK != 0 {
            continue;
        }
        let image = match TiffImage::new(tiff, ifd) {
            Ok(i) => i,
            Err(e) => {
                first_error.get_or_insert(e);
                continue;
            }
        };
        if subfile & REDUCED != 0 {
            if let Some((pages, _)) = groups.last() {
                let base = reduced.last().and_then(|r| r.last()).unwrap_or(&pages[0]);
                if pages.len() == 1 && image.width < base.width && image.height <= base.height && image.spp == base.spp {
                    reduced.last_mut().expect("one list per group").push(image);
                    continue;
                }
            }
        }
        match groups.last_mut() {
            Some((pages, ifds)) if pages[0].same_layout(&image) && reduced.last().is_some_and(|r| r.is_empty()) => {
                pages.push(image);
                ifds.push(index);
            }
            _ => {
                groups.push((vec![image], vec![index]));
                reduced.push(Vec::new());
            }
        }
    }
    if groups.is_empty() {
        return Err(first_error.unwrap_or_else(|| Error::Corrupt("TIFF without decodable image".into())));
    }

    let mut series = Vec::new();
    for ((pages, ifds), reduced) in groups.into_iter().zip(reduced) {
        let mut levels = vec![pages];
        if !reduced.is_empty() {
            levels.extend(reduced.into_iter().map(|i| vec![i]));
        } else {
            let main: Vec<&Ifd> = ifds.iter().map(|&i| &tiff.ifds[i]).collect();
            levels.extend(sub_ifd_levels(tiff, &main, &levels[0])?);
        }
        series.push(make_series(tiff, file, ifds[0], levels)?);
    }
    Ok(series)
}

/// Pyramid levels from the SubIFDs of each page, if all pages agree.
fn sub_ifd_levels(tiff: &TiffFile, ifds: &[&Ifd], pages: &[TiffImage]) -> Result<Vec<Vec<TiffImage>>> {
    let mut levels: Vec<Vec<TiffImage>> = Vec::new();
    for (p, ifd) in ifds.iter().enumerate() {
        let Ok(subs) = tiff.sub_ifds(ifd) else { return Ok(Vec::new()) };
        if p == 0 {
            levels = subs.iter().map(|_| Vec::with_capacity(ifds.len())).collect();
        }
        if subs.len() != levels.len() {
            return Ok(Vec::new());
        }
        for (l, sub) in subs.iter().enumerate() {
            match TiffImage::new(tiff, sub) {
                Ok(img) if img.spp == pages[p].spp && img.pixel_type == pages[p].pixel_type => levels[l].push(img),
                _ => return Ok(Vec::new()),
            }
        }
    }
    // Levels must shrink.
    let mut width = pages[0].width;
    for level in &levels {
        if level[0].width >= width || level.iter().any(|i| !i.same_layout(&level[0])) {
            return Ok(Vec::new());
        }
        width = level[0].width;
    }
    Ok(levels)
}

/// Level metadata of one image; strips count as tiles of full width.
pub(super) fn level_meta(image: &TiffImage, full_width: u32) -> LevelMeta {
    let tile = (image.tiled || image.chunk_h < image.height).then_some((image.chunk_w, image.chunk_h));
    LevelMeta {
        size_x: image.width as u64,
        size_y: image.height as u64,
        tile,
        downsample: full_width as f64 / image.width as f64,
    }
}

/// Creates a series with one page per time point.
fn make_series(tiff: &TiffFile, file: usize, ifd_index: usize, levels: Vec<Vec<TiffImage>>) -> Result<Series> {
    let ifd = &tiff.ifds[ifd_index];
    let first = &levels[0][0];
    let channels = first.channels() as u32;
    let pages = levels[0].len();
    let gray = first.samples_out == 1;
    let samples = |c: u32| if gray { Samples::One(c as u16) } else { Samples::All };
    let planes = (0..pages)
        .flat_map(|page| (0..channels).map(move |c| Some(PlaneRef { page, samples: samples(c) })))
        .collect();
    let channel = ChannelMeta { lut: first.lut.clone(), color: first.display_color, ..Default::default() };
    let meta = SeriesMeta {
        size_c: channels,
        size_z: 1,
        size_t: pages as u32,
        pixel_type: first.pixel_type,
        significant_bits: (!matches!(first.significant_bits, 8 | 16 | 32 | 64)).then_some(first.significant_bits as u8),
        samples_per_pixel: first.samples_out,
        levels: levels.iter().map(|l| level_meta(&l[0], first.width)).collect(),
        physical_size: resolution(tiff, ifd)?,
        channels: vec![channel; channels as usize],
        acquisition_date: tiff.string(ifd, tag::DATE_TIME)?.and_then(|d| tiff_date(&d)),
        description: tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.filter(|d| !d.is_empty() && !d.contains("<OME")),
        ..Default::default()
    };
    let levels = levels
        .into_iter()
        .map(|l| l.into_iter().map(|image| Some(Slot::ready(file, image))).collect())
        .collect();
    Ok(Series { meta, levels, planes, plane_meta: Vec::new(), first_ifd: (file, ifd_index) })
}

/// Pixel size from XResolution, YResolution and ResolutionUnit.
pub(super) fn resolution(tiff: &TiffFile, ifd: &Ifd) -> Result<PhysicalSize> {
    let unit_nm = match tiff.uint(ifd, tag::RESOLUTION_UNIT)?.unwrap_or(2) {
        2 => 25.4e6,
        3 => 1e7,
        _ => return Ok(PhysicalSize::default()),
    };
    let size = |t: u16| -> Result<Option<f64>> {
        Ok(tiff.floats(ifd, t)?.and_then(|v| v.first().copied()).filter(|r| r.is_finite() && *r > 0.0).map(|r| unit_nm / r))
    };
    Ok(PhysicalSize { x_nm: size(tag::X_RESOLUTION)?, y_nm: size(tag::Y_RESOLUTION)?, z_nm: None })
}

/// Converts a TIFF date "YYYY:MM:DD HH:MM:SS" to ISO 8601.
pub(super) fn tiff_date(d: &str) -> Option<String> {
    let b = d.trim().as_bytes();
    if b.len() < 19 || b[4] != b':' || b[7] != b':' || b[10] != b' ' || !d.is_char_boundary(19) {
        return None;
    }
    Some(format!("{}-{}-{}T{}", &d[0..4], &d[5..7], &d[8..10], &d[11..19]))
}

/// Selected tags as key-value pairs for [`OriginalMeta::pairs`].
pub(super) fn original_meta(tiff: &TiffFile, ifd: &Ifd) -> Result<Vec<(String, String)>> {
    let mut v = vec![
        ("Byte order".to_string(), if tiff.little { "little-endian" } else { "big-endian" }.to_string()),
        ("BigTIFF".to_string(), tiff.big.to_string()),
    ];
    for e in ifd.entries() {
        let name = match e.tag {
            tag::IMAGE_WIDTH => "ImageWidth",
            tag::IMAGE_LENGTH => "ImageLength",
            tag::BITS_PER_SAMPLE => "BitsPerSample",
            tag::COMPRESSION => "Compression",
            tag::PHOTOMETRIC => "PhotometricInterpretation",
            tag::SAMPLES_PER_PIXEL => "SamplesPerPixel",
            tag::PLANAR_CONFIGURATION => "PlanarConfiguration",
            tag::PREDICTOR => "Predictor",
            tag::SAMPLE_FORMAT => "SampleFormat",
            tag::RESOLUTION_UNIT => "ResolutionUnit",
            tag::SOFTWARE => "Software",
            tag::DATE_TIME => "DateTime",
            tag::MAKE => "Make",
            tag::MODEL => "Model",
            tag::X_RESOLUTION => "XResolution",
            tag::Y_RESOLUTION => "YResolution",
            _ => continue,
        };
        let value = match e.tag {
            tag::SOFTWARE | tag::DATE_TIME | tag::MAKE | tag::MODEL => tiff.string(ifd, e.tag)?.unwrap_or_default(),
            tag::X_RESOLUTION | tag::Y_RESOLUTION => join(&tiff.floats(ifd, e.tag)?.unwrap_or_default()),
            _ => join(&tiff.uints(ifd, e.tag)?.unwrap_or_default()),
        };
        v.push((name.to_string(), value));
    }
    Ok(v)
}

fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
}

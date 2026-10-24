//! # vsi
//!
//! Olympus/Evident cellSens and VS200 VSI reader.
//!
//! A `.vsi` file is a little-endian TIFF whose first IFD is a preview,
//! followed by a tagged record tree with all metadata. Pixels are in ETS
//! files in `_<stem>_/stack<id>/`, or, in single-file datasets, in further
//! IFDs of the `.vsi` itself. Every image stack becomes a series, except the
//! slide label and overview (associated images `Label` and `Macro`) and
//! focus maps, focus points and masks (ignored). The preview is the
//! `Thumbnail`.
//!
//! Provenance: OpenReadout VSI format notes and bioio-vsi format notes
//! (`doc/olympus-vsi/`, written from sample files and permissively licensed
//! readers); own analysis of sample files. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod ets;
mod tree;

use std::fs::File;
use std::path::{Path, PathBuf};

use ets::Ets;
use tree::Field;

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta,
    PhysicalSize, Plane, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Vsi, "Olympus VSI", "1");

/// The record tree starts right after the 8-byte TIFF header.
const TREE_OFFSET: u64 = 8;
/// Longest side of an associated image; larger ones are read at a
/// coarser pyramid level.
const ASSOCIATED_MAX: u64 = 4096;

/// Tags of the record tree.
mod tag {
    pub const COLLECTION: u32 = 2000;
    pub const STACK: u32 = 2001;
    pub const PLANE: u32 = 2002;
    pub const DIM_SIZES: u32 = 2003;
    pub const SETTINGS: u32 = 2005;
    pub const DIMENSION: u32 = 2007;
    pub const DIM_ENTRY: u32 = 2008;
    pub const Z_STEP: u32 = 2013;
    pub const TIME: u32 = 2015;
    pub const LAYOUT: u32 = 2018;
    pub const PIXEL_SIZE: u32 = 2019;
    pub const PIXEL_UNIT: u32 = 2020;
    pub const CHANNEL_NAME: u32 = 2021;
    pub const DIM_KIND: u32 = 2023;
    pub const NAME: u32 = 2030;
    pub const LAYOUT_ALT: u32 = 2037;
    pub const DEVICES: u32 = 2043;
    pub const IMAGE_RECT: u32 = 2053;
    pub const IMAGE_TYPE: u32 = 2074;
    pub const TILE_ORIGIN: u32 = 2410;
    pub const EMISSION: u32 = 2417;
    pub const SETTING_NAME: u32 = 2419;
    pub const OBJECTIVE_SET: u32 = 120114;
    pub const OBJ_MAGNIFICATION: u32 = 120060;
    pub const OBJ_NA: u32 = 120061;
    pub const OBJ_NAME: u32 = 120063;
    pub const DOCUMENT: u32 = 2;
    pub const PRODUCT: u32 = 34;
    pub const VERSION: u32 = 35;
}

/// Image types of `2005/2074`.
mod image_type {
    pub const OVERVIEW: i64 = 1;
    pub const FOCUS_MAP: i64 = 4;
    pub const LABEL: i64 = 256;
    pub const FOCUS_POINTS: i64 = 1024;
    pub const SAMPLE_MASK: i64 = 8_388_608;
}

/// Returns `true` if `head` starts like a `.vsi` file: TIFF header followed
/// by a record volume.
pub(super) fn matches(head: &[u8]) -> bool {
    head.len() >= 12 && head[..4] == [b'I', b'I', 42, 0] && u16::from_le_bytes([head[10], head[11]]) == 21321
}

/// Olympus VSI dataset.
#[derive(Debug)]
pub(super) struct Vsi {
    path: PathBuf,
    images: Vec<Image>,
    metas: Vec<SeriesMeta>,
    associated: Vec<(AssociatedKind, Image)>,
    associated_kinds: Vec<AssociatedKind>,
    thumbnail: Option<TiffImage>,
    tiff: Option<TiffFile>,
}

/// Where a stack's pixels are.
#[derive(Debug)]
enum Pixels {
    /// ETS tiles with the tile-grid origin of level 0.
    Ets { ets: Ets, origin: (i64, i64), sizes: Vec<(u64, u64)> },
    /// IFDs of the `.vsi`, one per plane in plane order.
    Ifds(Vec<TiffImage>),
}

/// How the series dimensions map to the stack's coordinate slots.
#[derive(Debug, Clone, Default)]
struct DimMap {
    /// Slot of C, Z and T, if present.
    c: Option<usize>,
    z: Option<usize>,
    t: Option<usize>,
    /// Size of every slot.
    sizes: Vec<u32>,
}

#[derive(Debug)]
struct Image {
    pixels: Pixels,
    dims: DimMap,
}

impl Vsi {
    /// Opens a `.vsi` file with its ETS files.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid record tree or ETS files
    /// and [`Error::UnsupportedVariant`] for unknown sample types.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let root = Field { tag: 0, index: None, kind: 0, inline: None, data: Vec::new(), children: tree::parse(&file, len, TREE_OFFSET)? };
        let collection = root.get(tag::COLLECTION).ok_or_else(|| Error::Corrupt("VSI without image collection".into()))?;
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ets_dir = path.with_file_name(format!("_{stem}_"));
        let tiff = TiffFile::open(path).ok();
        let thumbnail = tiff.as_ref().and_then(|t| TiffImage::new(t, t.ifds.first()?).ok());

        let mut images = Vec::new();
        let mut metas = Vec::new();
        let mut associated = Vec::new();
        // IFDs after the preview, for single-file datasets.
        let mut ifds = tiff.as_ref().map(|t| (1..t.ifds.len()).collect::<Vec<_>>()).unwrap_or_default().into_iter();
        for stack in collection.all(tag::STACK) {
            let settings = stack.get(tag::SETTINGS);
            let kind = settings.and_then(|s| s.get(tag::IMAGE_TYPE)).and_then(Field::int);
            if matches!(kind, Some(image_type::FOCUS_MAP | image_type::FOCUS_POINTS | image_type::SAMPLE_MASK)) {
                continue;
            }
            let dims = dim_map(stack);
            let id = stack.index.unwrap_or(0);
            let pixels = match find_ets(&ets_dir.join(format!("stack{id}"))) {
                Some(ets_path) => {
                    let ets = Ets::open(&ets_path)?;
                    let origin = tile_origin(stack, &ets);
                    let rect = image_rect(stack);
                    let sizes = level_sizes(&ets, origin, rect)?;
                    Pixels::Ets { ets, origin, sizes }
                }
                None if !ets_dir.is_dir() => {
                    // Single-file dataset: the planes are the next IFDs.
                    let planes = dims.sizes.iter().map(|&s| s.max(1) as usize).product::<usize>().max(1);
                    let Some(tiff) = &tiff else { continue };
                    let list: Vec<TiffImage> = ifds.by_ref().take(planes).filter_map(|i| TiffImage::new(tiff, &tiff.ifds[i]).ok()).collect();
                    if list.len() != planes {
                        continue;
                    }
                    Pixels::Ifds(list)
                }
                None => continue,
            };
            let image = Image { pixels, dims };
            match kind {
                Some(image_type::LABEL) => associated.push((AssociatedKind::Label, image)),
                Some(image_type::OVERVIEW) => associated.push((AssociatedKind::Macro, image)),
                _ => {
                    metas.push(series_meta(&root, stack, &image)?);
                    images.push(image);
                }
            }
        }
        if images.is_empty() {
            return Err(Error::Corrupt(format!("{}: no image stack with pixel data", path.display())));
        }
        let mut associated_kinds: Vec<AssociatedKind> = associated.iter().map(|(k, _)| k.clone()).collect();
        if thumbnail.is_some() {
            associated_kinds.push(AssociatedKind::Thumbnail);
        }
        Ok(Self { path: path.to_path_buf(), images, metas, associated, associated_kinds, thumbnail, tiff })
    }

    fn tree(&self) -> Result<Field> {
        let file = File::open(&self.path)?;
        let len = file.metadata()?.len();
        Ok(Field { tag: 0, index: None, kind: 0, inline: None, data: Vec::new(), children: tree::parse(&file, len, TREE_OFFSET)? })
    }
}

/// The first `frame_t*.ets` file in a stack directory.
fn find_ets(dir: &Path) -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            n.starts_with("frame_t") && n.ends_with(".ets")
        })
        .collect();
    names.sort();
    names.into_iter().next()
}

/// Kind and size of each coordinate slot of a stack.
fn dim_map(stack: &Field) -> DimMap {
    let sizes: Vec<u32> = stack.get(tag::DIM_SIZES).map(|f| f.ints().into_iter().map(|v| v.max(1) as u32).collect()).unwrap_or_default();
    let mut map = DimMap { sizes: sizes.clone(), ..Default::default() };
    for (slot, dim) in stack.all(tag::DIMENSION).enumerate() {
        if slot >= sizes.len() {
            break;
        }
        let kind = dim.get(tag::DIM_ENTRY).and_then(|e| e.get(tag::DIM_KIND)).and_then(Field::int);
        let target = match kind {
            Some(1) => &mut map.z,
            Some(4) => &mut map.c,
            // Time, and other kinds with more than one entry, become T.
            Some(_) | None if sizes[slot] > 1 => &mut map.t,
            _ => continue,
        };
        if target.is_none() {
            *target = Some(slot);
        }
    }
    map
}

/// The layout record of the first plane that has one.
fn layout(stack: &Field) -> Option<&Field> {
    stack.all(tag::PLANE).find_map(|p| p.get(tag::LAYOUT).or_else(|| p.get(tag::LAYOUT_ALT)))
}

/// Level-0 image size from the record tree, if stored.
fn image_rect(stack: &Field) -> Option<(u64, u64)> {
    let r = layout(stack)?.get(tag::IMAGE_RECT)?.ints();
    (r.len() >= 4 && r[2] > 0 && r[3] > 0).then(|| (r[2] as u64, r[3] as u64))
}

/// Offset of the tile grid: the ETS header value if set, else record 2410.
fn tile_origin(stack: &Field, ets: &Ets) -> (i64, i64) {
    if ets.origin != (0, 0) {
        return ets.origin;
    }
    let v = layout(stack).and_then(|l| l.get(tag::TILE_ORIGIN)).map(Field::ints).unwrap_or_default();
    (v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0))
}

/// Origin of the tile grid at `level`: level 0 divided by 2^level, rounded
/// to the nearest pixel, halves toward zero.
fn level_origin(origin: (i64, i64), level: usize) -> (i64, i64) {
    let scale = |v: i64| {
        let d = 1i64 << level.min(62);
        let q = v / d;
        let r = v % d;
        if r.abs() * 2 > d { q + r.signum() } else { q }
    };
    (scale(origin.0), scale(origin.1))
}

/// Pixel size of every pyramid level: halved per level, clipped to the
/// stored tiles.
fn level_sizes(ets: &Ets, origin: (i64, i64), rect: Option<(u64, u64)>) -> Result<Vec<(u64, u64)>> {
    let full = match (ets.sizes.get(..2), rect) {
        (Some([w, h]), _) if *w > 0 && *h > 0 => (*w as u64, *h as u64),
        (_, Some(r)) => r,
        _ => {
            // Without sizes, the extent of the level-0 tiles.
            let l = ets.levels.first().ok_or_else(|| Error::Corrupt("ETS without tiles".into()))?;
            ((l.col0 + l.cols as i64) as u64 * ets.tile_w as u64, (l.row0 + l.rows as i64) as u64 * ets.tile_h as u64)
        }
    };
    let mut sizes = vec![full];
    for level in 1..ets.levels.len() {
        let prev = sizes[level - 1];
        let (ox, oy) = level_origin(origin, level);
        let l = &ets.levels[level];
        let extent_x = ((l.col0 + l.cols as i64) * ets.tile_w as i64 + ox).max(1) as u64;
        let extent_y = ((l.row0 + l.rows as i64) * ets.tile_h as i64 + oy).max(1) as u64;
        sizes.push((prev.0.div_ceil(2).min(extent_x), prev.1.div_ceil(2).min(extent_y)));
    }
    Ok(sizes)
}

/// Nanometres per unit of a VSI unit string such as `10^-6m^1`.
fn unit_nm(unit: &str) -> Option<f64> {
    let exp = unit.strip_prefix("10^")?.strip_suffix("m^1")?.parse::<i32>().ok()?;
    Some(10f64.powi(exp + 9))
}

/// Formats Unix seconds as ISO 8601 (UTC).
fn iso_date(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

fn series_meta(root: &Field, stack: &Field, image: &Image) -> Result<SeriesMeta> {
    let settings = stack.get(tag::SETTINGS);
    let dims = &image.dims;
    let size = |slot: Option<usize>| slot.and_then(|s| dims.sizes.get(s)).copied().unwrap_or(1);
    let (pixel_type, spp, levels) = match &image.pixels {
        Pixels::Ets { ets, sizes, .. } => {
            let levels = sizes
                .iter()
                .enumerate()
                .map(|(l, &(w, h))| LevelMeta {
                    size_x: w,
                    size_y: h,
                    tile: Some((ets.tile_w, ets.tile_h)),
                    downsample: (1u64 << l) as f64,
                })
                .collect();
            (ets.pixel_type, ets.spp, levels)
        }
        Pixels::Ifds(list) => {
            let f = &list[0];
            let tile = (f.tiled || f.chunk_h < f.height).then_some((f.chunk_w, f.chunk_h));
            (f.pixel_type, f.samples_out, vec![LevelMeta { size_x: f.width as u64, size_y: f.height as u64, tile, downsample: 1.0 }])
        }
    };
    // Pixel size, in the unit of 2020 (micrometres in every known file).
    let unit = settings.and_then(|s| s.get(tag::PIXEL_UNIT)).and_then(Field::text);
    let nm = unit.as_deref().and_then(unit_nm).unwrap_or(1e3);
    let px = settings.and_then(|s| s.get(tag::PIXEL_SIZE)).map(Field::floats).unwrap_or_default();
    let z_step = dims
        .z
        .and_then(|slot| stack.all(tag::DIMENSION).nth(slot))
        .and_then(|d| d.get(tag::DIM_ENTRY))
        .and_then(|e| e.get(tag::Z_STEP))
        .and_then(Field::float)
        .map(|v| v.abs() * 1e3);
    let size_c = size(dims.c);
    let channel_entries: Vec<&Field> = dims
        .c
        .and_then(|slot| stack.all(tag::DIMENSION).nth(slot))
        .map(|d| d.all(tag::DIM_ENTRY).collect())
        .unwrap_or_default();
    let setting_name = settings.and_then(|s| s.get(tag::SETTING_NAME)).and_then(Field::text);
    let channels = (0..size_c as usize)
        .map(|c| {
            let e = channel_entries.get(c);
            ChannelMeta {
                name: e
                    .and_then(|e| e.get(tag::CHANNEL_NAME).or_else(|| e.get(tag::SETTING_NAME)))
                    .and_then(Field::text)
                    .or_else(|| (size_c == 1).then(|| setting_name.clone()).flatten()),
                emission_wavelength_nm: e.and_then(|e| e.get(tag::EMISSION)).and_then(Field::float),
                ..Default::default()
            }
        })
        .collect();
    let objective = settings.and_then(|s| {
        s.get(tag::DEVICES)?.children.iter().find_map(|d| {
            let o = d.get(tag::OBJECTIVE_SET)?;
            let m = o.get(tag::OBJ_MAGNIFICATION).and_then(Field::float);
            m.map(|m| ObjectiveMeta {
                model: o.get(tag::OBJ_NAME).and_then(Field::text),
                magnification: Some(m),
                numerical_aperture: o.get(tag::OBJ_NA).and_then(Field::float),
                ..Default::default()
            })
        })
    });
    let _ = root;
    Ok(SeriesMeta {
        name: settings.and_then(|s| s.get(tag::NAME)).and_then(Field::text),
        acquisition_date: settings.and_then(|s| s.get(tag::TIME)).and_then(Field::int).filter(|&t| t > 0).map(iso_date),
        size_c,
        size_z: size(dims.z),
        size_t: size(dims.t),
        pixel_type,
        samples_per_pixel: spp,
        levels,
        physical_size: PhysicalSize {
            x_nm: px.first().map(|v| v * nm),
            y_nm: px.get(1).or(px.first()).map(|v| v * nm),
            z_nm: z_step,
        },
        channels,
        objective,
        ..Default::default()
    })
}

impl Image {
    /// Slot values of plane (c, z, t), slot 0 first.
    fn slot_values(&self, plane: Plane, nslots: usize) -> Vec<i64> {
        let mut v = vec![0i64; nslots];
        for (slot, value) in [(self.dims.c, plane.c), (self.dims.z, plane.z), (self.dims.t, plane.t)] {
            if let Some(s) = slot.filter(|&s| s < nslots) {
                v[s] = value as i64;
            }
        }
        v
    }

    /// Delivers `region` of `plane` at `level` to `sink`.
    fn read(&self, tiff: Option<&TiffFile>, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        match &self.pixels {
            Pixels::Ifds(list) => {
                let values = self.slot_values(plane, self.dims.sizes.len());
                let index = ets::plane_index(&values, &self.dims.sizes) as usize;
                let image = list.get(index).ok_or_else(|| Error::Corrupt(format!("plane {index} missing")))?;
                let tiff = tiff.ok_or_else(|| Error::Corrupt("VSI without TIFF structure".into()))?;
                let samples = if image.samples_out == 1 { Samples::One(0) } else { Samples::All };
                image.read_native(tiff, region, samples, sink)
            }
            Pixels::Ets { ets, origin, .. } => {
                let plane_index = ets::plane_index(&self.slot_values(plane, ets.slots.len()), &ets.slots);
                let (ox, oy) = level_origin(*origin, level);
                let (tw, th) = (ets.tile_w as i64, ets.tile_h as i64);
                let (x0, y0) = (region.x as i64, region.y as i64);
                let (x1, y1) = (x0 + region.width as i64, y0 + region.height as i64);
                let px = ets.spp as usize * ets.pixel_type.bytes();
                let background = ets.background();
                scratch::with(ets.tile_bytes(), |tile| {
                    for row in (y0 - oy).div_euclid(th)..=(y1 - 1 - oy).div_euclid(th) {
                        for col in (x0 - ox).div_euclid(tw)..=(x1 - 1 - ox).div_euclid(tw) {
                            let present = ets.read_tile(level, plane_index, col, row, tile)?;
                            let (tx, ty) = (col * tw + ox, row * th + oy);
                            let (cx0, cx1) = (x0.max(tx), x1.min(tx + tw));
                            let w = (cx1 - cx0) as usize;
                            for y in y0.max(ty)..y1.min(ty + th) {
                                let start = ((y - ty) * tw + (cx0 - tx)) as usize * px;
                                sink.write_with((y - y0) as usize, (cx0 - x0) as usize, w, |dst| {
                                    if present {
                                        dst.copy_from_slice(&tile[start..start + w * px]);
                                    } else {
                                        for p in dst.chunks_exact_mut(px) {
                                            p.copy_from_slice(background);
                                        }
                                    }
                                    Ok(())
                                })?;
                            }
                        }
                    }
                    Ok(())
                })
            }
        }
    }
}

impl Backend for Vsi {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let root = self.tree()?;
        let mut meta = OriginalMeta::default();
        if let Some(doc) = root.get(tag::DOCUMENT) {
            for (t, name) in [(tag::PRODUCT, "Product"), (tag::VERSION, "Version")] {
                if let Some(v) = doc.get(t).and_then(Field::text) {
                    meta.pairs.push((name.into(), v));
                }
            }
        }
        if let Some(name) = &self.metas[series].name {
            meta.pairs.push(("Image name".into(), name.clone()));
        }
        Ok(meta)
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        if *kind == AssociatedKind::Thumbnail {
            let (Some(thumb), Some(tiff)) = (&self.thumbnail, &self.tiff) else {
                return Err(Error::OutOfRange("no thumbnail".into()));
            };
            let region = Region::new(0, 0, thumb.width as u64, thumb.height as u64);
            return native_image(region, thumb.samples_out, thumb.pixel_type, |sink| {
                let samples = if thumb.samples_out == 1 { Samples::One(0) } else { Samples::All };
                thumb.read_native(tiff, region, samples, sink)
            });
        }
        let (_, image) = self
            .associated
            .iter()
            .find(|(k, _)| k == kind)
            .ok_or_else(|| Error::OutOfRange(format!("no associated image {kind:?}")))?;
        let Pixels::Ets { ets, sizes, .. } = &image.pixels else {
            return Err(Error::UnsupportedVariant("associated image without ETS".into()));
        };
        // The finest level within the size limit, else the coarsest.
        let level = sizes.iter().position(|&(w, h)| w.max(h) <= ASSOCIATED_MAX).unwrap_or(sizes.len() - 1);
        let (w, h) = sizes[level];
        let region = Region::new(0, 0, w, h);
        native_image(region, ets.spp, ets.pixel_type, |sink| image.read(None, level, Plane::default(), region, sink))
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        self.images[series].read(self.tiff.as_ref(), level, plane, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_dates_and_origins() {
        assert_eq!(unit_nm("10^-6m^1"), Some(1e3));
        assert_eq!(unit_nm("10^-9m^1"), Some(1.0));
        assert_eq!(iso_date(1_770_728_975), "2026-02-10T13:09:35Z");
        assert_eq!(iso_date(0), "1970-01-01T00:00:00Z");
        assert_eq!(level_origin((-42, -1), 1), (-21, 0));
        assert_eq!(level_origin((-43, 13306), 1), (-21, 6653));
        assert_eq!(level_origin((-3, 0), 1), (-1, 0));
    }
}

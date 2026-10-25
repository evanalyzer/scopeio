//! # dicom
//!
//! DICOM reader: whole-slide microscopy (VL WSI) pyramids and other
//! single- or multi-frame images.
//!
//! A whole-slide image is a set of files sharing a Series Instance UID: one
//! file per pyramid level (Image Type value 3 `VOLUME`), plus `THUMBNAIL`,
//! `LABEL` and `OVERVIEW` images, returned as associated images. Each frame
//! is a tile of the total pixel matrix, ordered row-major (`TILED_FULL`) or
//! placed by its per-frame Plane Position (Slide) (`TILED_SPARSE`); tiles
//! no frame covers read as white (RGB) or zero. Focal planes are z, optical
//! paths are channels. Other images are one series whose frames are z.
//!
//! Frames are native (uncompressed), RLE, JPEG (baseline and extended) or
//! JPEG 2000 (including HTJ2K); each is decoded when a read touches it.
//!
//! Provenance: DICOM PS3.3 (C.7.6 image modules, C.8.12.4 whole slide
//! microscopy), PS3.5 (encoding, Annex G RLE), PS3.10 (file format) in
//! `doc/dicom/`; checked against OpenSlide and Bio-Formats as black boxes
//! on the OpenSlide test data (CC0). No GPL/LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod elements;

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use elements::{Dataset, Value};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane,
    Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Dicom, "DICOM", "1").extensions(&["dcm", "dicom"]);

/// Returns `true` if `head` starts like a DICOM Part 10 file.
pub(super) fn matches(head: &[u8]) -> bool {
    elements::matches(head)
}

// Tags.
const IMAGE_TYPE: u32 = 0x0008_0008;
const MODALITY: u32 = 0x0008_0060;
const SERIES_UID: u32 = 0x0020_000E;
const SERIES_DESCRIPTION: u32 = 0x0008_103E;
const ACQUISITION_DATETIME: u32 = 0x0008_002A;
const CONTENT_DATE: u32 = 0x0008_0023;
const CONTENT_TIME: u32 = 0x0008_0033;
const SAMPLES: u32 = 0x0028_0002;
const PHOTOMETRIC: u32 = 0x0028_0004;
const PLANAR: u32 = 0x0028_0006;
const FRAMES: u32 = 0x0028_0008;
const ROWS: u32 = 0x0028_0010;
const COLUMNS: u32 = 0x0028_0011;
const PIXEL_SPACING: u32 = 0x0028_0030;
const BITS_ALLOCATED: u32 = 0x0028_0100;
const BITS_STORED: u32 = 0x0028_0101;
const PIXEL_REPRESENTATION: u32 = 0x0028_0103;
const TOTAL_COLUMNS: u32 = 0x0048_0006;
const TOTAL_ROWS: u32 = 0x0048_0007;
const FOCAL_PLANES: u32 = 0x0048_0303;
const ORGANIZATION: u32 = 0x0020_9311;
const SHARED_GROUPS: u32 = 0x5200_9229;
const PER_FRAME_GROUPS: u32 = 0x5200_9230;
const PIXEL_MEASURES: u32 = 0x0028_9110;
const SPACING_BETWEEN_SLICES: u32 = 0x0018_0088;
const PLANE_POSITION_SLIDE: u32 = 0x0048_021A;
const COLUMN_POSITION: u32 = 0x0048_021E;
const ROW_POSITION: u32 = 0x0048_021F;
const Z_OFFSET: u32 = 0x0040_074A;
const OPTICAL_PATH_SEQUENCE: u32 = 0x0048_0105;
const OPTICAL_PATH_ID: u32 = 0x0048_0106;
const OPTICAL_PATH_DESCRIPTION: u32 = 0x0048_0107;
const OPTICAL_PATH_IDENTIFICATION: u32 = 0x0048_0207;
const EMISSION_CENTER: u32 = 0x0022_0057;
const EXCITATION_CENTER: u32 = 0x0022_0055;
const EXTENDED_OFFSETS: u32 = 0x7FE0_0001;
const EXTENDED_LENGTHS: u32 = 0x7FE0_0002;
const OBJECTIVE_POWER: u32 = 0x0048_0112;
/// Largest frame decoded, in bytes.
const MAX_FRAME: usize = 1 << 28;

/// How the frames of an image are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coding {
    Native,
    Rle,
    Jpeg,
    Jpeg2000,
}

/// Where the bytes of one frame are.
#[derive(Debug, Clone)]
enum FrameData {
    /// One contiguous range.
    Range(u64, u64),
    /// Several fragments to concatenate.
    Parts(Vec<(u64, u64)>),
}

/// One image (DICOM instance) of the set.
#[derive(Debug)]
struct Image {
    file: File,
    coding: Coding,
    /// Matrix size and frame (tile) size.
    width: u64,
    height: u64,
    tile_w: u32,
    tile_h: u32,
    spp: u8,
    planar: bool,
    /// JPEG frames hold YCbCr to convert.
    #[cfg_attr(not(feature = "jpeg"), allow(dead_code))]
    ycbcr: bool,
    pixel_type: PixelType,
    frames: Vec<FrameData>,
    /// Frame of tile (col, row) for plane index `c + size_c * z`, by
    /// `((plane, row) * across + col)`; `u32::MAX` if none.
    grid: Vec<u32>,
    across: u32,
    down: u32,
}

impl Image {
    fn frame_bytes(&self) -> usize {
        self.tile_w as usize * self.tile_h as usize * self.spp as usize * self.pixel_type.bytes()
    }

    /// Decodes frame `k` into `out` (`frame_bytes()`, interleaved samples).
    fn decode(&self, k: usize, out: &mut [u8]) -> Result<()> {
        let data = match &self.frames[k] {
            FrameData::Range(at, len) => {
                if *len > MAX_FRAME as u64 {
                    return Err(Error::Corrupt("DICOM frame too large".into()));
                }
                let mut b = vec![0u8; *len as usize];
                read_exact_at(&self.file, &mut b, *at)?;
                b
            }
            FrameData::Parts(parts) => {
                let total: u64 = parts.iter().map(|p| p.1).sum();
                if total > MAX_FRAME as u64 {
                    return Err(Error::Corrupt("DICOM frame too large".into()));
                }
                let mut b = vec![0u8; total as usize];
                let mut o = 0;
                for &(at, len) in parts {
                    read_exact_at(&self.file, &mut b[o..o + len as usize], at)?;
                    o += len as usize;
                }
                b
            }
        };
        let n = out.len();
        match self.coding {
            Coding::Native => {
                let m = data.len().min(n);
                out[..m].copy_from_slice(&data[..m]);
                out[m..].fill(0);
                if self.planar && self.spp > 1 {
                    deplanarize(out, self.spp as usize, self.pixel_type.bytes());
                }
            }
            Coding::Rle => rle(&data, out, self.spp as usize, self.pixel_type.bytes())?,
            Coding::Jpeg => {
                #[cfg(feature = "jpeg")]
                {
                    use crate::codecs::jpeg::{Color, decode};
                    let color = if self.ycbcr { Color::YcbcrToRgb } else { Color::AsStored };
                    let info = decode(&data, None, color, out)?;
                    if info.width * info.height * info.components < n {
                        out[info.width * info.height * info.components..].fill(0);
                    }
                }
                #[cfg(not(feature = "jpeg"))]
                return Err(Error::UnsupportedVariant("JPEG frames (enable the `jpeg` feature)".into()));
            }
            Coding::Jpeg2000 => {
                #[cfg(feature = "jpeg2000")]
                crate::codecs::jpeg2000::decode(&data, self.pixel_type, out)?;
                #[cfg(not(feature = "jpeg2000"))]
                return Err(Error::UnsupportedVariant("JPEG 2000 frames (enable the `jpeg2000` feature)".into()));
            }
        }
        Ok(())
    }

    /// Delivers `region` of plane `plane` (`c + size_c * z`) to `sink`.
    fn read(&self, plane: usize, region: Region, fill: u8, sink: &mut Sink) -> Result<()> {
        let (tw, th) = (self.tile_w as u64, self.tile_h as u64);
        let px = self.spp as usize * self.pixel_type.bytes();
        let (x0, y0) = (region.x, region.y);
        let (x1, y1) = (x0 + region.width, y0 + region.height);
        scratch::with(self.frame_bytes(), |tile| {
            for ty in y0 / th..y1.div_ceil(th) {
                for tx in x0 / tw..x1.div_ceil(tw) {
                    let frame = self.grid.get(((plane as u64 * self.down as u64 + ty) * self.across as u64 + tx) as usize).copied().unwrap_or(u32::MAX);
                    let present = frame != u32::MAX;
                    if present {
                        self.decode(frame as usize, tile)?;
                    }
                    let (cx0, cx1) = (x0.max(tx * tw), x1.min((tx + 1) * tw));
                    let (cy0, cy1) = (y0.max(ty * th), y1.min((ty + 1) * th));
                    let w = (cx1 - cx0) as usize;
                    for y in cy0..cy1 {
                        let src = (((y - ty * th) * tw + cx0 - tx * tw) as usize) * px;
                        sink.write_with((y - y0) as usize, (cx0 - x0) as usize, w, |d| {
                            if present {
                                d.copy_from_slice(&tile[src..src + w * px]);
                            } else {
                                d.fill(fill);
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

/// Interleaves planar (`RRR…GGG…BBB…`) samples in place.
fn deplanarize(buf: &mut [u8], spp: usize, bytes: usize) {
    let n = buf.len() / (spp * bytes);
    let planar = buf.to_vec();
    for i in 0..n {
        for s in 0..spp {
            let src = (s * n + i) * bytes;
            let dst = (i * spp + s) * bytes;
            buf[dst..dst + bytes].copy_from_slice(&planar[src..src + bytes]);
        }
    }
}

/// Decodes a DICOM RLE frame (PS3.5 Annex G): segments of PackBits data,
/// one per byte of each sample, most significant byte first.
fn rle(data: &[u8], out: &mut [u8], spp: usize, bytes: usize) -> Result<()> {
    let bad = || Error::Corrupt("DICOM RLE frame header".into());
    if data.len() < 64 {
        return Err(bad());
    }
    let count = u32::from_le_bytes(data[..4].try_into().expect("4 bytes")) as usize;
    if count != spp * bytes || count > 15 {
        return Err(bad());
    }
    let offsets: Vec<usize> = (0..count).map(|i| u32::from_le_bytes(data[4 + 4 * i..8 + 4 * i].try_into().expect("4 bytes")) as usize).collect();
    let pixels = out.len() / (spp * bytes);
    let mut seg = vec![0u8; pixels];
    for (i, &start) in offsets.iter().enumerate() {
        let end = offsets.get(i + 1).copied().unwrap_or(data.len()).min(data.len());
        let src = data.get(start..end).ok_or_else(bad)?;
        seg.fill(0);
        let _ = crate::codecs::packbits(src, &mut seg);
        // Segment i: sample i / bytes, byte (bytes - 1 - i % bytes) little-endian.
        let (s, b) = (i / bytes, bytes - 1 - i % bytes);
        for (p, v) in seg.iter().enumerate() {
            out[(p * spp + s) * bytes + b] = *v;
        }
    }
    Ok(())
}

/// Frame ranges of the pixel data element.
fn frames(ds: &Dataset, frames: usize, frame_bytes: usize, coding: Coding) -> Result<Vec<FrameData>> {
    let pixels = ds.get(elements::PIXEL_DATA).ok_or_else(|| Error::Corrupt("DICOM without pixel data".into()))?;
    match (&pixels.value, coding) {
        (Value::Far(at, len), Coding::Native) => {
            if (frames as u64).saturating_mul(frame_bytes as u64) > *len + 1 {
                return Err(Error::Corrupt("DICOM pixel data shorter than its frames".into()));
            }
            Ok((0..frames).map(|k| FrameData::Range(at + (k * frame_bytes) as u64, frame_bytes as u64)).collect())
        }
        (Value::Data(_), Coding::Native) => Err(Error::UnsupportedVariant("DICOM pixel data held inline".into())),
        (Value::Fragments(bot, fragments), _) => {
            if fragments.len() == frames {
                return Ok(fragments.iter().map(|&(a, l)| FrameData::Range(a, l)).collect());
            }
            // Fragment start positions relative to the first fragment item.
            let base = fragments.first().map_or(0, |f| f.0 - 8);
            let item_start = |f: &(u64, u64)| f.0 - 8 - base;
            let starts: Vec<u64> = if bot.len() == frames {
                bot.iter().map(|&o| o as u64).collect()
            } else if let (Some(Value::Data(o)), Some(Value::Data(_))) = (ds.get(EXTENDED_OFFSETS).map(|e| &e.value), ds.get(EXTENDED_LENGTHS).map(|e| &e.value)) {
                o.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes"))).collect()
            } else if frames == 1 {
                vec![0]
            } else {
                return Err(Error::UnsupportedVariant(format!("DICOM with {} fragments for {frames} frames and no offset table", fragments.len())));
            };
            if starts.len() != frames {
                return Err(Error::Corrupt("DICOM offset table does not match the frames".into()));
            }
            let mut out = Vec::with_capacity(frames);
            for (k, &s) in starts.iter().enumerate() {
                let e = starts.get(k + 1).copied().unwrap_or(u64::MAX);
                let parts: Vec<(u64, u64)> = fragments.iter().filter(|f| (s..e).contains(&item_start(f))).copied().collect();
                out.push(FrameData::Parts(parts));
            }
            Ok(out)
        }
        _ => Err(Error::Corrupt("DICOM pixel data in an unexpected form".into())),
    }
}

/// An opened file and its data set.
struct Instance {
    file: File,
    ts: String,
    ds: Dataset,
}

impl Instance {
    fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let p = elements::read(&file)?;
        Ok(Instance { file, ts: p.transfer_syntax, ds: p.dataset })
    }

    /// Value 3 of Image Type (`VOLUME`, `LABEL`, …).
    fn kind(&self) -> String {
        self.ds.texts(IMAGE_TYPE).get(2).cloned().unwrap_or_default()
    }

    fn total(&self) -> (u64, u64) {
        let n = |t| self.ds.number(t).map(|v| v as u64);
        match (n(TOTAL_COLUMNS), n(TOTAL_ROWS)) {
            (Some(w), Some(h)) => (w, h),
            _ => (n(COLUMNS).unwrap_or(0), n(ROWS).unwrap_or(0)),
        }
    }

    /// The image, its planes as (focal plane, optical path) and the number
    /// of each.
    fn image(self, size_z: usize, paths: &[String]) -> Result<Image> {
        let ds = &self.ds;
        let coding = match self.ts.as_str() {
            "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1" => Coding::Native,
            "1.2.840.10008.1.2.5" => Coding::Rle,
            "1.2.840.10008.1.2.4.50" | "1.2.840.10008.1.2.4.51" => Coding::Jpeg,
            "1.2.840.10008.1.2.4.90" | "1.2.840.10008.1.2.4.91" | "1.2.840.10008.1.2.4.201" | "1.2.840.10008.1.2.4.202" | "1.2.840.10008.1.2.4.203" => Coding::Jpeg2000,
            ts => return Err(Error::UnsupportedVariant(format!("DICOM transfer syntax {ts}"))),
        };
        let n = |t| ds.number(t).unwrap_or(0.0);
        let (tile_w, tile_h) = (n(COLUMNS) as u32, n(ROWS) as u32);
        let spp = (n(SAMPLES) as u8).max(1);
        let bits = n(BITS_ALLOCATED) as u32;
        let signed = n(PIXEL_REPRESENTATION) == 1.0;
        let pixel_type = match (bits, signed) {
            (8, false) => PixelType::U8,
            (8, true) => PixelType::I8,
            (16, false) => PixelType::U16,
            (16, true) => PixelType::I16,
            (32, false) if coding == Coding::Native => PixelType::U32,
            (32, true) if coding == Coding::Native => PixelType::I32,
            _ => return Err(Error::UnsupportedVariant(format!("DICOM with {bits} bits allocated"))),
        };
        if !matches!(spp, 1 | 3) || tile_w == 0 || tile_h == 0 {
            return Err(Error::UnsupportedVariant(format!("DICOM frames of {tile_w} x {tile_h} with {spp} samples")));
        }
        let photometric = ds.text(PHOTOMETRIC).unwrap_or_default();
        let ycbcr = photometric.starts_with("YBR_FULL") || (coding == Coding::Jpeg && spp == 3 && photometric != "RGB");
        let count = ds.text(FRAMES).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
        let (width, height) = self.total();
        let frame_bytes = tile_w as usize * tile_h as usize * spp as usize * pixel_type.bytes();
        if frame_bytes > MAX_FRAME {
            return Err(Error::UnsupportedVariant("DICOM frames too large".into()));
        }
        let frames = frames(ds, count, frame_bytes, coding)?;
        let across = width.div_ceil(tile_w as u64) as u32;
        let down = height.div_ceil(tile_h as u64) as u32;
        let planes = size_z * paths.len().max(1);
        let cells = planes as u64 * down as u64 * across as u64;
        if cells > (frames.len() as u64).saturating_mul(64).max(1 << 20) {
            return Err(Error::Corrupt("DICOM tile grid larger than its frames".into()));
        }
        let mut grid = vec![u32::MAX; cells as usize];
        let sparse = ds.text(ORGANIZATION).as_deref() != Some("TILED_FULL") && !ds.items(PER_FRAME_GROUPS).is_empty();
        if sparse {
            // Per-frame positions (1-based), focal plane and optical path.
            let shared_path = ds.item(SHARED_GROUPS).and_then(|s| s.item(OPTICAL_PATH_IDENTIFICATION)).and_then(|o| o.text(OPTICAL_PATH_ID));
            let mut zs: Vec<i64> = Vec::new();
            let groups = ds.items(PER_FRAME_GROUPS);
            for g in groups {
                if let Some(z) = g.item(PLANE_POSITION_SLIDE).and_then(|p| p.number(Z_OFFSET)) {
                    let z = (z * 1e6).round() as i64;
                    if !zs.contains(&z) {
                        zs.push(z);
                    }
                }
            }
            zs.sort_unstable();
            for (k, g) in groups.iter().enumerate().take(frames.len()) {
                let Some(pos) = g.item(PLANE_POSITION_SLIDE) else { continue };
                let (Some(col), Some(row)) = (pos.number(COLUMN_POSITION), pos.number(ROW_POSITION)) else { continue };
                let (tx, ty) = (((col - 1.0) / tile_w as f64).floor(), ((row - 1.0) / tile_h as f64).floor());
                if tx < 0.0 || ty < 0.0 || tx >= across as f64 || ty >= down as f64 {
                    continue;
                }
                let z = pos.number(Z_OFFSET).map(|z| (z * 1e6).round() as i64).and_then(|z| zs.iter().position(|v| *v == z)).unwrap_or(0).min(size_z - 1);
                let path = g.item(OPTICAL_PATH_IDENTIFICATION).and_then(|o| o.text(OPTICAL_PATH_ID)).or_else(|| shared_path.clone());
                let c = path.and_then(|p| paths.iter().position(|q| *q == p)).unwrap_or(0);
                let plane = c + paths.len().max(1) * z;
                grid[((plane as u64 * down as u64 + ty as u64) * across as u64 + tx as u64) as usize] = k as u32;
            }
        } else {
            // TILED_FULL (and plain multi-frame images): columns, rows, focal
            // planes, optical paths.
            let per_plane = across as usize * down as usize;
            for (k, cell) in (0..frames.len()).zip(0..) {
                let (tile, plane) = (cell % per_plane, cell / per_plane);
                let (z, c) = (plane % size_z, plane / size_z);
                if c >= paths.len().max(1) {
                    break;
                }
                let index = c + paths.len().max(1) * z;
                grid[index * per_plane + tile] = k as u32;
            }
        }
        Ok(Image {
            planar: n(PLANAR) == 1.0,
            file: self.file,
            coding,
            width,
            height,
            tile_w,
            tile_h,
            spp,
            ycbcr,
            pixel_type,
            frames,
            grid,
            across,
            down,
        })
    }
}

/// DICOM image set.
#[derive(Debug)]
pub(super) struct Dicom {
    levels: Vec<Image>,
    fill: u8,
    metas: [SeriesMeta; 1],
    associated_kinds: Vec<AssociatedKind>,
    associated: Vec<Image>,
    original: OriginalMeta,
}

impl Dicom {
    /// Opens a DICOM file and, for whole-slide images, the other files of
    /// its series in the same folder.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid files and
    /// [`Error::UnsupportedVariant`] for transfer syntaxes and pixel layouts
    /// not handled.
    pub fn open(path: &Path) -> Result<Self> {
        let first = Instance::open(path)?;
        let wsi = first.ds.text(MODALITY).as_deref() == Some("SM") && first.ds.get(TOTAL_COLUMNS).is_some();
        let mut instances = vec![first];
        if wsi {
            let uid = instances[0].ds.text(SERIES_UID);
            let dir = path.parent().unwrap_or(Path::new("."));
            let mut siblings: Vec<PathBuf> = std::fs::read_dir(dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && p != path).collect();
            siblings.sort();
            for p in siblings.into_iter().take(4096) {
                let mut head = [0u8; 132];
                let Ok(f) = File::open(&p) else { continue };
                if read_exact_at(&f, &mut head, 0).is_err() || !matches(&head) {
                    continue;
                }
                if let Ok(i) = Instance::open(&p)
                    && i.ds.text(SERIES_UID) == uid
                {
                    instances.push(i);
                }
            }
        }
        let (volumes, others): (Vec<Instance>, Vec<Instance>) = instances.into_iter().partition(|i| !wsi || i.kind() == "VOLUME");
        if volumes.is_empty() {
            return Err(Error::UnsupportedVariant("DICOM series without a VOLUME image; open a level file".into()));
        }
        // Levels: largest first, one instance per size.
        let mut by_size: BTreeMap<std::cmp::Reverse<(u64, u64)>, Instance> = BTreeMap::new();
        for i in volumes {
            by_size.entry(std::cmp::Reverse(i.total())).or_insert(i);
        }
        let base = by_size.values().next().expect("one level");
        let ds = &base.ds;
        let size_z = if wsi { ds.number(FOCAL_PLANES).map_or(1, |v| v.max(1.0) as usize) } else { 1 };
        // Optical paths as channels.
        let paths: Vec<String> = if wsi { ds.items(OPTICAL_PATH_SEQUENCE).iter().filter_map(|p| p.text(OPTICAL_PATH_ID)).collect() } else { Vec::new() };
        let size_c = paths.len().max(1);
        let channels: Vec<ChannelMeta> = if paths.is_empty() {
            vec![ChannelMeta::default()]
        } else {
            ds.items(OPTICAL_PATH_SEQUENCE)
                .iter()
                .map(|p| ChannelMeta {
                    name: p.text(OPTICAL_PATH_DESCRIPTION).or_else(|| p.text(OPTICAL_PATH_ID)),
                    excitation_wavelength_nm: p.number(EXCITATION_CENTER).filter(|v| *v > 0.0),
                    emission_wavelength_nm: p.number(EMISSION_CENTER).filter(|v| *v > 0.0),
                    ..Default::default()
                })
                .collect()
        };
        let spacing = ds.item(SHARED_GROUPS).and_then(|s| s.item(PIXEL_MEASURES)).map(|m| m.numbers(PIXEL_SPACING)).filter(|v| v.len() == 2).or_else(|| Some(ds.numbers(PIXEL_SPACING)).filter(|v| v.len() == 2));
        let z_spacing = ds.item(SHARED_GROUPS).and_then(|s| s.item(PIXEL_MEASURES)).and_then(|m| m.number(SPACING_BETWEEN_SLICES)).filter(|v| *v > 0.0 && size_z > 1);
        let date = ds.text(ACQUISITION_DATETIME).or_else(|| Some(format!("{}{}", ds.text(CONTENT_DATE)?, ds.text(CONTENT_TIME).unwrap_or_default()))).and_then(|d| dicom_date(&d));
        let magnification = ds.items(OPTICAL_PATH_SEQUENCE).first().and_then(|p| p.number(OBJECTIVE_POWER)).filter(|v| *v > 0.0);
        let photometric = ds.text(PHOTOMETRIC).unwrap_or_default();
        let bits_stored = ds.number(BITS_STORED).map(|b| b as u8);
        let mut original = OriginalMeta::default();
        for e in &ds.0 {
            if let Value::Data(_) = e.value
                && let Some(t) = ds.text(e.tag).filter(|_| e.vr.iter().all(u8::is_ascii_uppercase) && !matches!(&e.vr, b"OB" | b"OW" | b"UN" | b"US" | b"UL" | b"SS" | b"SL" | b"FL" | b"FD"))
            {
                original.pairs.push((format!("({:04X},{:04X})", e.tag >> 16, e.tag & 0xFFFF), t));
            }
        }
        let name = ds.text(SERIES_DESCRIPTION);
        // Non-WSI multi-frame images: frames are z.
        let frames_as_z = !wsi;
        let mut levels = Vec::new();
        let n_frames = |i: &Instance| i.ds.text(FRAMES).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
        let size_z = if frames_as_z { n_frames(by_size.values().next().expect("one level")) } else { size_z };
        for (_, i) in by_size {
            levels.push(if frames_as_z { i.image_frames_as_z()? } else { i.image(size_z, &paths)? });
        }
        let base = &levels[0];
        let level_metas = levels
            .iter()
            .map(|l| LevelMeta {
                size_x: l.width,
                size_y: l.height,
                tile: (l.across > 1 || l.down > 1).then_some((l.tile_w, l.tile_h)),
                downsample: base.width as f64 / l.width.max(1) as f64,
            })
            .collect();
        let meta = SeriesMeta {
            name,
            acquisition_date: date,
            size_c: if base.spp == 3 { 1 } else { size_c as u32 },
            size_z: size_z as u32,
            size_t: 1,
            pixel_type: base.pixel_type,
            significant_bits: bits_stored.filter(|b| (*b as usize) < base.pixel_type.bytes() * 8),
            samples_per_pixel: base.spp,
            levels: level_metas,
            physical_size: PhysicalSize {
                x_nm: spacing.as_ref().map(|s| s[1] * 1e6).filter(|v| *v > 0.0),
                y_nm: spacing.as_ref().map(|s| s[0] * 1e6).filter(|v| *v > 0.0),
                z_nm: z_spacing.map(|v| v * 1e6),
            },
            channels: if base.spp == 3 { vec![ChannelMeta::default()] } else { channels },
            objective: magnification.map(|m| crate::ObjectiveMeta { magnification: Some(m), ..Default::default() }),
            ..Default::default()
        };
        let mut dicom = Dicom {
            fill: if base.spp == 3 && photometric != "MONOCHROME1" { 255 } else { 0 },
            levels,
            metas: [meta],
            associated_kinds: Vec::new(),
            associated: Vec::new(),
            original,
        };
        for i in others {
            let kind = match i.kind().as_str() {
                "LABEL" => AssociatedKind::Label,
                "OVERVIEW" => AssociatedKind::Macro,
                "THUMBNAIL" => AssociatedKind::Thumbnail,
                _ => continue,
            };
            if !dicom.associated_kinds.contains(&kind)
                && let Ok(img) = i.image(1, &[])
            {
                dicom.associated_kinds.push(kind);
                dicom.associated.push(img);
            }
        }
        Ok(dicom)
    }
}

impl Instance {
    /// A plain image whose frames are z planes of one matrix.
    fn image_frames_as_z(self) -> Result<Image> {
        let n = self.ds.text(FRAMES).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
        let mut img = self.image(1, &[])?;
        // Each frame covers the whole matrix: one tile per plane.
        img.across = 1;
        img.down = 1;
        img.grid = (0..n as u32).collect();
        Ok(img)
    }
}

/// ISO-8601 from a DICOM DT (`YYYYMMDDHHMMSS.FFFFFF&ZZXX`) or date + time.
fn dicom_date(d: &str) -> Option<String> {
    let d = d.trim();
    let digits: String = d.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let mut s = format!("{}-{}-{}", &digits[..4], &digits[4..6], &digits[6..8]);
    if digits.len() >= 14 {
        s += &format!("T{}:{}:{}", &digits[8..10], &digits[10..12], &digits[12..14]);
    }
    Some(s)
}

impl Backend for Dicom {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let i = self.associated_kinds.iter().position(|k| k == kind).ok_or_else(|| Error::OutOfRange(format!("no associated image {kind:?}")))?;
        let img = &self.associated[i];
        let region = Region::new(0, 0, img.width, img.height);
        native_image(region, img.spp, img.pixel_type, |s| img.read(0, region, self.fill, s))
    }

    fn read_native(&self, _series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[0];
        let img = &self.levels[level];
        let index = if img.spp == 3 { plane.z as usize } else { (plane.c + m.size_c * plane.z) as usize };
        img.read(index, region, self.fill, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::elements::*;
    use super::*;

    #[test]
    fn rle_and_dates() {
        // One 8-bit sample: literal run of 3 then a repeat of 2.
        let mut data = vec![0u8; 64];
        data[0] = 1;
        data[4] = 64;
        data.extend_from_slice(&[2, 7, 8, 9, 0xFF, 5]);
        let mut out = [0u8; 5];
        rle(&data, &mut out, 1, 1).unwrap();
        assert_eq!(out, [7, 8, 9, 5, 5]);
        assert_eq!(dicom_date("20091229095915").as_deref(), Some("2009-12-29T09:59:15"));
        assert_eq!(dicom_date("20230614").as_deref(), Some("2023-06-14"));
    }

    fn dump(d: &Dataset, depth: usize, out: &mut Vec<String>) {
        for e in &d.0 {
            if out.len() > 400 {
                return;
            }
            let v = match &e.value {
                Value::Data(b) if b.len() <= 64 => format!("{:?}", String::from_utf8_lossy(b)),
                Value::Data(b) => format!("<{} bytes>", b.len()),
                Value::Far(a, l) => format!("<far {l} @ {a}>"),
                Value::Seq(items) => format!("<{} items>", items.len()),
                Value::Fragments(bot, f) => format!("<bot {} fragments {}>", bot.len(), f.len()),
            };
            out.push(format!("{}{:04X},{:04X} {} {}", "  ".repeat(depth), e.tag >> 16, e.tag & 0xFFFF, String::from_utf8_lossy(&e.vr), v));
            if let Value::Seq(items) = &e.value {
                for it in items.iter().take(2) {
                    dump(it, depth + 1, out);
                }
            }
        }
    }

    #[test]
    #[ignore = "inspection helper: DICOM_DUMP=<file> cargo test dicom_dump -- --ignored --nocapture"]
    fn dicom_dump() {
        let Ok(path) = std::env::var("DICOM_DUMP") else { return };
        let p = read(&std::fs::File::open(path).unwrap()).unwrap();
        println!("TS {}", p.transfer_syntax);
        let mut out = Vec::new();
        dump(&p.dataset, 0, &mut out);
        println!("{}", out.join("\n"));
    }
}

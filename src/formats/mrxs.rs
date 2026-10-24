//! # mrxs
//!
//! 3DHistech MIRAX reader (`.mrxs` plus a folder with `Slidedat.ini`,
//! `Index.dat` and data files).
//!
//! The scanner takes overlapping photos; each photo is split into
//! `CameraImageDivisionsPerSide`² images. Level k stores images that each
//! cover 2^(c + k) of these base images per side (c is the
//! `IMAGE_CONCAT_FACTOR` of level 0), downsampled to the base image size.
//! Each stored image covering whole photos is cut into one piece per photo,
//! and every piece is drawn at its photo's camera position (slide position
//! buffer) scaled to the level. Where a piece is a fractional number of
//! pixels (higher levels), it is cut and placed at the nearest whole pixel
//! (OpenSlide resamples instead: block means agree). Pieces are
//! drawn in image order (row-major image number), later ones on top; photos without data are not
//! drawn and the canvas keeps the slide's fill colour. The `.mrxs` file
//! itself is a preview JPEG. Associated images: barcode (label), slide
//! thumbnail (macro), preview (thumbnail).
//!
//! Not supported: PNG and BMP image data, compressed position buffers
//! (slide version 2.2 and later).
//!
//! Provenance: openslide.org MIRAX format page (prose, CC BY-SA 4.0) in
//! `doc/3dhistech-mrxs/`; own analysis of sample files; piece placement
//! derived by locating decoded images in renderings of OpenSlide run as a
//! black box (`openslide-write-png`). OpenSlide's C code (LGPL) was not
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta,
    PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Mrxs, "3DHistech MIRAX", "1");

/// Largest index, ini or associated image read, against corrupt files.
const MAX_FILE: u64 = 256 << 20;

/// Returns `true` for a `.mrxs` file with its slide folder next to it.
pub(super) fn matches(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mrxs")) && slide_dir(path).join("Slidedat.ini").is_file()
}

/// `a / b` rounded up, for positive `b`.
fn ceil_div(a: i64, b: i64) -> i64 {
    (a + b - 1).div_euclid(b)
}

fn slide_dir(path: &Path) -> PathBuf {
    path.with_extension("")
}

/// `Slidedat.ini`: sections of `key = value`.
#[derive(Debug, Default)]
struct Ini {
    sections: HashMap<String, Vec<(String, String)>>,
    order: Vec<(String, String, String)>,
}

impl Ini {
    fn parse(text: &str) -> Ini {
        let mut ini = Ini::default();
        let mut section = String::new();
        for line in text.trim_start_matches('\u{feff}').lines() {
            let line = line.trim();
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.to_string();
            } else if let Some((k, v)) = line.split_once('=') {
                let (k, v) = (k.trim().to_string(), v.trim().to_string());
                ini.sections.entry(section.clone()).or_default().push((k.clone(), v.clone()));
                ini.order.push((section.clone(), k, v));
            }
        }
        ini
    }

    fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section)?.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn int(&self, section: &str, key: &str) -> Option<i64> {
        self.get(section, key)?.parse().ok()
    }

    fn float(&self, section: &str, key: &str) -> Option<f64> {
        self.get(section, key)?.replace(',', ".").parse().ok()
    }
}

/// Where a stored image or associated blob lives.
#[derive(Debug, Clone, Copy)]
struct Blob {
    file: u32,
    offset: u64,
    len: u32,
}

/// A piece of a stored image drawn at a position of a level.
#[derive(Debug, Clone, Copy)]
struct Piece {
    /// Index into [`Level::images`].
    image: u32,
    x: i64,
    y: i64,
    src_x: u16,
    src_y: u16,
    w: u16,
    h: u16,
}

#[derive(Debug)]
struct Level {
    images: Box<[Blob]>,
    /// Pieces in drawing order.
    pieces: Box<[Piece]>,
    /// Grid of cells of `cell` pixels holding piece indices (CSR layout).
    cell: i64,
    cols: i64,
    cell_start: Box<[u32]>,
    cell_items: Box<[u32]>,
}

impl Level {
    fn new(images: Vec<Blob>, pieces: Vec<Piece>, width: u64, height: u64) -> Level {
        // Cells about the piece size, but never more than a few per piece.
        let area = (width as f64) * (height as f64);
        let min_cell = (area / (4.0 * pieces.len() as f64 + 16.0)).sqrt().ceil() as i64;
        let cell = pieces.iter().map(|p| p.w.max(p.h) as i64).max().unwrap_or(256).max(64).max(min_cell);
        let cols = ceil_div(width as i64, cell).max(1);
        let rows = ceil_div(height as i64, cell).max(1);
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); (cols * rows) as usize];
        for (i, p) in pieces.iter().enumerate() {
            let (cx0, cy0) = ((p.x.max(0) / cell).min(cols - 1), (p.y.max(0) / cell).min(rows - 1));
            let (cx1, cy1) = (((p.x + p.w as i64 - 1).max(0) / cell).min(cols - 1), ((p.y + p.h as i64 - 1).max(0) / cell).min(rows - 1));
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    buckets[(cy * cols + cx) as usize].push(i as u32);
                }
            }
        }
        let mut cell_start = Vec::with_capacity(buckets.len() + 1);
        let mut cell_items = Vec::new();
        for b in &buckets {
            cell_start.push(cell_items.len() as u32);
            cell_items.extend_from_slice(b);
        }
        cell_start.push(cell_items.len() as u32);
        Level { images: images.into(), pieces: pieces.into(), cell, cols, cell_start: cell_start.into(), cell_items: cell_items.into() }
    }

    /// Indices of the pieces intersecting the rectangle, in drawing order.
    fn query(&self, x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<u32> {
        let rows = (self.cell_start.len() as i64 - 1) / self.cols;
        let mut out = Vec::new();
        for cy in (y0 / self.cell).max(0)..=((y1 - 1) / self.cell).min(rows - 1) {
            for cx in (x0 / self.cell).max(0)..=((x1 - 1) / self.cell).min(self.cols - 1) {
                let c = (cy * self.cols + cx) as usize;
                out.extend_from_slice(&self.cell_items[self.cell_start[c] as usize..self.cell_start[c + 1] as usize]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out.retain(|&i| {
            let p = &self.pieces[i as usize];
            p.x < x1 && p.y < y1 && p.x + p.w as i64 > x0 && p.y + p.h as i64 > y0
        });
        out
    }
}

/// 3DHistech MIRAX slide.
#[derive(Debug)]
pub(super) struct Mrxs {
    dir: PathBuf,
    data_files: Vec<String>,
    files: Vec<OnceLock<std::result::Result<File, String>>>,
    keep_open: bool,
    levels: Vec<Level>,
    image_w: u32,
    image_h: u32,
    fill: [u8; 3],
    metas: Vec<SeriesMeta>,
    associated_kinds: Vec<AssociatedKind>,
    associated: Vec<Blob>,
    ini: Ini,
}

/// Index.dat: hierarchical and non-hierarchical record tables.
struct Index {
    data: Vec<u8>,
    hier_root: u32,
    nonhier_root: u32,
}

impl Index {
    fn u32_at(&self, at: usize) -> Result<u32> {
        let b = self.data.get(at..at + 4).ok_or_else(|| Error::Corrupt("MRXS index truncated".into()))?;
        Ok(u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }

    /// Data items of record `record` of a root, each `ints` 32-bit values.
    fn items(&self, root: u32, record: usize, ints: usize) -> Result<Vec<Vec<i32>>> {
        let mut page = self.u32_at(root as usize + 4 * record)? as usize;
        let mut out = Vec::new();
        let mut seen = 0;
        while page != 0 {
            seen += 1;
            if seen > 1 << 20 {
                return Err(Error::Corrupt("MRXS index pages loop".into()));
            }
            let n = self.u32_at(page)? as usize;
            let next = self.u32_at(page + 4)? as usize;
            if n > self.data.len() / (4 * ints) {
                return Err(Error::Corrupt("MRXS index page too large".into()));
            }
            for k in 0..n {
                let at = page + 8 + 4 * ints * k;
                out.push((0..ints).map(|i| self.u32_at(at + 4 * i).map(|v| v as i32)).collect::<Result<Vec<_>>>()?);
            }
            page = next;
        }
        Ok(out)
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_FILE {
        return Err(Error::Corrupt(format!("{} too large", path.display())));
    }
    Ok(std::fs::read(path)?)
}

/// ISO 8601 of `DD/MM/YYYY HH:MM:SS`.
fn creation_date(s: &str) -> Option<String> {
    let (date, time) = s.trim().split_once(' ')?;
    let d: Vec<u32> = date.split('/').filter_map(|v| v.parse().ok()).collect();
    let [day, month, year] = d[..] else { return None };
    ((1..=12).contains(&month) && (1..=31).contains(&day) && time.len() == 8).then(|| format!("{year:04}-{month:02}-{day:02}T{time}"))
}

impl Mrxs {
    /// Opens a MIRAX slide.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an inconsistent index and
    /// [`Error::UnsupportedVariant`] for image formats other than JPEG.
    pub fn open(path: &Path, opts: &OpenOptions) -> Result<Self> {
        let dir = slide_dir(path);
        let text = read_file(&dir.join("Slidedat.ini"))?;
        let ini = Ini::parse(&String::from_utf8_lossy(&text));
        let g = "GENERAL";
        let h = "HIERARCHICAL";
        let need = |s: &str, k: &str| ini.int(s, k).ok_or_else(|| Error::Corrupt(format!("MRXS without {s}.{k}")));
        let (nx, ny) = (need(g, "IMAGENUMBER_X")?, need(g, "IMAGENUMBER_Y")?);
        let div = ini.int(g, "CameraImageDivisionsPerSide").unwrap_or(1).max(1);
        let version = ini.float(g, "CURRENT_SLIDE_VERSION").unwrap_or(1.0);
        let slide_id = ini.get(g, "SLIDE_ID").unwrap_or("");
        if nx <= 0 || ny <= 0 || nx > 1 << 20 || ny > 1 << 20 {
            return Err(Error::Corrupt("MRXS image grid size".into()));
        }
        let index_name = ini.get(h, "INDEXFILE").unwrap_or("Index.dat");
        let data = read_file(&dir.join(index_name))?;
        let root_at = 5 + slide_id.len();
        let mut index = Index { data, hier_root: 0, nonhier_root: 0 };
        index.hier_root = index.u32_at(root_at)?;
        index.nonhier_root = index.u32_at(root_at + 4)?;
        let data_files: Vec<String> =
            (0..ini.int("DATAFILE", "FILE_COUNT").unwrap_or(0)).filter_map(|i| ini.get("DATAFILE", &format!("FILE_{i}")).map(String::from)).collect();

        // Zoom levels: the values of the first hierarchy.
        let zoom_count = need(h, "HIER_0_COUNT")? as usize;
        let level_section = |l: usize| ini.get(h, &format!("HIER_0_VAL_{l}_SECTION")).unwrap_or("").to_string();
        let s0 = level_section(0);
        let image_w = need(&s0, "DIGITIZER_WIDTH")? as u32;
        let image_h = need(&s0, "DIGITIZER_HEIGHT")? as u32;
        let c0 = ini.int(&s0, "IMAGE_CONCAT_FACTOR").unwrap_or(0).clamp(0, 30) as u32;
        let format = ini.get(&s0, "IMAGE_FORMAT").unwrap_or("JPEG");
        if !format.eq_ignore_ascii_case("JPEG") {
            return Err(Error::UnsupportedVariant(format!("MRXS image format {format}")));
        }
        if image_w == 0 || image_h == 0 || image_w > 1 << 15 || image_h > 1 << 15 {
            return Err(Error::Corrupt("MRXS image size".into()));
        }
        let fill = ini.int(&s0, "IMAGE_FILL_COLOR_BGR").unwrap_or(0xFFFFFF) as u32;
        let fill = [(fill & 0xFF) as u8, (fill >> 8 & 0xFF) as u8, (fill >> 16 & 0xFF) as u8];

        // Non-hierarchical records by "layer name.value name".
        let mut nonhier: HashMap<String, usize> = HashMap::new();
        let mut rec = 0;
        for i in 0..ini.int(h, "NONHIER_COUNT").unwrap_or(0) {
            let layer = ini.get(h, &format!("NONHIER_{i}_NAME")).unwrap_or("").to_string();
            for v in 0..ini.int(h, &format!("NONHIER_{i}_COUNT")).unwrap_or(0) {
                let name = ini.get(h, &format!("NONHIER_{i}_VAL_{v}")).unwrap_or("").to_string();
                nonhier.insert(format!("{layer}.{name}"), rec);
                nonhier.insert(name, rec);
                rec += 1;
            }
        }
        let blob = |item: &[i32]| Blob { file: item[4].max(0) as u32, offset: item[2] as u32 as u64, len: item[3] as u32 };

        let mut mrxs = Mrxs {
            dir,
            files: (0..data_files.len()).map(|_| OnceLock::new()).collect(),
            keep_open: data_files.len() <= opts.max_open_files,
            data_files,
            levels: Vec::new(),
            image_w,
            image_h,
            fill,
            metas: Vec::new(),
            associated_kinds: Vec::new(),
            associated: Vec::new(),
            ini: Ini::default(),
        };

        // Camera positions, one per photo, in level-0 pixels.
        let (px, py) = (ceil_div(nx, div), ceil_div(ny, div));
        let positions: Vec<Option<(i64, i64)>> = {
            if version >= 2.2 {
                return Err(Error::UnsupportedVariant("MRXS slide version 2.2 or later (compressed positions)".into()));
            }
            let rec = nonhier.get("VIMSLIDE_POSITION_BUFFER.default").copied();
            match rec.map(|r| index.items(index.nonhier_root, r, 5)).transpose()?.and_then(|v| v.into_iter().next()) {
                Some(item) => {
                    let b = blob(&item);
                    let mut buf = vec![0u8; b.len as usize];
                    mrxs.read_blob(b, &mut buf)?;
                    let n = (px * py) as usize;
                    (0..n)
                        .map(|i| {
                            let e = buf.get(9 * i..9 * i + 9)?;
                            let (x, y) = (i32::from_le_bytes(e[1..5].try_into().ok()?), i32::from_le_bytes(e[5..9].try_into().ok()?));
                            (e[0] != 0 || version < 1.9).then_some((x as i64, y as i64))
                        })
                        .collect()
                }
                // Without positions: the images abut on their grid.
                None => (0..px * py).map(|i| Some(((i % px) * div * image_w as i64 >> c0, (i / px) * div * image_h as i64 >> c0))).collect(),
            }
        };
        let photo = |cx: i64, cy: i64| -> Option<(i64, i64)> {
            (cx < px && cy < py).then(|| positions.get((cy * px + cx) as usize).copied().flatten()).flatten()
        };

        // Pieces of each level; level-0 coordinates relative to the origin.
        let mut level_pieces: Vec<(Vec<Blob>, Vec<(u32, f64, f64, u16, u16, u16, u16)>)> = Vec::new();
        for l in 0..zoom_count {
            let mut records = index.items(index.hier_root, l, 4)?;
            // Drawn in image order (row-major), later images on top.
            records.sort_by_key(|r| r[0]);
            let span = 1i64 << (c0 + l as u32).min(40); // base images per stored image side
            let scale = (1i64 << l.min(40)) as f64; // level downsample
            let mut images = Vec::with_capacity(records.len());
            let mut pieces = Vec::new();
            for r in &records {
                let (image, offset, len, file) = (r[0] as i64, r[1] as u32 as u64, r[2] as u32, r[3].max(0) as u32);
                let (ix, iy) = (image % nx, image / nx);
                if image < 0 || iy >= ny {
                    continue;
                }
                let id = images.len() as u32;
                images.push(Blob { file, offset, len });
                // Photos per image side; one piece per photo, cut at whole
                // pixels (pieces are fractional above the split level).
                let photos = span / div;
                if photos >= 1 {
                    let (pw, ph) = ((div * image_w as i64) as f64 / span as f64, (div * image_h as i64) as f64 / span as f64);
                    for dy in 0..photos {
                        for dx in 0..photos {
                            let Some((x, y)) = photo(ix / div + dx, iy / div + dy) else { continue };
                            let (sx0, sx1) = ((dx as f64 * pw).round(), ((dx + 1) as f64 * pw).round());
                            let (sy0, sy1) = ((dy as f64 * ph).round(), ((dy + 1) as f64 * ph).round());
                            if sx1 <= sx0 || sy1 <= sy0 {
                                continue;
                            }
                            let (fx, fy) = (x as f64 / scale + sx0 - dx as f64 * pw, y as f64 / scale + sy0 - dy as f64 * ph);
                            pieces.push((id, fx, fy, sx0 as u16, sy0 as u16, (sx1 - sx0) as u16, (sy1 - sy0) as u16));
                        }
                    }
                } else if let Some((x, y)) = photo(ix / div, iy / div) {
                    // Part of one photo.
                    let ox = ((ix % div) * image_w as i64) as f64 / span as f64;
                    let oy = ((iy % div) * image_h as i64) as f64 / span as f64;
                    pieces.push((id, x as f64 / scale + ox, y as f64 / scale + oy, 0, 0, image_w as u16, image_h as u16));
                }
            }
            level_pieces.push((images, pieces));
        }
        // Canvas: bounding box of the level-0 pieces.
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in &level_pieces.first().ok_or_else(|| Error::Corrupt("MRXS without levels".into()))?.1 {
            x0 = x0.min(p.1);
            y0 = y0.min(p.2);
            x1 = x1.max(p.1 + p.5 as f64);
            y1 = y1.max(p.2 + p.6 as f64);
        }
        if x0 > x1 {
            return Err(Error::Corrupt("MRXS without image data".into()));
        }
        let (w0, h0) = ((x1 - x0).ceil() as u64, (y1 - y0).ceil() as u64);
        // The camera positions cannot spread the images much beyond their grid.
        let (grid_w, grid_h) = ((nx * image_w as i64) as u64 * 2 + (1 << 16), (ny * image_h as i64) as u64 * 2 + (1 << 16));
        if w0 > grid_w.min(1 << 31) || h0 > grid_h.min(1 << 31) {
            return Err(Error::Corrupt(format!("MRXS canvas {w0} x {h0} from camera positions")));
        }
        let mut level_metas = Vec::new();
        for (l, (images, pieces)) in level_pieces.into_iter().enumerate() {
            let scale = (1u64 << l.min(40)) as f64;
            let (w, h) = ((w0 as f64 / scale).floor() as u64, (h0 as f64 / scale).floor() as u64);
            if w == 0 || h == 0 || pieces.is_empty() {
                break;
            }
            let pieces: Vec<Piece> = pieces
                .into_iter()
                .map(|(image, x, y, sx, sy, pw, ph)| Piece {
                    image,
                    x: (x - x0 / scale).round() as i64,
                    y: (y - y0 / scale).round() as i64,
                    src_x: sx,
                    src_y: sy,
                    w: pw,
                    h: ph,
                })
                .collect();
            level_metas.push(LevelMeta { size_x: w, size_y: h, tile: Some((image_w, image_h)), downsample: scale });
            mrxs.levels.push(Level::new(images, pieces, w, h));
        }

        let mpp = |k: &str| ini.float(&s0, k).filter(|v| *v > 0.0).map(|v| v * 1e3);
        let channels = vec![ChannelMeta::default()];
        mrxs.metas.push(SeriesMeta {
            name: ini.get(g, "SLIDE_NAME").map(String::from),
            acquisition_date: ini.get(g, "SLIDE_CREATIONDATETIME").and_then(creation_date),
            size_c: 1,
            size_z: 1,
            size_t: 1,
            pixel_type: PixelType::U8,
            samples_per_pixel: 3,
            levels: level_metas,
            physical_size: PhysicalSize { x_nm: mpp("MICROMETER_PER_PIXEL_X"), y_nm: mpp("MICROMETER_PER_PIXEL_Y"), z_nm: None },
            channels,
            objective: ini.float(g, "OBJECTIVE_MAGNIFICATION").filter(|m| *m > 0.0).map(|m| ObjectiveMeta {
                magnification: Some(m),
                model: ini.get(g, "OBJECTIVE_NAME").map(String::from),
                ..Default::default()
            }),
            ..Default::default()
        });
        for (name, kind) in [
            ("ScanDataLayer_SlideBarcode", AssociatedKind::Label),
            ("ScanDataLayer_SlideThumbnail", AssociatedKind::Macro),
            ("ScanDataLayer_SlidePreview", AssociatedKind::Thumbnail),
        ] {
            let Some(&r) = nonhier.get(name) else { continue };
            if let Some(item) = index.items(index.nonhier_root, r, 5)?.into_iter().next() {
                if item[3] > 0 {
                    mrxs.associated_kinds.push(kind);
                    mrxs.associated.push(blob(&item));
                }
            }
        }
        mrxs.ini = ini;
        Ok(mrxs)
    }

    /// Reads a blob of a data file into `buf` (`blob.len` bytes).
    fn read_blob(&self, blob: Blob, buf: &mut [u8]) -> Result<()> {
        let name = self.data_files.get(blob.file as usize).ok_or_else(|| Error::Corrupt(format!("MRXS data file {} missing", blob.file)))?;
        let path = self.dir.join(name);
        if !self.keep_open {
            return read_exact_at(&File::open(&path)?, buf, blob.offset);
        }
        let file = self.files[blob.file as usize].get_or_init(|| File::open(&path).map_err(|e| format!("{}: {e}", path.display())));
        let file = file.as_ref().map_err(|e| Error::Io(std::io::Error::other(e.clone())))?;
        read_exact_at(file, buf, blob.offset)
    }

    /// Decodes a stored JPEG image into `out` (`image_w * image_h * 3`).
    fn decode(&self, blob: Blob, out: &mut [u8]) -> Result<(usize, usize)> {
        #[cfg(feature = "jpeg")]
        {
            use crate::codecs::jpeg::{Color, decode};
            scratch::with(blob.len as usize, |src| {
                self.read_blob(blob, src)?;
                let info = decode(src, None, Color::YcbcrToRgb, out)?;
                if info.components != 3 {
                    return Err(Error::UnsupportedVariant(format!("MRXS JPEG with {} components", info.components)));
                }
                Ok((info.width, info.height))
            })
        }
        #[cfg(not(feature = "jpeg"))]
        {
            let _ = (blob, out);
            Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
        }
    }
}

impl Backend for Mrxs {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        for (s, k, v) in &self.ini.order {
            meta.pairs.push((format!("mirax.{s}.{k}"), v.clone()));
        }
        Ok(meta)
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let i = self.associated_kinds.iter().position(|k| k == kind).ok_or_else(|| Error::OutOfRange(format!("no associated image {kind:?}")))?;
        let blob = self.associated[i];
        if blob.len as u64 > MAX_FILE {
            return Err(Error::Corrupt("MRXS associated image too large".into()));
        }
        #[cfg(feature = "jpeg")]
        {
            use crate::codecs::jpeg::{Color, decode};
            let mut src = vec![0u8; blob.len as usize];
            self.read_blob(blob, &mut src)?;
            // Size from the frame header; up to 64k x 64k.
            let mut out = vec![0u8; jpeg_pixels(&src).ok_or_else(|| Error::Corrupt("MRXS associated image is not a JPEG".into()))? * 3];
            let info = decode(&src, None, Color::YcbcrToRgb, &mut out)?;
            out.truncate(info.width * info.height * info.components);
            Ok(NativeImage { width: info.width as u64, height: info.height as u64, samples_per_pixel: info.components as u8, pixel_type: PixelType::U8, data: out })
        }
        #[cfg(not(feature = "jpeg"))]
        Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
    }

    fn read_native(&self, _series: usize, level: usize, _plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let lv = &self.levels[level];
        let (x0, y0) = (region.x as i64, region.y as i64);
        let (x1, y1) = (x0 + region.width as i64, y0 + region.height as i64);
        for r in 0..region.height as usize {
            sink.write_with(r, 0, region.width as usize, |d| {
                for px in d.chunks_exact_mut(3) {
                    px.copy_from_slice(&self.fill);
                }
                Ok(())
            })?;
        }
        let hits = lv.query(x0, y0, x1, y1);
        let (iw, ih) = (self.image_w as usize, self.image_h as usize);
        scratch::with(iw * ih * 3, |tile| {
            let mut decoded = u32::MAX;
            let (mut tw, mut th) = (0, 0);
            for &i in &hits {
                let p = lv.pieces[i as usize];
                if p.image != decoded {
                    (tw, th) = self.decode(lv.images[p.image as usize], tile)?;
                    decoded = p.image;
                }
                // Clip the piece to the region and to the decoded image.
                let (sx0, sy0) = (p.src_x as i64, p.src_y as i64);
                let pw = (p.w as i64).min(tw as i64 - sx0);
                let ph = (p.h as i64).min(th as i64 - sy0);
                let (ax0, ax1) = (x0.max(p.x), x1.min(p.x + pw));
                let (ay0, ay1) = (y0.max(p.y), y1.min(p.y + ph));
                if ax0 >= ax1 || ay0 >= ay1 {
                    continue;
                }
                let w = (ax1 - ax0) as usize;
                for y in ay0..ay1 {
                    let src = (((y - p.y + sy0) as usize) * tw + (ax0 - p.x + sx0) as usize) * 3;
                    sink.write_with((y - y0) as usize, (ax0 - x0) as usize, w, |d| {
                        d.copy_from_slice(&tile[src..src + w * 3]);
                        Ok(())
                    })?;
                }
            }
            Ok(())
        })
    }
}

/// Pixel count from the first SOF marker of a JPEG stream.
#[cfg(feature = "jpeg")]
fn jpeg_pixels(b: &[u8]) -> Option<usize> {
    let mut pos = 2;
    while pos + 9 < b.len() {
        if b[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = b[pos + 1];
        let len = u16::from_be_bytes([b[pos + 2], b[pos + 3]]) as usize;
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u16::from_be_bytes([b[pos + 5], b[pos + 6]]) as usize;
            let w = u16::from_be_bytes([b[pos + 7], b[pos + 8]]) as usize;
            // Associated images are small; larger sizes come from corrupt files.
            return (w > 0 && h > 0 && w * h <= 1 << 26).then_some(w * h);
        }
        pos += 2 + len;
    }
    None
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_and_dates() {
        let ini = Ini::parse("\u{feff}[GENERAL]\r\nA = 1\r\nB = x y\r\n[S]\r\nMPP = 0,5\r\n");
        assert_eq!(ini.int("GENERAL", "A"), Some(1));
        assert_eq!(ini.get("GENERAL", "B"), Some("x y"));
        assert_eq!(ini.float("S", "MPP"), Some(0.5));
        assert_eq!(creation_date("29/12/2009 12:43:52").as_deref(), Some("2009-12-29T12:43:52"));
    }
}

//! # bif
//!
//! Roche Ventana BIF reader (VENTANA DP 200 and iScan BigTIFF slides).
//!
//! Directory 0 is the label/overview image (`Label_Image`), 1 the tissue
//! probability map, 2 the full-resolution scan and 3 onwards the pyramid.
//! In directory 2 the tiles of a row may overlap horizontally: the XMP
//! `EncodeInfo` gives the overlap of each pair of neighbours
//! (`TileJointInfo`, tiles numbered along a serpentine path from the lower
//! left), where `Tile2` covers `Tile1`. The tiles themselves are stored on
//! the TIFF tile grid of the padded image. Level 0 is stitched from that; lower levels abut
//! but were made from the unstitched grid, so each level-0 tile column's
//! footprint is moved to its stitched position (as Bio-Formats does), and
//! levels are cropped to the stitched size. Unscanned tiles (no data) read as
//! the `ScanWhitePoint`.
//!
//! iScan slides (no `Frame` nodes) may have several AOIs and overlap in
//! both directions. Their tiles are placed as OpenSlide (run as a black box)
//! places them: on a lattice per scanned AOI whose pitch is the tile size
//! less the mean overlap of all joined joints, at the AOI's stage position
//! (`Pos-X`, `Pos-Y` with y pointing up), read from the AOI's `AoiOrigin` in
//! the stored grid, rows from the top and later tiles on top. Lower levels
//! draw each tile's footprint in the stored level at the scaled position.
//!
//! Volumetric scans (tag `ImageDepth` 32997, `Z-layers` > 1) store the
//! nominal focus plane first in the tile tables, then the near planes, then
//! the far ones, each a block of the same tiles with the same overlaps.
//! They are returned as z planes from near to far, the nominal plane in the
//! middle; within the near and far blocks the planes are assumed to run
//! outward from the nominal one (the white paper does not say, and no
//! public volumetric sample exists). Levels without further planes return
//! their nominal plane for every z.
//!
//! Provenance: Roche "BIF image file format for digital pathology" white
//! paper (`doc/ventana-bif/`); openslide.org Ventana page (prose, CC BY-SA
//! 4.0); own analysis of sample files. No GPL/LGPL code consulted.
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

use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::{Samples, TiffImage};
use super::tiff::{original_meta, tiff_date};
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta,
    PhysicalSize, Plane, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Bif, "Ventana BIF", "1").extensions(&["bif"]);

/// XMP packet tag.
const TAG_XMP: u16 = 700;
/// Focal planes of a volumetric scan (SGI ImageDepth).
const TAG_IMAGE_DEPTH: u16 = 32997;

/// Storage block of focal plane `z` of `depth` (near to far): the nominal
/// plane first, then the near planes, then the far ones.
fn storage_block(z: usize, depth: usize) -> usize {
    let nominal = (depth - 1) / 2;
    if z < nominal { nominal - z } else if z == nominal { 0 } else { z }
}

/// Returns `true` if the XMP of a directory holds Ventana `iScan` metadata.
pub(super) fn matches(tiff: &TiffFile) -> bool {
    tiff.ifds.first().and_then(|ifd| ifd.entry(TAG_XMP)).and_then(|e| tiff.raw(e).ok()).is_some_and(|x| {
        let x = String::from_utf8_lossy(&x);
        x.contains("<iScan")
    })
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.descendants().find(|c| c.tag_name().name() == name)
}

fn num<T: std::str::FromStr>(n: Node, attr: &str) -> Option<T> {
    n.attribute(attr)?.trim().parse().ok()
}

/// Where a level-0 tile is drawn.
#[derive(Debug, Clone, Copy)]
struct Placed {
    /// Index into the tile tables.
    chunk: u32,
    /// Visible canvas columns and the tile column they start at.
    x0: u32,
    x1: u32,
    src_x: u32,
    y: u32,
}

/// A level-0 tile of an iScan slide on its regular lattice.
#[derive(Debug, Clone, Copy)]
struct LatticeTile {
    /// Origin of the tile in the stored (padded) TIFF grid.
    gx: u32,
    gy: u32,
    /// Canvas position of the tile.
    x: f64,
    y: f64,
}

/// Placement of iScan slides (no `Frame` nodes): the tiles of each scanned
/// AOI on a lattice whose pitch is the tile size less the mean overlap of
/// the joined joints.
#[derive(Debug)]
struct Lattice {
    /// In drawing order: AOIs in turn, rows from the top, columns from the
    /// left; later tiles cover earlier ones.
    tiles: Vec<LatticeTile>,
    tile: (u32, u32),
    width: u64,
    height: u64,
}

/// Ventana BIF slide.
#[derive(Debug)]
pub(super) struct Bif {
    tiff: TiffFile,
    /// Each level's focal planes, near to far.
    levels: Vec<Vec<TiffImage>>,
    /// Level-0 tiles by tile row.
    rows: Vec<Vec<Placed>>,
    lattice: Option<Lattice>,
    tile: (u32, u32),
    white: u8,
    metas: Vec<SeriesMeta>,
    associated_kinds: Vec<AssociatedKind>,
    associated: Vec<TiffImage>,
    original: OriginalMeta,
}

/// Level-0 tiles by tile row, the tile size and the stitched width.
type Stitching = (Vec<Vec<Placed>>, (u32, u32), u32);

/// Level-0 placement from the `EncodeInfo` XMP; `None` if it is missing
/// or describes no overlap (the TIFF tile grid is then used as is).
fn stitching(xmp: &str, chunks: usize, across: u32) -> Result<Option<Stitching>> {
    let Ok(doc) = Document::parse(xmp) else { return Ok(None) };
    let Some(encode) = child(doc.root_element(), "EncodeInfo").or_else(|| (doc.root_element().tag_name().name() == "EncodeInfo").then(|| doc.root_element())) else {
        return Ok(None);
    };
    if num::<u32>(encode, "Ver").unwrap_or(0) < 2 {
        return Err(Error::UnsupportedVariant("BIF EncodeInfo version below 2".into()));
    }
    let infos: Vec<Node> = encode.descendants().filter(|n| n.tag_name().name() == "ImageInfo").collect();
    let [info] = infos[..] else {
        // Several AOIs: placed by their origins on the TIFF grid without overlap.
        return Ok(None);
    };
    let (rows_n, cols_n): (u32, u32) = (num(info, "NumRows").unwrap_or(0), num(info, "NumCols").unwrap_or(0));
    let (tw, th): (u32, u32) = (num(info, "Width").unwrap_or(0), num(info, "Height").unwrap_or(0));
    if rows_n == 0 || cols_n == 0 || tw == 0 || th == 0 || rows_n as u64 * across as u64 > chunks as u64 {
        return Ok(None);
    }
    // Overlap and covering tile of each horizontal joint, by (row, left column).
    let serpentine = |index: u32| -> Option<(u32, u32)> {
        let i = index.checked_sub(1)?;
        let (pr, k) = (i / cols_n, i % cols_n);
        let c = if pr % 2 == 0 { k } else { cols_n - 1 - k };
        (pr < rows_n).then(|| (rows_n - 1 - pr, c))
    };
    let mut overlap = vec![0u32; (rows_n * cols_n) as usize];
    let mut right_on_top = vec![true; (rows_n * cols_n) as usize];
    for j in info.descendants().filter(|n| n.tag_name().name() == "TileJointInfo") {
        let dir = j.attribute("Direction").unwrap_or("");
        if dir != "LEFT" && dir != "RIGHT" {
            if num::<i64>(j, "OverlapY").unwrap_or(0) != 0 {
                return Err(Error::UnsupportedVariant("BIF with vertical tile overlap".into()));
            }
            continue;
        }
        let (Some(a), Some(b)) = (num::<u32>(j, "Tile1").and_then(serpentine), num::<u32>(j, "Tile2").and_then(serpentine)) else { continue };
        if a.0 != b.0 || a.1.abs_diff(b.1) != 1 {
            continue;
        }
        let (row, left) = (a.0, a.1.min(b.1));
        let o = num::<i64>(j, "OverlapX").unwrap_or(0).clamp(0, tw as i64 - 1) as u32;
        overlap[(row * cols_n + left) as usize] = o;
        right_on_top[(row * cols_n + left) as usize] = b.1 > a.1;
    }
    // Storage order of the tiles of the nominal plane (FrameInfo, a
    // sibling of ImageInfo).
    let frames: Vec<(u32, u32)> = encode
        .descendants()
        .filter(|n| n.tag_name().name() == "Frame" && num::<u32>(*n, "Z").unwrap_or(0) == 0)
        .filter_map(|f| {
            let (c, r) = f.attribute("XY")?.split_once(',')?;
            Some((c.trim().parse().ok()?, r.trim().parse().ok()?))
        })
        .collect();
    if frames.len() != (rows_n * cols_n) as usize || frames.iter().any(|&(c, r)| c >= cols_n || r >= rows_n) {
        return Ok(None);
    }
    // The tile tables follow the TIFF tile grid of the (padded) image; the
    // frames only confirm that every tile of the AOI is listed (checked on
    // the sample files against Bio-Formats: frame order gives wrong tiles).
    if across < cols_n {
        return Ok(None);
    }
    let chunk_of: Vec<u32> = (0..rows_n).flat_map(|r| (0..cols_n).map(move |c| r * across + c)).collect();
    let mut rows = Vec::with_capacity(rows_n as usize);
    let mut width = u32::MAX;
    for r in 0..rows_n {
        let mut row = Vec::with_capacity(cols_n as usize);
        let mut x = 0u32;
        for c in 0..cols_n {
            // Columns covered by a neighbour on top are not visible.
            let left_hidden = if c > 0 && right_on_top[(r * cols_n + c - 1) as usize] { 0 } else if c > 0 { overlap[(r * cols_n + c - 1) as usize] } else { 0 };
            let right_hidden = if c + 1 < cols_n && right_on_top[(r * cols_n + c) as usize] { overlap[(r * cols_n + c) as usize] } else { 0 };
            let start = x;
            let (x0, x1) = (start + left_hidden, start + tw - right_hidden);
            row.push(Placed { chunk: chunk_of[(r * cols_n + c) as usize], x0, x1, src_x: left_hidden, y: r * th });
            x = start + tw - if c + 1 < cols_n { overlap[(r * cols_n + c) as usize] } else { 0 };
        }
        width = width.min(x);
        rows.push(row);
    }
    Ok(Some((rows, (tw, th), width)))
}

/// The [`Lattice`] of an `EncodeInfo` without `Frame` nodes (iScan
/// scanners); `None` if the layout is incomplete.
///
/// Every AOI has `Pos-X`/`Pos-Y` (stage coordinates in level-0 pixels, y
/// pointing up) and its tile grid starts at `AoiOrigin` in the stored image.
/// Tiles are placed at a constant pitch, not by the overlap of each joint.
fn lattice(xmp: &str, grid: (u32, u32)) -> Option<Lattice> {
    let doc = Document::parse(xmp).ok()?;
    let encode = doc.descendants().find(|n| n.tag_name().name() == "EncodeInfo")?;
    if encode.descendants().any(|n| n.tag_name().name() == "Frame") {
        return None;
    }
    let infos: Vec<Node> = encode.descendants().filter(|n| n.tag_name().name() == "ImageInfo").collect();
    let origins = child(encode, "AoiOrigin")?;
    let (tw, th): (u32, u32) = (num(*infos.first()?, "Width")?, num(*infos.first()?, "Height")?);
    if tw == 0 || th == 0 {
        return None;
    }
    // Mean overlap of the joined joints of all AOIs.
    let (mut sx, mut nx, mut sy, mut ny) = (0f64, 0u64, 0f64, 0u64);
    for j in encode.descendants().filter(|n| n.tag_name().name() == "TileJointInfo" && n.attribute("FlagJoined") == Some("1")) {
        match j.attribute("Direction") {
            Some("LEFT" | "RIGHT") => (sx, nx) = (sx + num::<f64>(j, "OverlapX").unwrap_or(0.0), nx + 1),
            Some("UP" | "DOWN") => (sy, ny) = (sy + num::<f64>(j, "OverlapY").unwrap_or(0.0), ny + 1),
            _ => {}
        }
    }
    let mean = |s: f64, n: u64, size: u32| if n == 0 { 0.0 } else { (s / n as f64).clamp(0.0, size as f64 - 1.0) };
    let (px, py) = (tw as f64 - mean(sx, nx, tw), th as f64 - mean(sy, ny, th));
    struct Aoi {
        rows: u32,
        cols: u32,
        pos: (f64, f64),
        origin: (u32, u32),
    }
    let mut aois = Vec::new();
    for (i, info) in infos.iter().enumerate() {
        if num::<u32>(*info, "AOIScanned") == Some(0) {
            continue;
        }
        let index = num::<usize>(*info, "AOIIndex").unwrap_or(i);
        let o = origins.children().find(|n| n.tag_name().name() == format!("AOI{index}"))?;
        let aoi = Aoi {
            rows: num(*info, "NumRows")?,
            cols: num(*info, "NumCols")?,
            pos: (num(*info, "Pos-X")?, num(*info, "Pos-Y")?),
            origin: (num(o, "OriginX")?, num(o, "OriginY")?),
        };
        if (num(*info, "Width"), num(*info, "Height")) != (Some(tw), Some(th))
            || !(aoi.pos.0.is_finite() && aoi.pos.1.is_finite())
            || aoi.rows == 0
            || aoi.cols == 0
            || aoi.origin.0 as u64 + aoi.cols as u64 * tw as u64 > grid.0 as u64
            || aoi.origin.1 as u64 + aoi.rows as u64 * th as u64 > grid.1 as u64
        {
            return None;
        }
        aois.push(aoi);
    }
    // Stage y points up: the canvas top is the highest AOI top.
    let top = |a: &Aoi| a.pos.1 + (a.rows - 1) as f64 * py + th as f64;
    let x0 = aois.iter().map(|a| a.pos.0).reduce(f64::min)?;
    let y_top = aois.iter().map(top).reduce(f64::max)?;
    let y_bottom = aois.iter().map(|a| a.pos.1).reduce(f64::min)?;
    let x1 = aois.iter().map(|a| a.pos.0 + (a.cols - 1) as f64 * px + tw as f64).reduce(f64::max)?;
    let mut tiles = Vec::new();
    for a in &aois {
        let (ax, ay) = (a.pos.0 - x0, y_top - top(a));
        for r in 0..a.rows {
            for c in 0..a.cols {
                tiles.push(LatticeTile { gx: a.origin.0 + c * tw, gy: a.origin.1 + r * th, x: ax + c as f64 * px, y: ay + r as f64 * py });
            }
        }
    }
    let (width, height) = ((x1 - x0).ceil() as u64, (y_top - y_bottom).ceil() as u64);
    // Stitching shrinks the grid; a much larger canvas means corrupt positions.
    if width > 4 * grid.0 as u64 || height > 4 * grid.1 as u64 {
        return None;
    }
    Some(Lattice { tiles, tile: (tw, th), width, height })
}

impl Bif {
    /// Opens a BIF file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] if the scan image is missing and
    /// [`Error::UnsupportedVariant`] for stitching data this reader cannot
    /// apply.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let tiff = TiffFile::open(path)?;
        let mut levels = Vec::new();
        let mut associated = Vec::new();
        let mut scan_ifd = None;
        for (i, ifd) in tiff.ifds.iter().enumerate() {
            let desc = tiff.string(ifd, tag::IMAGE_DESCRIPTION)?.unwrap_or_default();
            let Ok(image) = TiffImage::new(&tiff, ifd) else { continue };
            match desc.trim() {
                "Label_Image" | "Label Image" => associated.push((AssociatedKind::Macro, image)),
                "Thumbnail" => associated.push((AssociatedKind::Thumbnail, image)),
                "Probability_Image" => associated.push((AssociatedKind::Other("probability".into()), image)),
                d if d.starts_with("level=") => {
                    scan_ifd.get_or_insert(i);
                    levels.push((i, image));
                }
                _ => {}
            }
        }
        let scan_ifd = scan_ifd.ok_or_else(|| Error::Corrupt("BIF without scan image".into()))?;
        levels.sort_by_key(|(_, l)| std::cmp::Reverse(l.width));
        let depth_of = |i: usize| -> Result<usize> {
            let d = tiff.floats(&tiff.ifds[i], TAG_IMAGE_DEPTH)?.and_then(|v| v.first().copied()).unwrap_or(1.0);
            Ok(if (1.0..=1024.0).contains(&d) { d as usize } else { 1 })
        };
        let depth = depth_of(levels[0].0)?;
        let levels: Vec<Vec<TiffImage>> = levels
            .into_iter()
            .map(|(i, img)| {
                // A level without the further planes repeats its nominal one.
                let planes = if depth_of(i)? == depth { (0..depth).map(|z| img.layer(storage_block(z, depth))).collect::<Result<_>>()? } else { vec![img; depth] };
                Ok(planes)
            })
            .collect::<Result<_>>()?;
        let base = &levels[0][0];
        let xmp = |i: usize| {
            tiff.ifds[i].entry(TAG_XMP).and_then(|e| tiff.raw(e).ok()).map(|x| String::from_utf8_lossy(&x).trim_end_matches('\0').to_string())
        };
        let scan_xmp = xmp(scan_ifd).unwrap_or_default();
        let across = base.width.div_ceil(base.chunk_w.max(1));
        let (rows, tile, width) = match stitching(&scan_xmp, base.chunk_count(), across)? {
            Some((rows, tile, width)) if tile == (base.chunk_w, base.chunk_h) => (rows, tile, width),
            _ => (Vec::new(), (base.chunk_w, base.chunk_h), base.width),
        };
        let height = if rows.is_empty() { base.height } else { rows.len() as u32 * tile.1 };
        let lattice = if rows.is_empty() { lattice(&scan_xmp, (base.width, base.height)) } else { None };
        // iScan attributes of the label directory.
        let iscan = xmp(0).unwrap_or_default();
        let (mut mpp, mut mag, mut white, mut z_spacing) = (None, None, 255u8, None);
        let mut pairs = original_meta(&tiff, &tiff.ifds[scan_ifd])?;
        if let Ok(doc) = Document::parse(&iscan)
            && let Some(n) = child(doc.root_element(), "iScan").or_else(|| (doc.root_element().tag_name().name() == "iScan").then(|| doc.root_element())) {
                mpp = num::<f64>(n, "ScanRes").filter(|v| *v > 0.0);
                mag = num::<f64>(n, "Magnification").filter(|v| *v > 0.0);
                white = num::<u8>(n, "ScanWhitePoint").unwrap_or(255);
                z_spacing = num::<f64>(n, "Z-spacing").filter(|v| *v > 0.0 && depth > 1);
                pairs.extend(n.attributes().map(|a| (format!("ventana.{}", a.name()), a.value().to_string())));
            }
        let level_metas: Vec<LevelMeta> = levels
            .iter()
            .enumerate()
            .map(|(l, planes)| {
                let img = &planes[0];
                let f = (1u64 << l) as f64;
                let (w, h) = match &lattice {
                    // Lower levels round up, as OpenSlide reports them.
                    // The lattice canvas need not fit in the stored grid.
                    Some(lat) => (lat.width.div_ceil(1 << l), lat.height.div_ceil(1 << l)),
                    None if l == 0 => ((width as u64).min(img.width as u64), (height as u64).min(img.height as u64)),
                    None => (((width as f64 / f) as u64).min(img.width as u64), ((height as f64 / f) as u64).min(img.height as u64)),
                };
                LevelMeta { size_x: w, size_y: h, tile: Some((img.chunk_w, img.chunk_h)), downsample: f }
            })
            .collect();
        let meta = SeriesMeta {
            acquisition_date: tiff.string(&tiff.ifds[scan_ifd], tag::DATE_TIME)?.and_then(|d| tiff_date(&d)),
            size_c: 1,
            size_z: depth as u32,
            size_t: 1,
            pixel_type: base.pixel_type,
            samples_per_pixel: base.samples_out,
            levels: level_metas,
            physical_size: PhysicalSize { x_nm: mpp.map(|v| v * 1e3), y_nm: mpp.map(|v| v * 1e3), z_nm: z_spacing.map(|v| v * 1e3) },
            channels: vec![ChannelMeta::default()],
            objective: mag.map(|m| ObjectiveMeta { magnification: Some(m), ..Default::default() }),
            ..Default::default()
        };
        let mut bif = Bif {
            tiff,
            levels,
            rows,
            lattice,
            tile,
            white,
            metas: vec![meta],
            associated_kinds: Vec::new(),
            associated: Vec::new(),
            original: OriginalMeta { pairs, documents: vec![("XMP".into(), scan_xmp)] },
        };
        for (kind, image) in associated {
            if !bif.associated_kinds.contains(&kind) {
                bif.associated_kinds.push(kind);
                bif.associated.push(image);
            }
        }
        Ok(bif)
    }

    /// Stitched level 0 of focal plane `z`.
    fn read_stitched(&self, z: usize, region: Region, sink: &mut Sink) -> Result<()> {
        let base = &self.levels[0][z];
        let (tw, th) = self.tile;
        let px = base.samples_out as usize * base.pixel_type.bytes();
        let (x0, y0) = (region.x as u32, region.y as u32);
        let (x1, y1) = (x0 + region.width as u32, y0 + region.height as u32);
        scratch::with(tw as usize * th as usize * px, |tile| {
            for row in &self.rows[(y0 / th) as usize..(y1.div_ceil(th) as usize).min(self.rows.len())] {
                for p in row.iter().filter(|p| p.x0 < x1 && p.x1 > x0 && p.x1 > p.x0) {
                    let present = p.chunk != u32::MAX && base.decode_chunk(&self.tiff, p.chunk as usize, tile)?;
                    let (ax0, ax1) = (x0.max(p.x0), x1.min(p.x1));
                    let w = (ax1 - ax0) as usize;
                    for y in y0.max(p.y)..y1.min(p.y + th) {
                        let src = (((y - p.y) * tw + p.src_x + ax0 - p.x0) as usize) * px;
                        sink.write_with((y - y0) as usize, (ax0 - x0) as usize, w, |d| {
                            if present {
                                d.copy_from_slice(&tile[src..src + w * px]);
                            } else {
                                d.fill(self.white);
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

impl Bif {
    /// A lower level: the stored pyramid was made from the unstitched tile
    /// grid, so the footprint of each level-0 tile column is moved to its
    /// stitched position, scaled to the level.
    fn read_shifted(&self, level: usize, z: usize, region: Region, sink: &mut Sink) -> Result<()> {
        let img = &self.levels[level][z];
        let f = 1u64 << level.min(30);
        let (tw, th) = (self.tile.0 as u64, self.tile.1 as u64);
        let px = img.samples_out as usize * img.pixel_type.bytes();
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        let (x0, y0) = (region.x, region.y);
        let (x1, y1) = (x0 + region.width, y0 + region.height);
        // Canvas the stored level does not reach reads as unscanned.
        for y in 0..region.height as usize {
            sink.write_with(y, 0, region.width as usize, |d| {
                d.fill(self.white);
                Ok(())
            })?;
        }
        for (r, row) in self.rows.iter().enumerate() {
            // Level rows of this band of level-0 tiles.
            let (ry0, ry1) = ((r as u64 * th).div_ceil(f).max(y0), (((r as u64 + 1) * th).div_ceil(f)).min(y1));
            if ry0 >= ry1 {
                continue;
            }
            for (c, p) in row.iter().enumerate() {
                let (cx0, cx1) = ((p.x0 as u64).div_ceil(f).max(x0), (p.x1 as u64).div_ceil(f).min(x1));
                if cx0 >= cx1 {
                    continue;
                }
                // Stored column of canvas column cx0.
                let stored = (c as u64 * tw + p.src_x as u64) / f + (cx0 - (p.x0 as u64).div_ceil(f));
                let w = (cx1 - cx0).min((img.width as u64).saturating_sub(stored));
                if w == 0 {
                    continue;
                }
                let src = Region::new(stored, ry0, w, ry1.min(img.height as u64) - ry0);
                let part = native_image(src, img.samples_out, img.pixel_type, |s| img.read_native(&self.tiff, src, samples, s))?;
                for y in 0..src.height as usize {
                    let line = &part.data[y * w as usize * px..(y + 1) * w as usize * px];
                    sink.write_with((ry0 - y0) as usize + y, (cx0 - x0) as usize, w as usize, |d| {
                        d.copy_from_slice(line);
                        Ok(())
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl Bif {
    /// A level of an iScan slide: each tile's footprint in the stored level
    /// drawn at its lattice position scaled to the level, snapped to whole
    /// pixels, later tiles on top. Canvas no tile covers reads as unscanned.
    fn read_lattice(&self, lat: &Lattice, level: usize, z: usize, region: Region, sink: &mut Sink) -> Result<()> {
        let img = &self.levels[level][z];
        let f = (1u64 << level.min(30)) as f64;
        let px = img.samples_out as usize * img.pixel_type.bytes();
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        let (x0, y0) = (region.x as i64, region.y as i64);
        let (x1, y1) = (x0 + region.width as i64, y0 + region.height as i64);
        for y in 0..region.height as usize {
            sink.write_with(y, 0, region.width as usize, |d| {
                d.fill(self.white);
                Ok(())
            })?;
        }
        let (tw, th) = (lat.tile.0 as f64, lat.tile.1 as f64);
        for t in &lat.tiles {
            // Footprint on the canvas and in the stored level.
            let (dx0, dy0) = ((t.x / f).round() as i64, (t.y / f).round() as i64);
            let (sx0, sy0) = ((t.gx as f64 / f).round() as i64, (t.gy as f64 / f).round() as i64);
            let w = ((t.gx as f64 + tw) / f).round() as i64 - sx0;
            let h = ((t.gy as f64 + th) / f).round() as i64 - sy0;
            let w = w.min(img.width as i64 - sx0);
            let h = h.min(img.height as i64 - sy0);
            let (cx0, cx1) = (x0.max(dx0), x1.min(dx0 + w));
            let (cy0, cy1) = (y0.max(dy0), y1.min(dy0 + h));
            if cx0 >= cx1 || cy0 >= cy1 {
                continue;
            }
            let src = Region::new((sx0 + cx0 - dx0) as u64, (sy0 + cy0 - dy0) as u64, (cx1 - cx0) as u64, (cy1 - cy0) as u64);
            let part = native_image(src, img.samples_out, img.pixel_type, |s| img.read_native(&self.tiff, src, samples, s))?;
            let row = src.width as usize * px;
            for y in 0..src.height as usize {
                sink.write_with((cy0 - y0) as usize + y, (cx0 - x0) as usize, src.width as usize, |d| {
                    d.copy_from_slice(&part.data[y * row..(y + 1) * row]);
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
}

impl Backend for Bif {
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
        let region = Region::new(0, 0, img.width as u64, img.height as u64);
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        native_image(region, img.samples_out, img.pixel_type, |sink| img.read_native(&self.tiff, region, samples, sink))
    }

    fn read_native(&self, _series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let z = plane.z as usize;
        if let Some(lat) = &self.lattice {
            return self.read_lattice(lat, level, z, region, sink);
        }
        if level == 0 && !self.rows.is_empty() {
            return self.read_stitched(z, region, sink);
        }
        if !self.rows.is_empty() {
            return self.read_shifted(level, z, region, sink);
        }
        let img = &self.levels[level][z];
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        img.read_native(&self.tiff, region, samples, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focal_planes_near_to_far() {
        // Storage: nominal, near -1, near -2, far +1, far +2.
        assert_eq!((0..5).map(|z| storage_block(z, 5)).collect::<Vec<_>>(), [2, 1, 0, 3, 4]);
        assert_eq!(storage_block(0, 1), 0);
    }

    #[test]
    fn iscan_lattice() {
        // Two AOIs as in OpenSlide's OS-2 sample, scaled down: stage y points up.
        let xmp = r#"<EncodeInfo Ver='2'>
            <ImageInfo AOIScanned="1" Width="10" Height="8" NumRows="2" NumCols="2" Pos-X="0" Pos-Y="20">
              <TileJointInfo FlagJoined='1' Direction='RIGHT' Tile1='1' Tile2='2' OverlapX='3' OverlapY='1'/>
              <TileJointInfo FlagJoined='0' Direction='RIGHT' Tile1='3' Tile2='4' OverlapX='0' OverlapY='0'/>
              <TileJointInfo FlagJoined='1' Direction='UP' Tile1='1' Tile2='4' OverlapX='-1' OverlapY='2'/></ImageInfo>
            <ImageInfo AOIScanned="0" Width="10" Height="8" NumRows="1" NumCols="1" Pos-X="5" Pos-Y="5"/>
            <ImageInfo AOIScanned="1" Width="10" Height="8" NumRows="1" NumCols="2" Pos-X="30" Pos-Y="0">
              <TileJointInfo FlagJoined='1' Direction='RIGHT' Tile1='1' Tile2='2' OverlapX='5' OverlapY='0'/></ImageInfo>
            <AoiOrigin><AOI0 OriginX="0" OriginY="0"/><AOI1 OriginX="20" OriginY="16"/><AOI2 OriginX="0" OriginY="16"/></AoiOrigin>
            </EncodeInfo>"#;
        let lat = lattice(xmp, (30, 24)).unwrap();
        // Pitch: 10 - (3 + 5) / 2 = 6 across, 8 - 2 = 6 down.
        let pos: Vec<_> = lat.tiles.iter().map(|t| (t.gx, t.gy, t.x, t.y)).collect();
        // AOI0 top: 20 + 6 + 8 = 34 is the canvas top; AOI2 top at 34 - 8.
        assert_eq!(pos, [(0, 0, 0.0, 0.0), (10, 0, 6.0, 0.0), (0, 8, 0.0, 6.0), (10, 8, 6.0, 6.0), (0, 16, 30.0, 26.0), (10, 16, 36.0, 26.0)]);
        assert_eq!((lat.width, lat.height), (46, 34));
        // DP 200 layouts (Frame nodes) are not lattices.
        assert!(lattice(&xmp.replace("</EncodeInfo>", "<FrameInfo><Frame XY='0,0' Z='0'/></FrameInfo></EncodeInfo>"), (30, 24)).is_none());
    }

    #[test]
    fn stitches_rows_with_overlap() {
        // Two rows of three tiles; physical numbering from the lower left:
        // bottom row 1 2 3, top row 6 5 4.
        let xmp = r#"<EncodeInfo Ver="2"><SlideStitchInfo><ImageInfo NumRows="2" NumCols="3" Width="10" Height="4">
            <TileJointInfo Direction="LEFT" Tile1="1" Tile2="2" OverlapX="3" OverlapY="0"/>
            <TileJointInfo Direction="LEFT" Tile1="2" Tile2="3" OverlapX="0" OverlapY="0"/>
            <TileJointInfo Direction="LEFT" Tile1="5" Tile2="4" OverlapX="2" OverlapY="0"/>
            <TileJointInfo Direction="LEFT" Tile1="6" Tile2="5" OverlapX="2" OverlapY="0"/>
            </ImageInfo><FrameInfo>
            <Frame XY="0,0" Z="0"/><Frame XY="1,0" Z="0"/><Frame XY="2,0" Z="0"/>
            <Frame XY="0,1" Z="0"/><Frame XY="1,1" Z="0"/><Frame XY="2,1" Z="0"/></FrameInfo></SlideStitchInfo></EncodeInfo>"#;
        let (rows, tile, width) = stitching(xmp, 8, 4).unwrap().unwrap();
        assert_eq!(tile, (10, 4));
        // Top row (image row 0) holds tiles 6 5 4: 5 covers 6, 4 covers 5.
        let top: Vec<(u32, u32, u32)> = rows[0].iter().map(|p| (p.x0, p.x1, p.src_x)).collect();
        assert_eq!(top, [(0, 8, 0), (8, 16, 0), (16, 26, 0)]);
        // Bottom row 1 2 3: 2 covers 1 by 3 columns.
        let bottom: Vec<(u32, u32, u32)> = rows[1].iter().map(|p| (p.x0, p.x1, p.src_x)).collect();
        assert_eq!(bottom, [(0, 7, 0), (7, 17, 0), (17, 27, 0)]);
        assert_eq!(width, 26);
        // TIFF grid of 4 tiles across: row 1, column 0.
        assert_eq!(rows[1][0].chunk, 4);
    }
}

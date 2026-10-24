//! # bif
//!
//! Roche Ventana BIF reader (VENTANA DP 200 BigTIFF slides).
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
//! the `ScanWhitePoint`. Only the nominal focus plane is read.
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

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Bif, "Ventana BIF", "1");

/// XMP packet tag.
const TAG_XMP: u16 = 700;

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

/// Ventana BIF slide.
#[derive(Debug)]
pub(super) struct Bif {
    tiff: TiffFile,
    levels: Vec<TiffImage>,
    /// Level-0 tiles by tile row.
    rows: Vec<Vec<Placed>>,
    tile: (u32, u32),
    white: u8,
    metas: Vec<SeriesMeta>,
    associated_kinds: Vec<AssociatedKind>,
    associated: Vec<TiffImage>,
    original: OriginalMeta,
}

/// Level-0 placement from the `EncodeInfo` XMP; `None` if it is missing
/// or describes no overlap (the TIFF tile grid is then used as is).
fn stitching(xmp: &str, chunks: usize, across: u32) -> Result<Option<(Vec<Vec<Placed>>, (u32, u32), u32)>> {
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
                    levels.push(image);
                }
                _ => {}
            }
        }
        let scan_ifd = scan_ifd.ok_or_else(|| Error::Corrupt("BIF without scan image".into()))?;
        levels.sort_by_key(|l| std::cmp::Reverse(l.width));
        let base = &levels[0];
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
        // iScan attributes of the label directory.
        let iscan = xmp(0).unwrap_or_default();
        let (mut mpp, mut mag, mut white) = (None, None, 255u8);
        let mut pairs = original_meta(&tiff, &tiff.ifds[scan_ifd])?;
        if let Ok(doc) = Document::parse(&iscan) {
            if let Some(n) = child(doc.root_element(), "iScan").or_else(|| (doc.root_element().tag_name().name() == "iScan").then(|| doc.root_element())) {
                mpp = num::<f64>(n, "ScanRes").filter(|v| *v > 0.0);
                mag = num::<f64>(n, "Magnification").filter(|v| *v > 0.0);
                white = num::<u8>(n, "ScanWhitePoint").unwrap_or(255);
                pairs.extend(n.attributes().map(|a| (format!("ventana.{}", a.name()), a.value().to_string())));
            }
        }
        let level_metas: Vec<LevelMeta> = levels
            .iter()
            .enumerate()
            .map(|(l, img)| {
                let f = (1u64 << l) as f64;
                let (w, h) = if l == 0 { (width as u64, height as u64) } else { ((width as f64 / f) as u64, (height as f64 / f) as u64) };
                LevelMeta { size_x: w.min(img.width as u64), size_y: h.min(img.height as u64), tile: Some((img.chunk_w, img.chunk_h)), downsample: f }
            })
            .collect();
        let meta = SeriesMeta {
            acquisition_date: tiff.string(&tiff.ifds[scan_ifd], tag::DATE_TIME)?.and_then(|d| tiff_date(&d)),
            size_c: 1,
            size_z: 1,
            size_t: 1,
            pixel_type: base.pixel_type,
            samples_per_pixel: base.samples_out,
            levels: level_metas,
            physical_size: PhysicalSize { x_nm: mpp.map(|v| v * 1e3), y_nm: mpp.map(|v| v * 1e3), z_nm: None },
            channels: vec![ChannelMeta::default()],
            objective: mag.map(|m| ObjectiveMeta { magnification: Some(m), ..Default::default() }),
            ..Default::default()
        };
        let mut bif = Bif {
            tiff,
            levels,
            rows,
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

    /// Stitched level 0.
    fn read_stitched(&self, region: Region, sink: &mut Sink) -> Result<()> {
        let base = &self.levels[0];
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
    fn read_shifted(&self, level: usize, region: Region, sink: &mut Sink) -> Result<()> {
        let img = &self.levels[level];
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

    fn read_native(&self, _series: usize, level: usize, _plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        if level == 0 && !self.rows.is_empty() {
            return self.read_stitched(region, sink);
        }
        if !self.rows.is_empty() {
            return self.read_shifted(level, region, sink);
        }
        let img = &self.levels[level];
        let samples = if img.samples_out == 1 { Samples::One(0) } else { Samples::All };
        img.read_native(&self.tiff, region, samples, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

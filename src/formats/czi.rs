//! # czi
//!
//! Zeiss CZI reader.
//!
//! A CZI file is a chain of segments; pixel data lives in subblocks, each a
//! tile of one plane at given dimension coordinates. One series per scene
//! (`S`), split further where the dimensions B, H, I, R, V vary. Mosaic
//! tiles (`M`) are composited in directory order (later entries on top,
//! as Bio-Formats does) onto a canvas spanning the scene's
//! full-resolution tiles; uncovered canvas is 0. Downsampled subblocks form
//! pyramid levels, one per downsampling factor. Colour pixel types are
//! stored blue first and returned as RGB(A). Attachments `Thumbnail`
//! (JPEG), `Label` and `SlidePreview` (embedded CZI) are associated images.
//!
//! Multi-file documents: the master holds the XML and the directories;
//! entries with `file_part` k > 0 read from part k, found next to the
//! master as `<stem> (k).czi` or `<stem>(k).czi` with that part number and
//! the master's primary GUID. An entry naming a part that is not there
//! reads from the master if the segment at its position repeats the entry
//! (some writers store the scene index in `file_part`).
//!
//! Not supported: lossless JPEG, camera raw data,
//! complex pixel types, super-resolved renderings (subblocks with more
//! stored than logical pixels are ignored).
//!
//! Provenance: OpenReadout CZI format notes (MIT OR Apache-2.0); ZEISS
//! libCZI documentation pages (MIT, docs only, no libCZI code) in
//! `doc/zeiss-czi/`; own analysis of sample files. No GPL/LGPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod decode;
mod dir;
mod meta;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;

use decode::{Geometry, Mask};
use dir::{Attachment, Entry, Span};

use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, Error, FormatId, FormatInfo, LevelMeta, OpenOptions, OriginalMeta, PhysicalSize, PixelType,
    Plane, PlaneMeta, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Czi, "Zeiss CZI", "1");

/// Largest associated image decoded, in pixels per side.
const ASSOCIATED_MAX: u64 = 4096;
/// Largest subblock decoded, against corrupt sizes.
const MAX_SUBBLOCK: usize = 1 << 31;
/// Most file parts looked for, against corrupt part numbers.
const MAX_PARTS: usize = 1024;

/// Returns `true` if `head` starts with the CZI file segment.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(b"ZISRAWFILE")
}

/// A subblock in the compact index.
#[derive(Debug, Clone, Copy)]
struct Block {
    position: u64,
    part: u32,
    /// Start in level-0 pixels.
    x: i32,
    y: i32,
    stored_w: u32,
    stored_h: u32,
    compression: u32,
    /// Sort key: series, level, c, z, t (indices relative to the series).
    key: (u32, u8, u32, u32, u32),
}

/// x, y, width, height in level pixels.
type Rect = (i64, i64, i64, i64);

/// Geometry of one series.
#[derive(Debug)]
struct Scene {
    /// Level-0 origin of the canvas.
    x0: i64,
    y0: i64,
    /// Downsampling factor of each level, level 0 first.
    factors: Vec<f64>,
    geometry: (PixelType, u8),
    /// Range of `blocks` holding this series.
    blocks: std::ops::Range<usize>,
}

/// One file of a document: the master (part 0) or a following part.
#[derive(Debug)]
struct Part {
    file_part: u32,
    file: File,
    span: Span,
}

impl Part {
    /// Returns `true` if the subblock at the position of `e` repeats `e`.
    fn holds_subblock(&self, e: &Entry) -> bool {
        let mut b = [0u8; 512];
        let n = (self.span.len.saturating_sub(e.position) as usize).min(b.len());
        self.span.read(&self.file, &mut b[..n], e.position).is_ok()
            && b[..n].starts_with(b"ZISRAWSUBBLOCK")
            && n > 48
            && dir::entry(&b[48..n]).is_ok_and(|(own, _)| own == *e)
    }

    /// Returns `true` if the attachment at the position of `a` repeats `a`.
    fn holds_attachment(&self, a: &Attachment) -> bool {
        let mut b = [0u8; 48 + 128];
        self.span.read(&self.file, &mut b, a.position).is_ok()
            && b.starts_with(b"ZISRAWATTACH")
            && dir::attachment_entry(&b[48..]).is_some_and(|own| own == *a)
    }
}

/// Part `k` of the document whose master is `master`, if next to it.
fn find_part(master: &Path, k: u32, primary_guid: &[u8; 16]) -> Option<Part> {
    let stem = master.file_stem()?.to_string_lossy();
    let ext = master.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for name in [format!("{stem} ({k}){ext}"), format!("{stem}({k}){ext}")] {
        let Ok(file) = File::open(master.with_file_name(name)) else { continue };
        let Ok(len) = file.metadata().map(|m| m.len()) else { continue };
        let span = Span { base: 0, len };
        if dir::file_header(&file, span).is_ok_and(|h| h.file_part == k && &h.primary_guid == primary_guid) {
            return Some(Part { file_part: k, file, span });
        }
    }
    None
}

/// Zeiss CZI file.
#[derive(Debug)]
pub(super) struct Czi {
    /// The master first.
    parts: Vec<Part>,
    metadata: u64,
    blocks: Box<[Block]>,
    scenes: Vec<Scene>,
    metas: Vec<SeriesMeta>,
    attachments: Vec<Attachment>,
    /// Associated images and the index of their attachment.
    associated_kinds: Vec<AssociatedKind>,
    associated: Vec<usize>,
    /// Time of each time point from the `TimeStamps` attachment, seconds.
    time_stamps: Vec<f64>,
}

impl Czi {
    /// Opens a CZI file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid segment structure and
    /// [`Error::UnsupportedVariant`] for a following part of a multi-file
    /// document and pixel types not supported.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Self::open_span(file, Span { base: 0, len }, Some(path))
    }

    /// Opens the CZI in `span` of `file`; `path` locates following parts.
    fn open_span(file: File, span: Span, path: Option<&Path>) -> Result<Self> {
        let header = dir::file_header(&file, span)?;
        if header.file_part != 0 {
            return Err(Error::UnsupportedVariant("CZI file part of a multi-file document; open the first part".into()));
        }
        if header.directory == 0 {
            return Err(Error::Corrupt("CZI without subblock directory".into()));
        }
        let mut entries = dir::directory(&file, span, header.directory)?;
        let xml = if header.metadata != 0 { dir::metadata_xml(&file, span, header.metadata)? } else { String::new() };
        let xml_meta = meta::parse(&xml);
        let mut attachments =
            if header.attachments != 0 { dir::attachments(&file, span, header.attachments).unwrap_or_default() } else { Vec::new() };
        let mut parts = vec![Part { file_part: 0, file, span }];
        let wanted: BTreeSet<u32> =
            entries.iter().map(|e| e.file_part).chain(attachments.iter().map(|a| a.file_part)).filter(|&k| k != 0).collect();
        if let Some(path) = path {
            parts.extend(wanted.into_iter().take(MAX_PARTS).filter_map(|k| find_part(path, k, &header.primary_guid)));
        }
        let present = |k: u32| parts.iter().any(|p| p.file_part == k);
        for e in &mut entries {
            if !present(e.file_part) && parts[0].holds_subblock(e) {
                e.file_part = 0;
            }
        }
        for a in &mut attachments {
            if !present(a.file_part) && parts[0].holds_attachment(a) {
                a.file_part = 0;
            }
        }
        let (blocks, scenes, metas) = build_index(&entries, &xml_meta)?;
        let mut czi = Czi {
            parts,
            metadata: header.metadata,
            blocks,
            scenes,
            metas,
            associated_kinds: Vec::new(),
            associated: Vec::new(),
            attachments,
            time_stamps: Vec::new(),
        };
        czi.time_stamps = czi.time_stamps();
        if czi.time_stamps.len() > 1 {
            let n = czi.time_stamps.len();
            let increment = (czi.time_stamps[n - 1] - czi.time_stamps[0]) / (n - 1) as f64;
            for m in &mut czi.metas {
                if m.size_t > 1 && increment > 0.0 {
                    m.time_increment_s = Some(increment);
                }
            }
        }
        for (i, a) in czi.attachments.iter().enumerate() {
            let kind = match (a.name.as_str(), a.content_type.as_str()) {
                ("Thumbnail", "JPG") => AssociatedKind::Thumbnail,
                ("Label", "CZI") => AssociatedKind::Label,
                ("SlidePreview", "CZI") => AssociatedKind::Macro,
                _ => continue,
            };
            if !czi.associated_kinds.contains(&kind) {
                czi.associated_kinds.push(kind);
                czi.associated.push(i);
            }
        }
        Ok(czi)
    }

    /// Time of each time point from the `TimeStamps` attachment.
    fn time_stamps(&self) -> Vec<f64> {
        let Some(a) = self.attachments.iter().find(|a| a.content_type == "CZTIMS") else { return Vec::new() };
        let Ok(p) = self.part(a.file_part) else { return Vec::new() };
        let Ok((start, len)) = dir::attachment_data(&p.file, p.span, a.position) else { return Vec::new() };
        if !(8..=1 << 24).contains(&len) {
            return Vec::new();
        }
        let mut b = vec![0u8; len as usize];
        if p.span.read(&p.file, &mut b, start).is_err() {
            return Vec::new();
        }
        let count = u32::from_le_bytes(b[4..8].try_into().expect("4 bytes")) as usize;
        b[8..].chunks_exact(8).take(count).map(|v| f64::from_le_bytes(v.try_into().expect("8 bytes"))).collect()
    }

    /// File part `k`.
    fn part(&self, k: u32) -> Result<&Part> {
        self.parts.iter().find(|p| p.file_part == k).ok_or_else(|| {
            Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, format!("CZI file part {k} not found next to the master")))
        })
    }

    /// Blocks of `plane` at `level` of `series`, in directory order.
    fn plane_blocks(&self, series: usize, level: usize, plane: Plane) -> &[Block] {
        let all = &self.blocks[self.scenes[series].blocks.clone()];
        let key = (series as u32, level as u8, plane.c, plane.z, plane.t);
        let start = all.partition_point(|b| b.key < key);
        let end = start + all[start..].partition_point(|b| b.key == key);
        &all[start..end]
    }

    /// Rectangle of `block` at `level`: x, y, width, height in level pixels.
    fn placement(&self, series: usize, level: usize, block: &Block) -> Rect {
        let s = &self.scenes[series];
        let f = s.factors[level];
        let x = ((block.x as i64 - s.x0) as f64 / f).floor() as i64;
        let y = ((block.y as i64 - s.y0) as f64 / f).floor() as i64;
        (x, y, block.stored_w as i64, block.stored_h as i64)
    }

    /// Composites `region` of a plane from its subblocks.
    fn compose(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let (pixel_type, spp) = self.scenes[series].geometry;
        let (rx0, ry0) = (region.x as i64, region.y as i64);
        let (rx1, ry1) = (rx0 + region.width as i64, ry0 + region.height as i64);
        let hits: Vec<(&Block, Rect)> = self
            .plane_blocks(series, level, plane)
            .iter()
            .map(|b| (b, self.placement(series, level, b)))
            .filter(|(_, (x, y, w, h))| *x < rx1 && *y < ry1 && x + w > rx0 && y + h > ry0)
            .collect();
        // One subblock covering the region writes every pixel itself.
        let single = hits.len() == 1 && {
            let (x, y, w, h) = hits[0].1;
            x <= rx0 && y <= ry0 && x + w >= rx1 && y + h >= ry1
        };
        if !single {
            for r in 0..region.height as usize {
                sink.write_with(r, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
        }
        for (block, (bx, by, bw, bh)) in hits {
            let g = Geometry { width: bw as usize, height: bh as usize, pixel_type, spp: spp as usize };
            let px = g.pixel_bytes();
            let p = self.part(block.part)?;
            let sub = dir::subblock(&p.file, p.span, block.position)?;
            let mask = if sub.attachment_len > 0 && sub.attachment_len < 1 << 24 {
                let mut b = vec![0u8; sub.attachment_len as usize];
                p.span.read(&p.file, &mut b, sub.attachment)?;
                Mask::parse(&b)
            } else {
                None
            };
            let (cx0, cx1) = (rx0.max(bx), rx1.min(bx + bw));
            let (cy0, cy1) = (ry0.max(by), ry1.min(by + bh));
            let w = (cx1 - cx0) as usize;
            let col = (cx0 - bx) as usize;
            // Writes row `y` of the subblock, `src` holding its pixels from `col`.
            let put = |sink: &mut Sink, y: i64, src: &[u8]| -> Result<()> {
                let out_y = (y - ry0) as usize;
                let out_x = (cx0 - rx0) as usize;
                let Some(mask) = &mask else {
                    return sink.write_with(out_y, out_x, w, |d| {
                        d.copy_from_slice(&src[..w * px]);
                        Ok(())
                    });
                };
                // Runs of valid pixels; invalid ones keep the background.
                let my = (y - by) as usize;
                let mut i = 0;
                while i < w {
                    let valid = mask.valid(col + i, my);
                    let run = (i..w).take_while(|&j| mask.valid(col + j, my) == valid).count();
                    if valid || single {
                        sink.write_with(out_y, out_x + i, run, |d| {
                            if valid {
                                d.copy_from_slice(&src[i * px..(i + run) * px]);
                            } else {
                                d.fill(0);
                            }
                            Ok(())
                        })?;
                    }
                    i += run;
                }
                Ok(())
            };
            if block.compression == 0 {
                // Uncompressed: read the rows needed; data shorter than the
                // geometry reads as zeros.
                let stride = g.width * px;
                scratch::with(w * px, |row| {
                    for y in cy0..cy1 {
                        let at = ((y - by) as usize * stride + col * px) as u64;
                        let avail = sub.data_len.saturating_sub(at).min(row.len() as u64) as usize;
                        row[avail..].fill(0);
                        p.span.read(&p.file, &mut row[..avail], sub.data + at)?;
                        decode::bgr_to_rgb(row, &g);
                        put(sink, y, row)?;
                    }
                    Ok::<(), Error>(())
                })?;
                continue;
            }
            if g.len() > MAX_SUBBLOCK || sub.data_len > MAX_SUBBLOCK as u64 {
                return Err(Error::Corrupt(format!("CZI subblock at {} too large", block.position)));
            }
            scratch::with(sub.data_len as usize, |data| {
                p.span.read(&p.file, data, sub.data)?;
                scratch::with(g.len(), |tile| {
                    decode::decode(data, block.compression, &g, tile)?;
                    for y in cy0..cy1 {
                        let start = ((y - by) as usize * g.width + col) * px;
                        put(sink, y, &tile[start..start + w * px])?;
                    }
                    Ok::<(), Error>(())
                })
            })?;
        }
        Ok(())
    }

    /// Payload position and length of attachment `index`.
    fn attachment(&self, index: usize) -> Result<(u64, u64)> {
        let a = &self.attachments[index];
        let p = self.part(a.file_part)?;
        dir::attachment_data(&p.file, p.span, a.position)
    }
}

/// Snaps the downsampling ratio of a subblock to a level factor: a power
/// of two, else an integer, within the rounding of its stored size.
fn snap_factor(size: u32, stored: u32) -> f64 {
    let (size, stored) = (size as f64, stored.max(1) as f64);
    let ratio = size / stored;
    let (lo, hi) = (size / (stored + 2.0), size / (stored - 2.0).max(0.5));
    let mut p = 1.0;
    let mut best: Option<f64> = None;
    while p <= hi * 2.0 && p < 1e9 {
        if (lo..=hi).contains(&p) && best.is_none_or(|b: f64| (p - ratio).abs() < (b - ratio).abs()) {
            best = Some(p);
        }
        p *= 2.0;
    }
    best.unwrap_or_else(|| if (lo..=hi).contains(&ratio.round()) { ratio.round() } else { ratio })
}

type Index = (Box<[Block]>, Vec<Scene>, Vec<SeriesMeta>);

/// Groups subblocks into series and levels and builds the series metadata.
fn build_index(entries: &[Entry], xml: &meta::XmlMeta) -> Result<Index> {
    // Series key: scene and the extra dimensions, in key order.
    let mut groups: BTreeMap<(i32, [i32; 5]), Vec<&Entry>> = BTreeMap::new();
    for e in entries {
        // Super-resolved renderings (more stored than logical pixels).
        if e.stored_w > e.w || e.stored_h > e.h || e.stored_w == 0 || e.stored_h == 0 {
            continue;
        }
        groups.entry((e.s, e.extra)).or_default().push(e);
    }
    // Extra dimensions that vary within a scene name the series.
    let varying: Vec<bool> = (0..5)
        .map(|d| {
            let mut by_scene: BTreeMap<i32, i32> = BTreeMap::new();
            groups.keys().any(|(s, x)| *by_scene.entry(*s).or_insert(x[d]) != x[d])
        })
        .collect();
    let scene_count = groups.keys().map(|(s, _)| *s).collect::<std::collections::BTreeSet<_>>().len();
    let mut blocks = Vec::with_capacity(entries.len());
    let mut scenes = Vec::new();
    let mut metas = Vec::new();
    for ((s, extra), list) in &groups {
        let level0: Vec<&&Entry> = list.iter().filter(|e| e.stored_w == e.w && e.stored_h == e.h).collect();
        if level0.is_empty() {
            continue;
        }
        let series = scenes.len() as u32;
        let geometry = decode::layout(level0[0].pixel_type)?;
        if list.iter().any(|e| decode::layout(e.pixel_type).ok() != Some(geometry)) {
            return Err(Error::UnsupportedVariant("CZI scene with mixed pixel types".into()));
        }
        let x0 = level0.iter().map(|e| e.x as i64).min().unwrap_or(0);
        let y0 = level0.iter().map(|e| e.y as i64).min().unwrap_or(0);
        let x1 = level0.iter().map(|e| e.x as i64 + e.w as i64).max().unwrap_or(0);
        let y1 = level0.iter().map(|e| e.y as i64 + e.h as i64).max().unwrap_or(0);
        let min = |f: fn(&Entry) -> i32| level0.iter().map(|e| f(e)).min().unwrap_or(0);
        let max = |f: fn(&Entry) -> i32| level0.iter().map(|e| f(e)).max().unwrap_or(0);
        let (c0, z0, t0) = (min(|e| e.c), min(|e| e.z), min(|e| e.t));
        let (size_c, size_z, size_t) = ((max(|e| e.c) - c0 + 1) as u32, (max(|e| e.z) - z0 + 1) as u32, (max(|e| e.t) - t0 + 1) as u32);
        // Levels: distinct factors of the downsampled subblocks.
        let mut factors = vec![1.0f64];
        let factor_of = |e: &Entry| if e.stored_w == e.w && e.stored_h == e.h { 1.0 } else { snap_factor(e.w, e.stored_w) };
        for e in list {
            let f = factor_of(e);
            if f > 1.0 && !factors.iter().any(|g| (g - f).abs() / g < 1e-6) {
                factors.push(f);
            }
        }
        factors.sort_by(|a, b| a.total_cmp(b));
        let (w0, h0) = ((x1 - x0) as f64, (y1 - y0) as f64);
        factors.retain(|f| (w0 / f).floor() >= 1.0 && (h0 / f).floor() >= 1.0);
        let start = blocks.len();
        for e in list {
            let f = factor_of(e);
            let Some(level) = factors.iter().position(|g| (g - f).abs() / g < 1e-6) else { continue };
            let rel = |v: i32, origin: i32| (v - origin).max(0) as u32;
            let (c, z, t) = (rel(e.c, c0), rel(e.z, z0), rel(e.t, t0));
            if c >= size_c || z >= size_z || t >= size_t {
                continue;
            }
            blocks.push(Block {
                position: e.position,
                part: e.file_part,
                x: e.x,
                y: e.y,
                stored_w: e.stored_w,
                stored_h: e.stored_h,
                compression: e.compression,
                key: (series, level as u8, c, z, t),
            });
        }
        // Stable: within a plane, later directory entries are drawn on top.
        blocks[start..].sort_by_key(|b| b.key);
        let levels: Vec<LevelMeta> = factors
            .iter()
            .enumerate()
            .map(|(l, &f)| {
                let in_plane: Vec<&Block> = blocks[start..].iter().filter(|b| b.key == (series, l as u8, 0, 0, 0)).collect();
                // A virtual tile grid of the typical subblock size for mosaics.
                let tile = (in_plane.len() > 1).then(|| {
                    let mut sizes: Vec<(u32, u32)> = in_plane.iter().map(|b| (b.stored_w, b.stored_h)).collect();
                    sizes.sort_unstable();
                    sizes[sizes.len() / 2]
                });
                LevelMeta { size_x: (w0 / f).floor() as u64, size_y: (h0 / f).floor() as u64, tile, downsample: f }
            })
            .collect();
        let mut name = xml.scenes.get(s).cloned().or_else(|| (scene_count > 1).then(|| format!("Scene #{s}")));
        let extras: Vec<String> = (0..5)
            .filter(|&d| varying[d])
            .map(|d| format!("{}={}", dir::EXTRA_DIMS[d] as char, extra[d]))
            .collect();
        if !extras.is_empty() {
            let suffix = extras.join(" ");
            name = Some(match name {
                Some(n) => format!("{n} {suffix}"),
                None => suffix,
            });
        }
        let mut channels = xml.channels.clone();
        channels.resize(size_c as usize, Default::default());
        let (pixel_type, spp) = geometry;
        metas.push(SeriesMeta {
            name,
            acquisition_date: xml.date.clone(),
            size_c,
            size_z,
            size_t,
            pixel_type,
            significant_bits: xml.significant_bits.filter(|&b| (b as usize) < pixel_type.bytes() * 8 && pixel_type != PixelType::F32),
            samples_per_pixel: spp,
            levels,
            physical_size: PhysicalSize { x_nm: xml.scale_nm[0], y_nm: xml.scale_nm[1].or(xml.scale_nm[0]), z_nm: xml.scale_nm[2] },
            channels,
            objective: xml.objective.clone(),
            ..Default::default()
        });
        scenes.push(Scene { x0, y0, factors, geometry, blocks: start..blocks.len() });
    }
    if scenes.is_empty() {
        return Err(Error::Corrupt("CZI without full-resolution subblocks".into()));
    }
    Ok((blocks.into_boxed_slice(), scenes, metas))
}

impl Backend for Czi {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        let first = self.time_stamps.first().copied().unwrap_or(0.0);
        Ok(PlaneMeta { delta_t_s: self.time_stamps.get(plane.t as usize).map(|t| t - first), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        for a in &self.attachments {
            meta.pairs.push((format!("Attachment {}", a.name), a.content_type.clone()));
        }
        if self.metadata != 0 {
            meta.documents.push(("CZI-XML".into(), dir::metadata_xml(&self.parts[0].file, self.parts[0].span, self.metadata)?));
        }
        Ok(meta)
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let index = self
            .associated_kinds
            .iter()
            .position(|k| k == kind)
            .map(|i| self.associated[i])
            .ok_or_else(|| Error::OutOfRange(format!("no associated image {kind:?}")))?;
        let (start, len) = self.attachment(index)?;
        let p = self.part(self.attachments[index].file_part)?;
        if self.attachments[index].content_type == "JPG" {
            return jpeg_image(&p.file, p.span, start, len);
        }
        // An embedded CZI: its coarsest level within the size limit.
        let inner = Czi::open_span(p.file.try_clone()?, Span { base: p.span.base + start, len }, None)?;
        let meta = &inner.metas[0];
        let level = meta.levels.iter().position(|l| l.size_x.max(l.size_y) <= ASSOCIATED_MAX).unwrap_or(meta.levels.len() - 1);
        let l = &meta.levels[level];
        let region = Region::new(0, 0, l.size_x, l.size_y);
        native_image(region, meta.samples_per_pixel, meta.pixel_type, |sink| inner.compose(0, level, Plane::default(), region, sink))
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        self.compose(series, level, plane, region, sink)
    }
}

/// Decodes a JPEG attachment.
fn jpeg_image(file: &File, span: Span, start: u64, len: u64) -> Result<NativeImage> {
    if len > 64 << 20 {
        return Err(Error::Corrupt("CZI thumbnail too large".into()));
    }
    let mut data = vec![0u8; len as usize];
    span.read(file, &mut data, start)?;
    #[cfg(feature = "jpeg")]
    {
        use crate::codecs::jpeg::{Color, decode};
        let (w, h) = jpeg_size(&data).ok_or_else(|| Error::Corrupt("CZI thumbnail without JPEG frame".into()))?;
        let mut out = vec![0u8; w * h * 3];
        let info = decode(&data, None, Color::YcbcrToRgb, &mut out)?;
        out.truncate(info.width * info.height * info.components);
        Ok(NativeImage {
            width: info.width as u64,
            height: info.height as u64,
            samples_per_pixel: info.components as u8,
            pixel_type: PixelType::U8,
            data: out,
        })
    }
    #[cfg(not(feature = "jpeg"))]
    Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
}

/// Width and height from the first SOF marker of a JPEG stream.
#[cfg(feature = "jpeg")]
fn jpeg_size(b: &[u8]) -> Option<(usize, usize)> {
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
            // Thumbnails are small; larger sizes come from corrupt files.
            return (w > 0 && h > 0 && w * h <= 1 << 26).then_some((w, h));
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
    fn snaps_level_factors() {
        assert_eq!(snap_factor(1024, 512), 2.0);
        assert_eq!(snap_factor(1025, 512), 2.0);
        // A 1/128 tile with rounded stored size.
        assert_eq!(snap_factor(65_620, 512), 128.0);
        assert_eq!(snap_factor(2187, 81), 27.0);
        assert_eq!(snap_factor(300, 100), 3.0);
    }
}

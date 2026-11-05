//! # sdt
//!
//! Becker & Hickl SPCM data reader (`.sdt`, TCSPC / FLIM).
//!
//! The 42-byte file header gives the offsets of the info text, the setup
//! block, the measurement descriptions (`MEASURE_INFO`, one per module or
//! channel) and the first data block. Each data block header (22 bytes;
//! 40-bit offsets from revision 15) gives the data offset, the next block,
//! the block type (creation mode, contents, element type, ZIP or LZ4
//! compression), its measurement description and the uncompressed length.
//! Image blocks hold one decay histogram of `adc_re` time bins per pixel,
//! stored y, x, time bin (fastest); the image size is `image_x` / `image_y`
//! of the measurement description, else `scan_x` / `scan_y`. Other blocks
//! (single decays, FCS, MCS) are one row of `adc_re` bins.
//!
//! Each data block is a series; the time bins are T (with the bin width
//! `tac_r / (tac_g * adc_re)` as time increment), as Bio-Formats reports
//! them. Uncompressed blocks are read in place. ZIP blocks (a ZIP archive
//! with one deflated member) are inflated as a stream once per requested
//! region, keeping only that region's pixels with all their time bins
//! (time bins are fastest, so any plane touches every bin of its pixels);
//! further planes of the same region come from that copy. A 64 x 64 crop
//! of a 512 x 512 x 4096-bin block holds 33 MB instead of the 2 GB block.
//! LZ4 blocks are not supported yet.
//!
//! Provenance: `sdtfile` (BSD-3-Clause, Christoph Gohlke; read as
//! documentation, notice in THIRD_PARTY_NOTICES.md) for the header, block
//! header and measurement description layouts and the block type bits; the
//! bh TCSPC Handbook for the meaning of the TAC parameters; own analysis of
//! the SDT sample on the OME sample server (CC BY 4.0); checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Sdt, "Becker & Hickl SPCM", "1").extensions(&["sdt"]);

const FILE_HEADER: usize = 42;
const BLOCK_HEADER: usize = 22;
/// Most data blocks followed, against loops.
const MAX_BLOCKS: usize = 1 << 16;
/// Largest decompressed block held in memory.
const MAX_INFLATED: u64 = 4 << 30;

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn i16_at(b: &[u8], at: usize) -> i16 {
    u16_at(b, at) as i16
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}
fn i32_at(b: &[u8], at: usize) -> i32 {
    u32_at(b, at) as i32
}
fn f32_at(b: &[u8], at: usize) -> f64 {
    f32::from_bits(u32_at(b, at)) as f64
}

/// Returns `true` for an SDT file: a valid header pointing to its info text.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    if head.len() < FILE_HEADER || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("sdt")) {
        return false;
    }
    let valid = matches!(u16_at(head, 40), 0x55AA | 0xAA55) || u16_at(head, 32) == 0x5555;
    let info = i32_at(head, 2) as usize;
    valid && head.get(info..).is_some_and(|t| String::from_utf8_lossy(&t[..t.len().min(64)]).contains("*IDENTIFICATION"))
}

/// One data block.
#[derive(Debug)]
struct Block {
    offset: u64,
    /// Compressed length (to the next block) for ZIP blocks.
    stored: u64,
    zip: bool,
    width: u64,
    bins: u64,
    /// The last decoded region of a ZIP block.
    cache: Mutex<Option<Arc<SubCube>>>,
}

/// Pixels `x0..x1` of rows `y0..y1` of a block with all their time bins,
/// plane by plane (time bin, then y, then x).
#[derive(Debug)]
struct SubCube {
    x0: u64,
    x1: u64,
    y0: u64,
    y1: u64,
    data: Vec<u8>,
}

/// SDT file.
#[derive(Debug)]
pub(super) struct Sdt {
    file: File,
    blocks: Vec<Block>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Sdt {
    /// Opens an SDT file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header or blocks and
    /// [`Error::UnsupportedVariant`] for LZ4 or floating-point blocks.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let read = |at: u64, n: usize| -> Result<Vec<u8>> {
            if at.checked_add(n as u64).is_none_or(|e| e > len) {
                return Err(Error::Corrupt("SDT structure beyond the end of the file".into()));
            }
            let mut b = vec![0u8; n];
            read_exact_at(&file, &mut b, at)?;
            Ok(b)
        };
        let h = read(0, FILE_HEADER)?;
        let revision = u16_at(&h, 0) as u32 & 0xF;
        let (info_offs, info_len) = (i32_at(&h, 2).max(0) as u64, u16_at(&h, 6) as usize);
        let mut blocks_n = i16_at(&h, 18) as i64;
        if blocks_n == 0x7FFF {
            blocks_n = u32_at(&h, 34) as i64;
        }
        let (meas_offs, meas_n, meas_len) = (i32_at(&h, 24).max(0) as u64, i16_at(&h, 28).max(0) as usize, i16_at(&h, 30).max(0) as usize);
        let info = String::from_utf8_lossy(&read(info_offs, info_len).unwrap_or_default()).replace("\r\n", "\n");
        // Measurement descriptions: the fields up to the image size.
        let meas: Vec<Vec<u8>> = (0..meas_n.min(1024)).filter_map(|i| read(meas_offs + (i * meas_len) as u64, meas_len.min(512)).ok()).collect();
        let field = |m: &Vec<u8>, at: usize, n: usize| (m.len() >= at + n).then_some(at);
        let mut blocks = Vec::new();
        let mut metas = Vec::new();
        let mut at = i32_at(&h, 14).max(0) as u64;
        for _ in 0..(blocks_n.clamp(0, MAX_BLOCKS as i64)) {
            let b = read(at, BLOCK_HEADER)?;
            let (data, next) = if revision >= 15 {
                (u32_at(&b, 2) as u64 | (b[0] as u64) << 32, u32_at(&b, 6) as u64 | (b[1] as u64) << 32)
            } else {
                (u32_at(&b, 2) as u64, u32_at(&b, 6) as u64)
            };
            let block_type = u16_at(&b, 10);
            let m = meas
                .get(i16_at(&b, 12).max(0) as usize)
                .or(meas.first())
                .ok_or_else(|| Error::Corrupt("SDT block without measurement description".into()))?;
            let length = u32_at(&b, 18) as u64;
            let pixel_type = match block_type & 0xF00 {
                0x000 => PixelType::U16,
                0x100 => PixelType::U32,
                0x200 => PixelType::F64,
                t => return Err(Error::UnsupportedVariant(format!("SDT element type 0x{t:x}"))),
            };
            if block_type & 0x4000 != 0 {
                return Err(Error::UnsupportedVariant("SDT LZ4-compressed blocks".into()));
            }
            let zip = block_type & 0x1000 != 0;
            let bins = field(m, 82, 2).map(|a| i16_at(m, a) as u16 as u64).filter(|&v| v > 0).unwrap_or(65536);
            let count = length / pixel_type.bytes() as u64;
            let dims = |ax: usize, ay: usize| -> Option<(u64, u64)> {
                let (x, y) = (i32_at(m, field(m, ax, 4)?) as i64, i32_at(m, field(m, ay, 4)?) as i64);
                (x > 0 && y > 0 && (x * y) as u64 * bins == count).then_some((x as u64, y as u64))
            };
            // Image blocks: image_x / image_y or scan_x / scan_y; else one row.
            let (width, height) = dims(309, 313).or_else(|| dims(173, 177)).unwrap_or((count / bins.max(1), 1));
            if width == 0 || width * height * bins * pixel_type.bytes() as u64 > length.max(1) {
                at = next;
                if next == 0 {
                    break;
                }
                continue;
            }
            // A compressed block ends at the next block, the last one at the end of the file.
            let end = if next > data { next.min(len) } else { len };
            let stored = if zip { end.saturating_sub(data) } else { length };
            if !zip && data + length > len {
                return Err(Error::Corrupt("SDT data block beyond the end of the file".into()));
            }
            let tac_r = field(m, 64, 4).map(|a| f32_at(m, a)).unwrap_or(0.0);
            let tac_g = field(m, 68, 2).map(|a| i16_at(m, a) as f64).unwrap_or(0.0);
            let bin_s = (tac_g > 0.0).then(|| tac_r / (tac_g * bins as f64)).filter(|v| v.is_finite() && *v > 0.0);
            let text = |a: usize, n: usize| String::from_utf8_lossy(&m[a..(a + n).min(m.len())]).trim_end_matches('\0').trim().to_string();
            let date = (m.len() >= 20).then(|| format!("{}T{}", text(9, 11), text(0, 9))).filter(|d| d.len() > 2);
            if bins > u32::MAX as u64 || width > u32::MAX as u64 {
                return Err(Error::Corrupt("SDT block dimensions too large".into()));
            }
            metas.push(SeriesMeta {
                name: Some(format!("block {}", blocks.len() + 1)),
                acquisition_date: date,
                size_c: 1,
                size_z: 1,
                size_t: bins as u32,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize::default(),
                time_increment_s: bin_s,
                channels: vec![ChannelMeta::default()],
                ..Default::default()
            });
            blocks.push(Block { offset: data, stored, zip, width, bins, cache: Mutex::new(None) });
            if next == 0 {
                break;
            }
            at = next;
        }
        if blocks.is_empty() {
            return Err(Error::Corrupt("SDT file without image data block".into()));
        }
        let mut original = OriginalMeta::default();
        for line in info.lines() {
            if let Some((k, v)) = line.split_once(':') {
                original.pairs.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        original.documents.push(("SDT info".into(), info));
        Ok(Sdt { file, blocks, metas, original })
    }

    /// Rows `y0..y1`, pixels `x0..x1` of a ZIP block (one deflated member)
    /// with all time bins: inflated as a stream, only that region kept,
    /// plane by plane. The last region is cached and serves every request
    /// inside it.
    fn subcube(&self, b: &Block, region: Region, e: u64) -> Result<Arc<SubCube>> {
        let (x0, x1, y0, y1) = (region.x, region.x + region.width, region.y, region.y + region.height);
        let mut cache = b.cache.lock().map_err(|_| Error::Corrupt("SDT cache poisoned".into()))?;
        if let Some(c) = cache.as_ref().filter(|c| c.x0 <= x0 && x1 <= c.x1 && c.y0 <= y0 && y1 <= c.y1) {
            return Ok(c.clone());
        }
        let row = b.width * b.bins * e;
        let pixel = b.bins * e;
        let (w, h) = (x1 - x0, y1 - y0);
        let (len, total) = (w * pixel, h * w * pixel);
        if total > MAX_INFLATED || b.stored > MAX_INFLATED {
            return Err(Error::UnsupportedVariant("compressed SDT block above 4 GiB".into()));
        }
        let mut zip = vec![0u8; b.stored as usize];
        read_exact_at(&self.file, &mut zip, b.offset)?;
        // ZIP local file header: signature, method at 8, name and extra lengths at 26.
        if zip.len() < 30 || &zip[..4] != b"PK\x03\x04" {
            return Err(Error::Corrupt("SDT block is not a ZIP archive".into()));
        }
        let method = u16_at(&zip, 8);
        let start = 30 + u16_at(&zip, 26) as usize + u16_at(&zip, 28) as usize;
        let body = zip.get(start..).ok_or_else(|| Error::Corrupt("SDT ZIP header truncated".into()))?;
        // Planes of the region (a short block leaves the rest zero); each
        // region row is gathered in stored order, then transposed.
        let mut data = vec![0u8; total as usize];
        let mut staged = vec![0u8; len as usize];
        let end = (y1 - 1) * row + x1 * pixel;
        let mut at = 0u64;
        let mut take = |chunk: &[u8]| -> bool {
            let (c0, c1) = (at, at + chunk.len() as u64);
            for r in (c0 / row).max(y0)..=((c1 - 1) / row).min(y1 - 1) {
                let (a, z) = (r * row + x0 * pixel, r * row + x1 * pixel);
                let (s, t) = (a.max(c0), z.min(c1));
                if s < t {
                    staged[(s - a) as usize..(t - a) as usize].copy_from_slice(&chunk[(s - c0) as usize..(t - c0) as usize]);
                    if t == z {
                        transpose_row(&staged, &mut data, (r - y0) as usize, w as usize, h as usize, b.bins as usize, e as usize);
                    }
                }
            }
            at = c1;
            at < end
        };
        match method {
            0 => {
                take(&body[..body.len().min(end as usize)]);
            }
            8 => {
                scratch::with(1 << 20, |buf| crate::codecs::inflate_each(body, false, buf, |chunk| Ok(take(chunk))).map(|_| ()))?;
            }
            m => return Err(Error::UnsupportedVariant(format!("SDT ZIP method {m}"))),
        }
        let cube = Arc::new(SubCube { x0, x1, y0, y1, data });
        *cache = Some(cube.clone());
        Ok(cube)
    }
}

/// Moves one region row stored pixel by pixel (`bins` samples each) into
/// row `y` of every plane of `planes` (`bins` planes of `w` x `h`).
fn transpose_row(src: &[u8], planes: &mut [u8], y: usize, w: usize, h: usize, bins: usize, e: usize) {
    match e {
        1 => transpose::<1>(src, planes, y, w, h, bins),
        2 => transpose::<2>(src, planes, y, w, h, bins),
        4 => transpose::<4>(src, planes, y, w, h, bins),
        _ => transpose::<8>(src, planes, y, w, h, bins),
    }
}

/// [`transpose_row`] for `E`-byte samples. Bins go in groups of 32 so that
/// reads and writes stay in cache.
fn transpose<const E: usize>(src: &[u8], planes: &mut [u8], y: usize, w: usize, h: usize, bins: usize) {
    let (src, planes) = (src.as_chunks::<E>().0, planes.as_chunks_mut::<E>().0);
    for b0 in (0..bins).step_by(32) {
        let b1 = (b0 + 32).min(bins);
        for x in 0..w {
            let pixel = &src[x * bins..(x + 1) * bins];
            for b in b0..b1 {
                planes[(b * h + y) * w + x] = pixel[b];
            }
        }
    }
}

impl Backend for Sdt {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let b = &self.blocks[series];
        let e = self.metas[series].pixel_type.bytes();
        let bins = b.bins as usize;
        let w = region.width as usize;
        // One image row spans `w * bins` elements; plane t is every bins-th.
        let span = w * bins * e;
        let little = cfg!(target_endian = "little");
        if b.zip {
            // Plane t of the cached region: contiguous rows.
            let c = self.subcube(b, region, e as u64)?;
            let (cw, ch) = ((c.x1 - c.x0) as usize, (c.y1 - c.y0) as usize);
            for r in 0..region.height as usize {
                let y = (region.y - c.y0) as usize + r;
                let at = ((plane.t as usize * ch + y) * cw + (region.x - c.x0) as usize) * e;
                let src = &c.data[at..at + w * e];
                sink.write_with(r, 0, w, |d| {
                    d.copy_from_slice(src);
                    if e > 1 && !little {
                        d.chunks_exact_mut(e).for_each(<[u8]>::reverse);
                    }
                    Ok(())
                })?;
            }
            return Ok(());
        }
        scratch::with(span, |row| {
            for r in 0..region.height as usize {
                let first = ((region.y + r as u64) * b.width + region.x) * b.bins * e as u64;
                read_exact_at(&self.file, row, b.offset + first)?;
                sink.write_with(r, 0, w, |d| {
                    for (k, out) in d.chunks_exact_mut(e).enumerate() {
                        let s = (k * bins + plane.t as usize) * e;
                        out.copy_from_slice(&row[s..s + e]);
                        if e > 1 && !little {
                            out.reverse();
                        }
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

//! # klb
//!
//! Keller Lab Block (`.klb`) reader.
//!
//! Little-endian header: `u8` version, `u32 xyzct[5]` (x, y, z, channel
//! and time sizes), `f32 pixelSize[5]` (µm for x / y / z, seconds for t),
//! `u8` data type (0 uint8, 1 uint16, 2 uint32, 3 uint64, 4 int8, 5 int16,
//! 6 int32, 7 int64, 8 float32, 9 float64), `u8` compression (0 none,
//! 1 bzip2), `char metadata[256]`, `u32 blockSize[5]` and one `u64` per
//! block: the end of the block's data, counted from the end of the header.
//! Blocks partition the 5-D array, x fastest, then y, z, c, t; each holds
//! its (at the array edges clipped) samples x fastest. A plane region is
//! read from the blocks it touches; decompressed blocks are kept in a small
//! cache, as a block spans several z planes.
//!
//! Provenance: KLB README (F. Amat, Janelia, `doc/klb/`; header items) and
//! Amat et al., Nature Protocols 2015, supplementary material; own analysis
//! of the KLB samples on the OME sample server (block offsets, data type and
//! compression codes, byte order); checked against Bio-Formats and
//! bioformats-rs as black boxes. bzip2 by `bzip2-rs` (MIT / Apache-2.0).
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Klb, "Keller Lab Block", "1").extensions(&["klb"]);

/// Bytes before the block offsets.
const FIXED: usize = 1 + 20 + 20 + 1 + 1 + 256 + 20;
/// Most blocks accepted, against corrupt sizes.
const MAX_BLOCKS: u64 = 1 << 26;
/// Largest decompressed block.
const MAX_BLOCK_BYTES: u64 = 1 << 30;
/// Decompressed bytes kept in the block cache.
const CACHE_BYTES: usize = 64 << 20;

/// Returns `true` for a KLB file: `.klb` with a plausible header.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("klb")) && head.len() >= FIXED && head[0] >= 1 && head[41] <= 9 && head[42] <= 1
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

/// KLB file.
#[derive(Debug)]
pub(super) struct Klb {
    file: File,
    /// Sizes and block sizes, x y z c t.
    dims: [u64; 5],
    block: [u64; 5],
    /// Blocks per dimension.
    grid: [u64; 5],
    /// Start of the block data and the end offset of each block.
    data_at: u64,
    ends: Vec<u64>,
    bzip2: bool,
    bytes: usize,
    cache: Mutex<VecDeque<(usize, Arc<Vec<u8>>)>>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Klb {
    /// Opens a KLB file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header and
    /// [`Error::UnsupportedVariant`] for 64-bit integer samples or other
    /// compressions.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut head = [0u8; FIXED];
        read_exact_at(&file, &mut head, 0).map_err(|_| Error::Corrupt("KLB header truncated".into()))?;
        let dims: [u64; 5] = std::array::from_fn(|i| u32_at(&head, 1 + 4 * i) as u64);
        let pixel: [f32; 5] = std::array::from_fn(|i| f32::from_bits(u32_at(&head, 21 + 4 * i)));
        let pixel_type = match head[41] {
            0 => PixelType::U8,
            1 => PixelType::U16,
            2 => PixelType::U32,
            4 => PixelType::I8,
            5 => PixelType::I16,
            6 => PixelType::I32,
            8 => PixelType::F32,
            9 => PixelType::F64,
            3 | 7 => return Err(Error::UnsupportedVariant("KLB 64-bit integer samples".into())),
            t => return Err(Error::Corrupt(format!("KLB data type {t}"))),
        };
        let bzip2 = match head[42] {
            0 => false,
            1 => true,
            c => return Err(Error::UnsupportedVariant(format!("KLB compression {c}"))),
        };
        let block: [u64; 5] = std::array::from_fn(|i| u32_at(&head, FIXED - 20 + 4 * i) as u64);
        if dims.contains(&0) || block.contains(&0) || dims[3] > u32::MAX as u64 {
            return Err(Error::Corrupt("KLB header with zero sizes".into()));
        }
        let grid: [u64; 5] = std::array::from_fn(|i| dims[i].div_ceil(block[i]));
        let n = grid
            .iter()
            .try_fold(1u64, |a, &g| a.checked_mul(g))
            .filter(|&n| n <= MAX_BLOCKS)
            .ok_or_else(|| Error::Corrupt("KLB with too many blocks".into()))?;
        let bytes = pixel_type.bytes();
        let block_bytes = block.iter().product::<u64>().saturating_mul(bytes as u64);
        if block_bytes > MAX_BLOCK_BYTES {
            return Err(Error::Corrupt("KLB blocks too large".into()));
        }
        let data_at = FIXED as u64 + 8 * n;
        if data_at > len {
            return Err(Error::Corrupt("KLB block offsets truncated".into()));
        }
        let mut raw = vec![0u8; 8 * n as usize];
        read_exact_at(&file, &mut raw, FIXED as u64)?;
        let ends: Vec<u64> = raw.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes"))).collect();
        if ends.windows(2).any(|w| w[1] < w[0]) || ends.last().is_some_and(|&e| data_at.saturating_add(e) > len) {
            return Err(Error::Corrupt("KLB block offsets out of order or beyond the file".into()));
        }
        let um = |v: f32| Some(v as f64).filter(|v| v.is_finite() && *v > 0.0).map(|v| v * 1e3);
        let meta = SeriesMeta {
            name: path.file_stem().map(|n| n.to_string_lossy().into_owned()),
            size_c: dims[3] as u32,
            size_z: dims[2].min(u32::MAX as u64) as u32,
            size_t: dims[4].min(u32::MAX as u64) as u32,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta {
                size_x: dims[0],
                size_y: dims[1],
                tile: Some((block[0].min(u32::MAX as u64) as u32, block[1].min(u32::MAX as u64) as u32)),
                downsample: 1.0,
            }],
            physical_size: PhysicalSize { x_nm: um(pixel[0]), y_nm: um(pixel[1]), z_nm: um(pixel[2]).filter(|_| dims[2] > 1) },
            time_increment_s: Some(pixel[4] as f64).filter(|v| v.is_finite() && *v > 0.0 && dims[4] > 1),
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        original.pairs.push(("headerVersion".into(), head[0].to_string()));
        let text = &head[43..299];
        let text = String::from_utf8_lossy(&text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())]).into_owned();
        if !text.is_empty() {
            original.pairs.push(("metadata".into(), text));
        }
        original.pairs.push(("blockSize".into(), format!("{block:?}")));
        Ok(Klb { file, dims, block, grid, data_at, ends, bzip2, bytes, cache: Mutex::new(VecDeque::new()), metas: vec![meta], original })
    }

    /// Size of block `b` (clipped at the array edges) per dimension.
    fn block_dims(&self, index: [u64; 5]) -> [u64; 5] {
        std::array::from_fn(|i| self.block[i].min(self.dims[i] - index[i] * self.block[i]))
    }

    /// Decompressed samples of block `b`.
    fn load(&self, b: usize, index: [u64; 5]) -> Result<Arc<Vec<u8>>> {
        if let Some((_, d)) = self.cache.lock().map_err(|_| Error::Corrupt("KLB cache poisoned".into()))?.iter().find(|(i, _)| *i == b) {
            return Ok(d.clone());
        }
        let start = if b == 0 { 0 } else { self.ends[b - 1] };
        let mut src = vec![0u8; (self.ends[b] - start) as usize];
        read_exact_at(&self.file, &mut src, self.data_at + start)?;
        let want = self.block_dims(index).iter().product::<u64>() as usize * self.bytes;
        let data = if self.bzip2 {
            let mut out = Vec::with_capacity(want);
            let mut dec = bzip2_rs::DecoderReader::new(src.as_slice());
            std::io::Read::read_to_end(&mut std::io::Read::take(&mut dec, want as u64), &mut out)
                .map_err(|e| Error::Corrupt(format!("KLB bzip2 block: {e}")))?;
            out
        } else {
            src
        };
        if data.len() < want {
            return Err(Error::Corrupt("KLB block shorter than its size".into()));
        }
        let data = Arc::new(data);
        let mut cache = self.cache.lock().map_err(|_| Error::Corrupt("KLB cache poisoned".into()))?;
        cache.push_front((b, data.clone()));
        let mut total = 0;
        cache.retain(|(_, d)| {
            total += d.len();
            total <= CACHE_BYTES
        });
        Ok(data)
    }
}

impl Backend for Klb {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let e = self.bytes;
        let fixed = [plane.z as u64, plane.c as u64, plane.t as u64];
        let (bz, bc, bt) = (fixed[0] / self.block[2], fixed[1] / self.block[3], fixed[2] / self.block[4]);
        let (x0, x1) = (region.x, region.x + region.width);
        let (y0, y1) = (region.y, region.y + region.height);
        for by in y0 / self.block[1]..y1.div_ceil(self.block[1]) {
            for bx in x0 / self.block[0]..x1.div_ceil(self.block[0]) {
                let index = [bx, by, bz, bc, bt];
                let b = (((bt * self.grid[3] + bc) * self.grid[2] + bz) * self.grid[1] + by) * self.grid[0] + bx;
                let data = self.load(b as usize, index)?;
                let d = self.block_dims(index);
                // Position of this plane inside the block.
                let (lz, lc, lt) = (fixed[0] - bz * self.block[2], fixed[1] - bc * self.block[3], fixed[2] - bt * self.block[4]);
                let plane_at = ((lt * d[3] + lc) * d[2] + lz) * d[1] * d[0];
                let (ox, oy) = (bx * self.block[0], by * self.block[1]);
                let (cx0, cx1) = (x0.max(ox), x1.min(ox + d[0]));
                for y in y0.max(oy)..y1.min(oy + d[1]) {
                    let at = ((plane_at + (y - oy) * d[0] + (cx0 - ox)) as usize) * e;
                    let row = &data[at..at + (cx1 - cx0) as usize * e];
                    sink.write_with((y - y0) as usize, (cx0 - x0) as usize, (cx1 - cx0) as usize, |out| {
                        out.copy_from_slice(row);
                        if e > 1 && cfg!(target_endian = "big") {
                            out.chunks_exact_mut(e).for_each(<[u8]>::reverse);
                        }
                        Ok(())
                    })?;
                }
            }
        }
        Ok(())
    }
}

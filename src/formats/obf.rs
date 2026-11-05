//! # obf
//!
//! Imspector / Abberior OBF reader (`.obf`, and `.msr` files that carry the
//! OBF header).
//!
//! The file header (`OMAS_BF\n\xff\xff`, format version, first stack
//! position, description) leads to a chain of stacks. Each stack header
//! gives the rank, the pixel counts, physical lengths and offsets of up to
//! 15 axes, the data type, the compression (0 none, 1 zlib), name and
//! description lengths, the on-disk data length and the next stack. From
//! stack version 1 a footer follows the data (`data_len_disk` after the
//! description): its fixed part (read up to its `size`) holds the SI units
//! of the axes, the number of samples written (a short write leaves the
//! rest zero) and the number of chunk positions; the variable part the axis
//! labels, column positions and labels, metadata, flush points, tag
//! dictionary and chunk positions. With chunk positions (stack version 6)
//! the data stream is split into chunks at `start + file_offset`; the last
//! chunk ends at the footer.
//!
//! Each stack is a series. Axes 0 and 1 are x and y; further axes are z
//! (label `Z` or length unit metres), time (seconds) or channels. Pixel
//! sizes are `len / res` of the metre axes. Uncompressed stacks are read in
//! place; zlib stacks are decompressed into memory on first read.
//!
//! Not supported: `.msr` files of the older archive layout (`CDataStack`,
//! no OBF header), complex, boolean and 64-bit integer stacks.
//!
//! Provenance: Imspector documentation "File Formats" (OBF file and stack
//! header, stack footer, SI units, flush points, chunk positions; MPI-BPC
//! and Abberior Instruments, `doc/imspector-obf/`); own analysis of the OBF
//! samples on the OME sample server (CC BY 4.0); checked against
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
use std::sync::OnceLock;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Obf, "Imspector OBF", "1").extensions(&["obf", "msr"]);

const FILE_MAGIC: &[u8] = b"OMAS_BF\n\xff\xff";
const STACK_MAGIC: &[u8] = b"OMAS_BF_STACK\n\xff\xff";
const MAX_DIMS: usize = 15;
/// Size of the stack header up to the name.
const STACK_HEADER: usize = 16 + 4 + 4 + MAX_DIMS * (4 + 8 + 8) + 5 * 4 + 3 * 8;
/// Most stacks followed, against loops.
const MAX_STACKS: usize = 4096;
/// Largest decompressed stack held in memory.
const MAX_INFLATED: u64 = 4 << 30;

/// Returns `true` for a file starting with the OBF header.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(FILE_MAGIC)
}

/// Little-endian reader over a byte slice that fails instead of panicking.
struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.at..self.at.checked_add(n).ok_or_else(truncated)?).ok_or_else(truncated)?;
        self.at += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8 bytes")))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }
}

fn truncated() -> Error {
    Error::Corrupt("OBF structure truncated".into())
}

/// One stack: geometry, data location and decoded stream.
#[derive(Debug)]
struct Stack {
    pixel_type: PixelType,
    spp: usize,
    /// Element strides of x and y, of z and t (0 if absent), and the channel
    /// axes (stride, size), fastest first.
    sx: u64,
    sy: u64,
    sz: u64,
    st: u64,
    c_axes: Vec<(u64, u64)>,
    /// Chunks of the data stream: (logical offset, file position, length).
    chunks: Vec<(u64, u64, u64)>,
    /// Bytes of stream data per the samples written.
    written: u64,
    zlib: bool,
    inflated: OnceLock<std::result::Result<Vec<u8>, String>>,
}

/// OBF file.
#[derive(Debug)]
pub(super) struct Obf {
    file: File,
    stacks: Vec<Stack>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Obf {
    /// Opens an OBF file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers and
    /// [`Error::UnsupportedVariant`] for MSR archives without OBF header or
    /// for unsupported data types.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let read = |at: u64, n: usize| -> Result<Vec<u8>> {
            if at.checked_add(n as u64).is_none_or(|e| e > len) {
                return Err(truncated());
            }
            let mut b = vec![0u8; n];
            read_exact_at(&file, &mut b, at)?;
            Ok(b)
        };
        let head = read(0, 26)?;
        if !head.starts_with(FILE_MAGIC) {
            return Err(Error::UnsupportedVariant("Imspector file without OBF header (MSR archive)".into()));
        }
        let mut c = Cur { b: &head, at: 10 };
        let version = c.u32()?;
        let mut next = c.u64()?;
        let descr_len = c.u32()? as usize;
        let mut original = OriginalMeta::default();
        original.pairs.push(("OBF format version".into(), version.to_string()));
        if descr_len > 0
            && descr_len < 1 << 24
            && let Ok(d) = read(26, descr_len)
        {
            original.documents.push(("OBF description".into(), String::from_utf8_lossy(&d).into_owned()));
        }
        let mut stacks = Vec::new();
        let mut metas = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while next != 0 && seen.insert(next) && stacks.len() < MAX_STACKS {
            let at = next;
            let h = read(at, STACK_HEADER)?;
            if !h.starts_with(STACK_MAGIC) {
                break;
            }
            let mut c = Cur { b: &h, at: 16 };
            let stack_version = c.u32()?;
            let rank = c.u32()? as usize;
            let res: Vec<u64> = (0..MAX_DIMS).map(|_| c.u32().map(u64::from)).collect::<Result<_>>()?;
            let lens: Vec<f64> = (0..MAX_DIMS).map(|_| c.f64()).collect::<Result<_>>()?;
            let _offsets: Vec<f64> = (0..MAX_DIMS).map(|_| c.f64()).collect::<Result<_>>()?;
            let (dt, compression, _level, name_len, descr_len) = (c.u32()?, c.u32()?, c.u32()?, c.u32()? as usize, c.u32()? as usize);
            let (_reserved, data_len, next_stack) = (c.u64()?, c.u64()?, c.u64()?);
            next = next_stack;
            if !(2..=MAX_DIMS).contains(&rank) || name_len > 1 << 16 || descr_len > 1 << 26 {
                continue;
            }
            let name = String::from_utf8_lossy(&read(at + STACK_HEADER as u64, name_len)?).into_owned();
            let start = at + (STACK_HEADER + name_len + descr_len) as u64;
            let (pixel_type, spp) = match dt {
                0x01 => (PixelType::U8, 1),
                0x02 => (PixelType::I8, 1),
                0x04 => (PixelType::U16, 1),
                0x08 => (PixelType::I16, 1),
                0x10 => (PixelType::U32, 1),
                0x20 => (PixelType::I32, 1),
                0x40 => (PixelType::F32, 1),
                0x80 => (PixelType::F64, 1),
                0x400 => (PixelType::U8, 3),
                0x800 => (PixelType::U8, 4),
                d => return Err(Error::UnsupportedVariant(format!("OBF data type 0x{d:x} in stack {name}"))),
            };
            let res = &res[..rank];
            let elements = res.iter().try_fold(1u64, |a, &r| a.checked_mul(r.max(1))).ok_or_else(|| Error::Corrupt("OBF stack too large".into()))?;
            let total = elements.checked_mul((pixel_type.bytes() * spp) as u64).ok_or_else(|| Error::Corrupt("OBF stack too large".into()))?;
            // Footer: units, samples written, labels and chunk positions.
            let mut units = vec![(0i32, 0i32); rank];
            let mut labels = vec![String::new(); rank];
            let mut written = total;
            let mut chunk_list: Vec<(u64, u64)> = Vec::new();
            let footer_at = start.checked_add(data_len).filter(|&f| f < len);
            if stack_version >= 1
                && let Some(foot) = footer_at
            {
                let fixed = read(foot, 4)?;
                let size = u32::from_le_bytes(fixed[..4].try_into().expect("4 bytes")) as usize;
                if (128..1 << 20).contains(&size) && foot + size as u64 <= len {
                    let f = read(foot, size)?;
                    let mut c = Cur { b: &f, at: 4 };
                    let has_pos: Vec<u32> = (0..MAX_DIMS).map(|_| c.u32()).collect::<Result<_>>()?;
                    let has_lab: Vec<u32> = (0..MAX_DIMS).map(|_| c.u32()).collect::<Result<_>>()?;
                    let meta_len = c.u32()? as u64;
                    // SI unit: nine exponent fractions and a scale factor.
                    let unit = |c: &mut Cur| -> Result<(i32, i32)> {
                        let e: Vec<(i32, i32)> = (0..9).map(|_| Ok((c.i32()?, c.i32()?))).collect::<Result<_>>()?;
                        c.f64()?;
                        Ok((e[0].0, e[2].0))
                    };
                    let (mut flushes, mut tags, mut chunks_n) = (0u64, 0u64, 0u64);
                    if size >= 208 + 80 * MAX_DIMS {
                        unit(&mut c)?;
                        for u in units.iter_mut().take(MAX_DIMS) {
                            *u = unit(&mut c)?;
                        }
                        for _ in rank..MAX_DIMS {
                            unit(&mut c)?;
                        }
                    }
                    if let (Ok(n), Ok(_block)) = (c.u64(), c.u64()) {
                        flushes = n;
                    }
                    if let Ok(t) = c.u64() {
                        tags = t;
                    }
                    let _ = (c.u64(), c.u32(), c.u64());
                    if let (Ok(samples), Ok(n)) = (c.u64(), c.u64()) {
                        if samples > 0 {
                            written = samples.saturating_mul((pixel_type.bytes() * spp) as u64).min(total);
                        }
                        chunks_n = n;
                    }
                    // Variable part: axis labels, column positions and labels,
                    // metadata, flush points, tag dictionary, chunk positions.
                    let mut p = foot + size as u64;
                    let string = |p: &mut u64| -> Result<String> {
                        let n = u32::from_le_bytes(read(*p, 4)?[..].try_into().expect("4 bytes")) as usize;
                        if n > 1 << 20 {
                            return Err(truncated());
                        }
                        let s = String::from_utf8_lossy(&read(*p + 4, n)?).into_owned();
                        *p += 4 + n as u64;
                        Ok(s)
                    };
                    let parsed = (|| -> Result<()> {
                        for l in labels.iter_mut() {
                            *l = string(&mut p)?;
                        }
                        for (i, &r) in res.iter().enumerate() {
                            if has_pos[i] != 0 {
                                p += 8 * r;
                            }
                        }
                        for (i, &r) in res.iter().enumerate() {
                            if has_lab[i] != 0 {
                                for _ in 0..r.min(1 << 20) {
                                    string(&mut p)?;
                                }
                            }
                        }
                        p += meta_len + 8 * flushes + tags;
                        if chunks_n > 0 && chunks_n < 1 << 24 {
                            let raw = read(p, 16 * chunks_n as usize)?;
                            chunk_list = raw
                                .as_chunks::<16>()
                                .0
                                .iter()
                                .map(|e| (u64::from_le_bytes(e[..8].try_into().expect("8")), u64::from_le_bytes(e[8..].try_into().expect("8"))))
                                .collect();
                        }
                        Ok(())
                    })();
                    if parsed.is_err() {
                        chunk_list.clear();
                    }
                }
            }
            // Chunks of the stream: the first at `start`, the last up to the footer.
            let stream_end = if compression == 0 { total } else { data_len };
            let mut chunks = vec![(0u64, start, 0u64)];
            for &(logical, offset) in &chunk_list {
                // Of several chunks with the same logical offset only the last has data.
                if chunks.last().is_some_and(|l| l.0 == logical) {
                    chunks.pop();
                }
                chunks.push((logical, start + offset, 0));
            }
            for k in 0..chunks.len() {
                let end = chunks.get(k + 1).map_or(stream_end.max(chunks[k].0), |n| n.0);
                let room = (start + data_len).saturating_sub(chunks[k].1);
                chunks[k].2 = end.saturating_sub(chunks[k].0).min(room);
            }
            // Axes: x, y, then z (metres or label Z), t (seconds), channels.
            let mut stride = 1u64;
            let (mut sx, mut sy, mut sz, mut st) = (1, 0, 0, 0);
            let (mut nz, mut nt) = (1u64, 1u64);
            let mut c_axes = Vec::new();
            let mut spacing = [None; 4];
            for i in 0..rank {
                let n = res[i].max(1);
                let step = (lens[i] / n as f64).abs();
                let label = labels[i].trim().to_ascii_lowercase();
                let metres = units[i].0 == 1;
                match i {
                    0 => (sx, spacing[0]) = (stride, metres.then_some(step * 1e9)),
                    1 => (sy, spacing[1]) = (stride, metres.then_some(step * 1e9)),
                    _ if nz == 1 && (label == "z" || label.ends_with(" z") || (metres && label != "t")) => {
                        (sz, nz, spacing[2]) = (stride, n, metres.then_some(step * 1e9))
                    }
                    _ if nt == 1 && (label == "t" || label.contains("time") || units[i].1 == 1) => (st, nt, spacing[3]) = (stride, n, Some(step)),
                    _ => c_axes.push((stride, n)),
                }
                stride *= n;
            }
            let size_c: u64 = c_axes.iter().map(|a| a.1).product();
            if [res[0], res[1], nz, nt, size_c].iter().any(|&v| v > u32::MAX as u64) {
                return Err(Error::Corrupt("OBF stack dimensions too large".into()));
            }
            let positive = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0);
            metas.push(SeriesMeta {
                name: Some(name.clone()),
                size_c: size_c as u32,
                size_z: nz as u32,
                size_t: nt as u32,
                pixel_type,
                samples_per_pixel: spp as u8,
                levels: vec![LevelMeta { size_x: res[0], size_y: res[1], tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: positive(spacing[0]), y_nm: positive(spacing[1]), z_nm: positive(spacing[2]).filter(|_| nz > 1) },
                time_increment_s: positive(spacing[3]).filter(|_| nt > 1),
                channels: vec![ChannelMeta::default(); size_c as usize],
                ..Default::default()
            });
            original.pairs.push((format!("Stack {} labels", stacks.len()), labels.join(", ")));
            stacks.push(Stack {
                pixel_type,
                spp,
                sx,
                sy,
                sz,
                st,
                c_axes,
                chunks,
                written: written.min(total),
                zlib: compression == 1,
                inflated: OnceLock::new(),
            });
            if compression > 1 {
                return Err(Error::UnsupportedVariant(format!("OBF compression {compression}")));
            }
        }
        if stacks.is_empty() {
            return Err(Error::Corrupt("OBF file without readable stack".into()));
        }
        Ok(Obf { file, stacks, metas, original })
    }

    /// Copies stream bytes `at..at + out.len()` of an uncompressed stack;
    /// bytes not written read as zeros.
    fn read_raw(&self, s: &Stack, mut at: u64, mut out: &mut [u8]) -> Result<()> {
        while !out.is_empty() {
            if at >= s.written {
                out.fill(0);
                return Ok(());
            }
            let k = s.chunks.partition_point(|c| c.0 <= at).saturating_sub(1);
            let (logical, pos, len) = s.chunks[k];
            let within = at - logical;
            let n = (len.saturating_sub(within)).min(out.len() as u64).min(s.written - at) as usize;
            if n == 0 {
                out.fill(0);
                return Ok(());
            }
            read_exact_at(&self.file, &mut out[..n], pos + within)?;
            out = &mut out[n..];
            at += n as u64;
        }
        Ok(())
    }

    /// The decompressed stream of a zlib stack, decoded on first use.
    fn inflated<'a>(&self, s: &'a Stack) -> Result<&'a [u8]> {
        let data = s.inflated.get_or_init(|| {
            let load = || -> Result<Vec<u8>> {
                let mut src = Vec::new();
                for &(_, pos, len) in &s.chunks {
                    if len > MAX_INFLATED {
                        return Err(Error::Corrupt("OBF chunk too large".into()));
                    }
                    let start = src.len();
                    src.resize(start + len as usize, 0);
                    read_exact_at(&self.file, &mut src[start..], pos)?;
                }
                if s.written > MAX_INFLATED {
                    return Err(Error::UnsupportedVariant("compressed OBF stack above 4 GiB".into()));
                }
                let mut out = vec![0u8; s.written as usize];
                let n = crate::codecs::deflate(&src, &mut out, true)?;
                // A stack ended early leaves the rest zero.
                out[n..].fill(0);
                Ok(out)
            };
            load().map_err(|e| e.to_string())
        });
        data.as_deref().map_err(|e| Error::Corrupt(e.clone()))
    }
}

/// Element offset of index `i` decomposed over `axes` (fastest first).
fn axes_offset(axes: &[(u64, u64)], mut i: u64) -> u64 {
    let mut at = 0;
    for &(stride, size) in axes {
        at += (i % size) * stride;
        i /= size;
    }
    at
}

impl Backend for Obf {
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
        let s = &self.stacks[series];
        let px = (s.pixel_type.bytes() * s.spp) as u64;
        let base = plane.z as u64 * s.sz + plane.t as u64 * s.st + axes_offset(&s.c_axes, plane.c as u64);
        let w = region.width as usize;
        let row_len = w * px as usize;
        let little = cfg!(target_endian = "little");
        let fix = |d: &mut [u8]| {
            if s.pixel_type.bytes() > 1 && !little {
                d.chunks_exact_mut(s.pixel_type.bytes()).for_each(<[u8]>::reverse);
            }
        };
        // OBF stores x fastest; other layouts are not written.
        if s.sx != 1 {
            return Err(Error::UnsupportedVariant("OBF stack with x not the fastest axis".into()));
        }
        let data = if s.zlib { Some(self.inflated(s)?) } else { None };
        scratch::with(row_len, |row| {
            for r in 0..region.height as usize {
                let at = (base + (region.y + r as u64) * s.sy + region.x) * px;
                match data {
                    Some(d) => {
                        let src = d.get(at as usize..at as usize + row_len);
                        match src {
                            Some(src) => row.copy_from_slice(src),
                            None => row.fill(0),
                        }
                    }
                    None => self.read_raw(s, at, row)?,
                }
                sink.write_with(r, 0, w, |d| {
                    d.copy_from_slice(row);
                    fix(d);
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

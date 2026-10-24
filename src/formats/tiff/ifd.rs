//! # ifd
//!
//! TIFF and BigTIFF container parsing: header, IFD chain and tag values.
//!
//! Reference: TIFF 6.0 sections 2 and 7, BigTIFF specification (see
//! `doc/tiff/`, `doc/bigtiff/`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use crate::io::read_exact_at;
use crate::{Error, Result};

/// Upper bound for the number of IFDs in one file, against corrupt chains.
const MAX_IFDS: usize = 1 << 22;

/// Tag numbers used by the readers.
pub(crate) mod tag {
    pub const NEW_SUBFILE_TYPE: u16 = 254;
    pub const IMAGE_WIDTH: u16 = 256;
    pub const IMAGE_LENGTH: u16 = 257;
    pub const BITS_PER_SAMPLE: u16 = 258;
    pub const COMPRESSION: u16 = 259;
    pub const PHOTOMETRIC: u16 = 262;
    pub const FILL_ORDER: u16 = 266;
    pub const IMAGE_DESCRIPTION: u16 = 270;
    pub const MAKE: u16 = 271;
    pub const MODEL: u16 = 272;
    pub const STRIP_OFFSETS: u16 = 273;
    pub const SAMPLES_PER_PIXEL: u16 = 277;
    pub const ROWS_PER_STRIP: u16 = 278;
    pub const STRIP_BYTE_COUNTS: u16 = 279;
    pub const X_RESOLUTION: u16 = 282;
    pub const Y_RESOLUTION: u16 = 283;
    pub const PLANAR_CONFIGURATION: u16 = 284;
    pub const RESOLUTION_UNIT: u16 = 296;
    pub const SOFTWARE: u16 = 305;
    pub const DATE_TIME: u16 = 306;
    pub const PREDICTOR: u16 = 317;
    pub const COLOR_MAP: u16 = 320;
    pub const TILE_WIDTH: u16 = 322;
    pub const TILE_LENGTH: u16 = 323;
    pub const TILE_OFFSETS: u16 = 324;
    pub const TILE_BYTE_COUNTS: u16 = 325;
    pub const SUB_IFDS: u16 = 330;
    pub const SAMPLE_FORMAT: u16 = 339;
    pub const JPEG_TABLES: u16 = 347;
    pub const YCBCR_SUBSAMPLING: u16 = 530;
    pub const REFERENCE_BLACK_WHITE: u16 = 532;
}

/// Sizes in bytes of the TIFF field types, indexed by type number.
fn type_size(typ: u16) -> Option<u64> {
    Some(match typ {
        1 | 2 | 6 | 7 => 1,  // BYTE, ASCII, SBYTE, UNDEFINED
        3 | 8 => 2,          // SHORT, SSHORT
        4 | 9 | 11 | 13 => 4, // LONG, SLONG, FLOAT, IFD
        5 | 10 | 12 | 16 | 17 | 18 => 8, // RATIONAL, SRATIONAL, DOUBLE, LONG8, SLONG8, IFD8
        _ => return None,
    })
}

/// One directory entry. `value` holds the value itself if it fits, otherwise
/// the offset of the value, both in file byte order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub tag: u16,
    typ: u16,
    pub count: u64,
    value: [u8; 8],
}

/// Image file directory: entries sorted by tag.
#[derive(Debug, Clone)]
pub(crate) struct Ifd {
    entries: Vec<Entry>,
}

impl Ifd {
    pub fn entry(&self, tag: u16) -> Option<&Entry> {
        self.entries.binary_search_by_key(&tag, |e| e.tag).ok().map(|i| &self.entries[i])
    }

    pub fn has(&self, tag: u16) -> bool {
        self.entry(tag).is_some()
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

/// An open TIFF file with its main IFD chain.
#[derive(Debug)]
pub(crate) struct TiffFile {
    pub file: File,
    pub little: bool,
    pub big: bool,
    pub len: u64,
    /// IFDs of the main chain, in file order.
    pub ifds: Vec<Ifd>,
}

impl TiffFile {
    /// Returns `true` if `head` starts with a TIFF or BigTIFF header.
    pub fn matches(head: &[u8]) -> bool {
        head.len() >= 4
            && matches!(&head[..4], [b'I', b'I', 42, 0] | [b'M', b'M', 0, 42] | [b'I', b'I', 43, 0] | [b'M', b'M', 0, 43])
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::from_file(File::open(path)?, MAX_IFDS)
    }

    /// Opens a file reading only its first IFD, for format detection.
    pub fn open_first(path: &Path) -> Result<Self> {
        Self::from_file(File::open(path)?, 1)
    }

    /// Parses the header and walks the IFD chain, reading at most
    /// `max_ifds` IFDs.
    pub fn from_file(file: File, max_ifds: usize) -> Result<Self> {
        let len = file.metadata()?.len();
        let mut head = [0u8; 16];
        read_exact_at(&file, &mut head[..8.min(len as usize)], 0)?;
        if !Self::matches(&head) {
            return Err(Error::Corrupt("not a TIFF file".into()));
        }
        let little = head[0] == b'I';
        let big = head[2] == 43 || head[3] == 43;
        let mut tiff = Self { file, little, big, len, ifds: Vec::new() };
        let first = if big {
            read_exact_at(&tiff.file, &mut head, 0)?;
            if tiff.u16(&head[4..]) != 8 {
                return Err(Error::UnsupportedVariant("BigTIFF with offset size other than 8".into()));
            }
            tiff.u64(&head[8..])
        } else {
            tiff.u32(&head[4..]) as u64
        };
        tiff.ifds = tiff.read_chain(first, max_ifds)?;
        if tiff.ifds.is_empty() {
            return Err(Error::Corrupt("TIFF without image directory".into()));
        }
        Ok(tiff)
    }

    /// Reads the IFD chain starting at `offset`, stopping at loops.
    pub fn read_chain(&self, mut offset: u64, max_ifds: usize) -> Result<Vec<Ifd>> {
        let mut ifds = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while offset != 0 && seen.insert(offset) && ifds.len() < max_ifds.min(MAX_IFDS) {
            let (ifd, next) = match self.read_ifd(offset) {
                Ok(v) => v,
                // A broken link at the end of the chain is common; keep what was read.
                Err(_) if !ifds.is_empty() => break,
                Err(e) => return Err(e),
            };
            ifds.push(ifd);
            offset = next;
        }
        Ok(ifds)
    }

    /// Reads one IFD, returning it and the offset of the next one.
    pub fn read_ifd(&self, offset: u64) -> Result<(Ifd, u64)> {
        let (count_size, entry_size, link_size): (usize, usize, usize) = if self.big { (8, 20, 8) } else { (2, 12, 4) };
        let mut buf = [0u8; 8];
        read_exact_at(&self.file, &mut buf[..count_size], offset)?;
        let n = if self.big { self.u64(&buf) } else { self.u16(&buf) as u64 };
        if n == 0 || offset + (count_size + link_size) as u64 + n * entry_size as u64 > self.len {
            return Err(Error::Corrupt(format!("invalid IFD at offset {offset}")));
        }
        let mut raw = vec![0u8; n as usize * entry_size + link_size];
        read_exact_at(&self.file, &mut raw, offset + count_size as u64)?;
        let mut entries: Vec<Entry> = raw[..n as usize * entry_size]
            .chunks_exact(entry_size)
            .map(|e| {
                let mut value = [0u8; 8];
                let (count, v) = if self.big {
                    (self.u64(&e[4..]), &e[12..20])
                } else {
                    (self.u32(&e[4..]) as u64, &e[8..12])
                };
                value[..v.len()].copy_from_slice(v);
                Entry { tag: self.u16(e), typ: self.u16(&e[2..]), count, value }
            })
            .collect();
        entries.sort_by_key(|e| e.tag);
        entries.dedup_by_key(|e| e.tag);
        let link = &raw[n as usize * entry_size..];
        let next = if self.big { self.u64(link) } else { self.u32(link) as u64 };
        Ok((Ifd { entries }, next))
    }

    /// Raw bytes of an entry's value, in file byte order.
    pub fn raw(&self, e: &Entry) -> Result<Vec<u8>> {
        let size = type_size(e.typ).unwrap_or(1).checked_mul(e.count).filter(|&s| s <= self.len);
        let size = size.ok_or_else(|| Error::Corrupt(format!("tag {} too large", e.tag)))? as usize;
        let inline = if self.big { 8 } else { 4 };
        if size <= inline {
            return Ok(e.value[..size].to_vec());
        }
        let offset = if self.big { self.u64(&e.value) } else { self.u32(&e.value) as u64 };
        let mut buf = vec![0u8; size];
        read_exact_at(&self.file, &mut buf, offset)?;
        Ok(buf)
    }

    /// Integer values of a tag of any integer type.
    pub fn uints(&self, ifd: &Ifd, tag: u16) -> Result<Option<Vec<u64>>> {
        ifd.entry(tag).map(|e| self.entry_uints(e)).transpose()
    }

    /// Integer values of an entry of any integer type.
    pub fn entry_uints(&self, e: &Entry) -> Result<Vec<u64>> {
        let raw = self.raw(e)?;
        Ok(match e.typ {
            1 | 6 | 7 => raw.iter().map(|&b| b as u64).collect(),
            3 | 8 => raw.chunks_exact(2).map(|b| self.u16(b) as u64).collect(),
            4 | 9 | 13 => raw.chunks_exact(4).map(|b| self.u32(b) as u64).collect(),
            16 | 17 | 18 => raw.chunks_exact(8).map(|b| self.u64(b)).collect(),
            t => return Err(Error::Corrupt(format!("tag {} has non-integer type {t}", e.tag))),
        })
    }

    /// First integer value of a tag.
    pub fn uint(&self, ifd: &Ifd, tag: u16) -> Result<Option<u64>> {
        let Some(e) = ifd.entry(tag) else { return Ok(None) };
        if e.count == 1 && type_size(e.typ).is_some_and(|s| s <= 8) {
            let v = &e.value;
            return Ok(Some(match e.typ {
                1 | 6 | 7 => v[0] as u64,
                3 | 8 => self.u16(v) as u64,
                4 | 9 | 13 => self.u32(v) as u64,
                16 | 17 | 18 if self.big => self.u64(v),
                _ => return Ok(self.uints(ifd, tag)?.and_then(|v| v.first().copied())),
            }));
        }
        Ok(self.uints(ifd, tag)?.and_then(|v| v.first().copied()))
    }

    /// Floating-point values of a rational, float or integer tag.
    pub fn floats(&self, ifd: &Ifd, tag: u16) -> Result<Option<Vec<f64>>> {
        let Some(e) = ifd.entry(tag) else { return Ok(None) };
        let raw = self.raw(e)?;
        let v = match e.typ {
            5 => raw.chunks_exact(8).map(|b| self.u32(b) as f64 / self.u32(&b[4..]) as f64).collect(),
            10 => raw
                .chunks_exact(8)
                .map(|b| self.u32(b) as i32 as f64 / self.u32(&b[4..]) as i32 as f64)
                .collect(),
            11 => raw.chunks_exact(4).map(|b| f32::from_bits(self.u32(b)) as f64).collect(),
            12 => raw.chunks_exact(8).map(|b| f64::from_bits(self.u64(b))).collect(),
            _ => return Ok(self.uints(ifd, tag)?.map(|v| v.into_iter().map(|x| x as f64).collect())),
        };
        Ok(Some(v))
    }

    /// ASCII tag value up to the first NUL, invalid UTF-8 replaced.
    pub fn string(&self, ifd: &Ifd, tag: u16) -> Result<Option<String>> {
        let Some(e) = ifd.entry(tag) else { return Ok(None) };
        let raw = self.raw(e)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        Ok(Some(String::from_utf8_lossy(&raw[..end]).into_owned()))
    }

    /// Offsets of the sub-IFDs of `ifd` (tag 330).
    pub fn sub_ifds(&self, ifd: &Ifd) -> Result<Vec<Ifd>> {
        let Some(offsets) = self.uints(ifd, tag::SUB_IFDS)? else { return Ok(Vec::new()) };
        offsets.into_iter().map(|o| self.read_ifd(o).map(|(ifd, _)| ifd)).collect()
    }

    pub fn u16(&self, b: &[u8]) -> u16 {
        let b = [b[0], b[1]];
        if self.little { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) }
    }

    pub fn u32(&self, b: &[u8]) -> u32 {
        let b = [b[0], b[1], b[2], b[3]];
        if self.little { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
    }

    pub fn u64(&self, b: &[u8]) -> u64 {
        let b: [u8; 8] = b[..8].try_into().expect("8 bytes");
        if self.little { u64::from_le_bytes(b) } else { u64::from_be_bytes(b) }
    }
}

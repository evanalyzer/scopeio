//! # ets
//!
//! ETS tile files of Olympus/Evident VSI datasets: SIS and ETS headers, the
//! tile table and tile decoding (raw, JPEG, JPEG 2000).
//!
//! Provenance: OpenReadout VSI format notes and bioio-vsi format notes
//! (`doc/olympus-vsi/`); own analysis of sample files. No GPL code
//! consulted.
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
use crate::{Error, PixelType, Result, scratch};

const SIS_MAGIC: &[u8; 4] = b"SIS\0";
const ETS_MAGIC: &[u8; 4] = b"ETS\0";
/// Upper bounds against corrupt headers.
const MAX_TILES: u64 = 1 << 26;
const MAX_TILE_BYTES: u64 = 1 << 28;

/// Tile compression, ETS header field at offset 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compression {
    Raw,
    Jpeg,
    Jpeg2000,
    Other(u32),
}

/// Tiles of one pyramid level, densely indexed by plane, row and column.
#[derive(Debug)]
pub(super) struct LevelTiles {
    pub col0: i64,
    pub row0: i64,
    pub cols: u64,
    pub rows: u64,
    offsets: Box<[u64]>,
    lens: Box<[u32]>,
}

/// An opened ETS file.
#[derive(Debug)]
pub(super) struct Ets {
    file: File,
    len: u64,
    pub pixel_type: PixelType,
    pub spp: u8,
    compression: Compression,
    pub tile_w: u32,
    pub tile_h: u32,
    /// One native pixel used for tiles that are not stored.
    background: [u8; 8],
    /// X, Y and the sizes of the further coordinate slots; may be empty.
    pub sizes: Vec<u32>,
    /// Size of each coordinate slot between y and the level, from the tiles.
    pub slots: Vec<u32>,
    /// Tile grid offset stored in the ETS header (x, y).
    pub origin: (i64, i64),
    pub levels: Vec<LevelTiles>,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

impl Ets {
    /// Opens an ETS file and reads its tile table.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers or tables and
    /// [`Error::UnsupportedVariant`] for unknown sample types.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut sis = [0u8; 64];
        if len < 64 {
            return Err(Error::Corrupt(format!("{}: too short for an ETS file", path.display())));
        }
        read_exact_at(&file, &mut sis, 0)?;
        if &sis[..4] != SIS_MAGIC {
            return Err(Error::Corrupt(format!("{}: not an ETS file", path.display())));
        }
        let coord_count = u32_at(&sis, 12) as usize;
        let ets_offset = u64_at(&sis, 16);
        let table = u64_at(&sis, 32);
        let count = u32_at(&sis, 40) as u64;
        if !(3..=16).contains(&coord_count) || count > MAX_TILES {
            return Err(Error::Corrupt(format!("{}: invalid SIS header", path.display())));
        }
        let mut ets = [0u8; 228];
        if ets_offset + 228 > len {
            return Err(Error::Corrupt(format!("{}: ETS header beyond the end", path.display())));
        }
        read_exact_at(&file, &mut ets, ets_offset)?;
        if &ets[..4] != ETS_MAGIC {
            return Err(Error::Corrupt(format!("{}: missing ETS header", path.display())));
        }
        let pixel_type = match u32_at(&ets, 8) {
            2 => PixelType::U8,
            4 => PixelType::U16,
            t => return Err(Error::UnsupportedVariant(format!("ETS sample type {t}"))),
        };
        let spp = u32_at(&ets, 12).clamp(1, 4) as u8;
        let compression = match u32_at(&ets, 20) {
            0 => Compression::Raw,
            2 => Compression::Jpeg,
            3 => Compression::Jpeg2000,
            c => Compression::Other(c),
        };
        let (tile_w, tile_h) = (u32_at(&ets, 28), u32_at(&ets, 32));
        if tile_w == 0 || tile_h == 0 || (tile_w as u64 * tile_h as u64 * spp as u64 * 2) > MAX_TILE_BYTES {
            return Err(Error::Corrupt(format!("{}: tile size {tile_w}x{tile_h}", path.display())));
        }
        let bg = u32_at(&ets, 108).to_le_bytes();
        let mut background = [0u8; 8];
        match pixel_type {
            PixelType::U8 => background[..spp as usize].copy_from_slice(&bg[..spp as usize]),
            _ => {
                let v = (u32_at(&ets, 108) as u16).to_ne_bytes();
                for s in 0..spp as usize {
                    background[2 * s..2 * s + 2].copy_from_slice(&v);
                }
            }
        }
        let nsizes = (u32_at(&ets, 184) as usize).min(9);
        let sizes = (0..nsizes).map(|i| u32_at(&ets, 188 + 4 * i)).collect();
        let origin = (i32_at(&ets, 172) as i64, i32_at(&ets, 176) as i64);

        // Tile table: count, coordinates (col, row, slots.., level), offset,
        // length, an unknown word.
        let entry = 4 + 4 * coord_count + 16;
        let table_len = count * entry as u64;
        if table.checked_add(table_len).is_none_or(|end| end > len) {
            return Err(Error::Corrupt(format!("{}: tile table beyond the end", path.display())));
        }
        let nslots = coord_count - 3;
        let tiles = scratch::with(table_len as usize, |raw| -> Result<Vec<([i64; 16], u64, u32)>> {
            read_exact_at(&file, raw, table)?;
            Ok(raw
                .chunks_exact(entry)
                .map(|e| {
                    let mut coords = [0i64; 16];
                    for (i, c) in coords.iter_mut().enumerate().take(coord_count) {
                        *c = i32_at(e, 4 + 4 * i) as i64;
                    }
                    (coords, u64_at(e, 4 + 4 * coord_count), u32_at(e, 12 + 4 * coord_count))
                })
                .collect())
        })?;
        let mut slots = vec![1u32; nslots];
        let mut nlevels = 0usize;
        for (c, _, _) in &tiles {
            for (s, size) in slots.iter_mut().enumerate() {
                *size = (*size).max(c[2 + s].max(0) as u32 + 1);
            }
            nlevels = nlevels.max(c[coord_count - 1].max(0) as usize + 1);
        }
        let planes: u64 = slots.iter().map(|&s| s as u64).product();
        let mut levels = Vec::with_capacity(nlevels);
        for level in 0..nlevels {
            let on_level = || tiles.iter().filter(|(c, _, _)| c[coord_count - 1] == level as i64);
            let (mut c0, mut c1, mut r0, mut r1) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
            for (c, _, _) in on_level() {
                c0 = c0.min(c[0]);
                c1 = c1.max(c[0]);
                r0 = r0.min(c[1]);
                r1 = r1.max(c[1]);
            }
            let (cols, rows) = if c0 > c1 { (0, 0) } else { ((c1 - c0 + 1) as u64, (r1 - r0 + 1) as u64) };
            let n = planes.checked_mul(cols).and_then(|v| v.checked_mul(rows)).filter(|&n| n <= MAX_TILES * 4);
            let n = n.ok_or_else(|| Error::Corrupt(format!("{}: tile grid too large", path.display())))? as usize;
            let mut offsets = vec![0u64; n];
            let mut lens = vec![0u32; n];
            for (c, offset, l) in on_level() {
                let plane = plane_index(&c[2..2 + nslots], &slots);
                let i = ((plane * rows + (c[1] - r0) as u64) * cols + (c[0] - c0) as u64) as usize;
                offsets[i] = *offset;
                lens[i] = *l;
            }
            levels.push(LevelTiles {
                col0: if cols == 0 { 0 } else { c0 },
                row0: if rows == 0 { 0 } else { r0 },
                cols,
                rows,
                offsets: offsets.into(),
                lens: lens.into(),
            });
        }
        Ok(Self { file, len, pixel_type, spp, compression, tile_w, tile_h, background, sizes, slots, origin, levels })
    }

    /// Bytes of one decoded tile.
    pub fn tile_bytes(&self) -> usize {
        self.tile_w as usize * self.tile_h as usize * self.spp as usize * self.pixel_type.bytes()
    }

    /// One native background pixel.
    pub fn background(&self) -> &[u8] {
        &self.background[..self.spp as usize * self.pixel_type.bytes()]
    }

    /// Decodes tile (`col`, `row`) of `plane` at `level` into `out`
    /// (`tile_bytes()` bytes, row-major, native samples).
    ///
    /// # Returns
    ///
    /// `false` if the tile is not stored.
    pub fn read_tile(&self, level: usize, plane: u64, col: i64, row: i64, out: &mut [u8]) -> Result<bool> {
        let Some(l) = self.levels.get(level) else { return Ok(false) };
        let (c, r) = (col - l.col0, row - l.row0);
        if c < 0 || r < 0 || c as u64 >= l.cols || r as u64 >= l.rows {
            return Ok(false);
        }
        let i = ((plane * l.rows + r as u64) * l.cols + c as u64) as usize;
        let (Some(&offset), Some(&len)) = (l.offsets.get(i), l.lens.get(i)) else { return Ok(false) };
        if len == 0 {
            return Ok(false);
        }
        if offset.checked_add(len as u64).is_none_or(|end| end > self.len) {
            return Err(Error::Corrupt("ETS tile beyond the end of the file".into()));
        }
        scratch::with(len as usize, |data| {
            read_exact_at(&self.file, data, offset)?;
            self.decode(data, out)
        })?;
        Ok(true)
    }

    fn decode(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        match self.compression {
            Compression::Raw => {
                let n = data.len().min(out.len());
                out[..n].copy_from_slice(&data[..n]);
                out[n..].fill(0);
                // Raw samples are little-endian.
                if cfg!(target_endian = "big") && self.pixel_type.bytes() > 1 {
                    out.chunks_exact_mut(self.pixel_type.bytes()).for_each(<[u8]>::reverse);
                }
                Ok(())
            }
            Compression::Jpeg => self.decode_jpeg(data, out),
            Compression::Jpeg2000 => self.decode_jpeg2000(data, out),
            Compression::Other(5) => Err(Error::UnsupportedVariant("ETS lossless JPEG tiles".into())),
            Compression::Other(c) => Err(Error::UnsupportedVariant(format!("ETS compression {c}"))),
        }
    }

    fn decode_jpeg(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        #[cfg(feature = "jpeg")]
        {
            use crate::codecs::jpeg::{Color, decode};
            if self.pixel_type != PixelType::U8 {
                return Err(Error::UnsupportedVariant("16-bit JPEG tiles".into()));
            }
            // Decoded JPEGs may be padded to whole MCUs (at most 16 pixels).
            let padded = (self.tile_w as usize + 16) * (self.tile_h as usize + 16) * self.spp as usize;
            let info = scratch::with(padded, |buf| -> Result<_> {
                let info = decode(data, None, Color::YcbcrToRgb, buf)?;
                let row = self.tile_w as usize * self.spp as usize;
                let src_row = info.width * info.components;
                for (y, dst) in out.chunks_exact_mut(row).enumerate() {
                    if y < info.height && info.components == self.spp as usize {
                        let n = row.min(src_row);
                        dst[..n].copy_from_slice(&buf[y * src_row..y * src_row + n]);
                        dst[n..].fill(0);
                    } else {
                        dst.fill(0);
                    }
                }
                Ok(info)
            })?;
            if info.components != self.spp as usize {
                return Err(Error::Corrupt(format!("JPEG tile with {} components, expected {}", info.components, self.spp)));
            }
            Ok(())
        }
        #[cfg(not(feature = "jpeg"))]
        {
            let _ = (data, out);
            Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
        }
    }

    fn decode_jpeg2000(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        #[cfg(feature = "jpeg2000")]
        {
            let info = crate::codecs::jpeg2000::decode(data, self.pixel_type, out)?;
            if info.components != self.spp as usize || info.width != self.tile_w as usize {
                // Smaller codestreams fill the tile from the top left.
                return Err(Error::Corrupt(format!(
                    "JPEG 2000 tile {}x{}x{} does not match {}x{}x{}",
                    info.width, info.height, info.components, self.tile_w, self.tile_h, self.spp
                )));
            }
            Ok(())
        }
        #[cfg(not(feature = "jpeg2000"))]
        {
            let _ = (data, out);
            Err(Error::UnsupportedVariant("jpeg2000 compression (enable the `jpeg2000` feature)".into()))
        }
    }
}

/// Linear plane index of slot values, slot 0 fastest.
pub(super) fn plane_index(values: &[i64], sizes: &[u32]) -> u64 {
    let mut index = 0u64;
    for (v, s) in values.iter().zip(sizes).rev() {
        index = index * *s as u64 + (*v).max(0) as u64;
    }
    index
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_index_slot_zero_fastest() {
        assert_eq!(plane_index(&[1, 2], &[3, 4]), 1 + 3 * 2);
        assert_eq!(plane_index(&[], &[]), 0);
    }
}

//! # mrc
//!
//! MRC / CCP4 map reader (`.mrc`, `.mrcs`, `.st`, `.ali`, `.rec`, `.map`):
//! the electron microscopy volume and image stack format.
//!
//! A 1024-byte header (sizes, pixel mode, cell dimensions in Ångström,
//! the `MAP ` marker at byte 208 and the machine stamp at byte 212 giving
//! the byte order, ten 80-byte labels), an optional extended header of
//! `nsymbt` bytes, then the sections, each one `nx x ny` plane stored
//! bottom row first (returned top row first). A volume
//! stack (space group 401) holds `nz / mz` volumes of `mz` sections; they
//! are returned as time points. Pixel sizes are the cell lengths divided
//! by the sampling (`mx`, `my`, `mz`). Half floats (mode 12) are returned
//! as F32 and 4-bit samples (mode 101) as U8.
//!
//! Provenance: MRC2014 format specification (Cheng et al., J. Struct.
//! Biol. 192 (2015), and the CCP-EM format page); IMOD MRC header
//! documentation (prose, for the IMOD stamp and mode 101); own analysis of
//! sample files; checked against Bio-Formats as a black box. No GPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{FormatId, FormatInfo, Error, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Mrc, "MRC", "1").extensions(EXTENSIONS);

const HEADER_LEN: u64 = 1024;
/// Stamp of IMOD-written headers at byte 152.
const IMOD_STAMP: i32 = 1_146_047_817;
/// Extensions accepted for old headers without the `MAP ` marker.
const EXTENSIONS: &[&str] = &["mrc", "mrcs", "st", "ali", "rec", "map", "preali"];

/// Stored sample encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Stored as the output type.
    Plain,
    /// IEEE half float, returned as F32.
    Half,
    /// Two 4-bit samples per byte, low nibble first, returned as U8.
    Nibble,
}

/// Returns `true` if `head` is an MRC header: the `MAP ` marker, or for
/// old headers an MRC extension and plausible sizes.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    if head.len() < 224 {
        return false;
    }
    let marker = &head[208..212] == b"MAP ";
    let ext = path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)));
    (marker || ext) && byte_order(head).is_some()
}

/// Byte order of the header: from the machine stamp, else the one with
/// plausible sizes and mode. `Some(true)` is little-endian.
fn byte_order(h: &[u8]) -> Option<bool> {
    match (h[212], h[213]) {
        (0x44, 0x44 | 0x41) => return plausible(h, true).then_some(true),
        (0x11, 0x11) => return plausible(h, false).then_some(false),
        _ => {}
    }
    [true, false].into_iter().find(|&little| plausible(h, little))
}

fn plausible(h: &[u8], little: bool) -> bool {
    let at = |o: usize| i32_at(h, o, little);
    (0..3).all(|i| at(4 * i) > 0 && at(4 * i) < 1 << 24) && matches!(at(12), 0..=2 | 6 | 12 | 101)
}

fn i32_at(h: &[u8], o: usize, little: bool) -> i32 {
    let b: [u8; 4] = h[o..o + 4].try_into().expect("4 bytes");
    if little { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) }
}

/// MRC file.
#[derive(Debug)]
pub(super) struct Mrc {
    file: File,
    little: bool,
    mode: Mode,
    /// Stored bytes per sample (0 for 4-bit samples).
    bytes: usize,
    data: u64,
    row_bytes: u64,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
}

impl Mrc {
    /// Opens an MRC file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers and
    /// [`Error::UnsupportedVariant`] for complex pixel modes.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        if len < HEADER_LEN {
            return Err(Error::Corrupt("MRC file too short".into()));
        }
        let mut h = [0u8; HEADER_LEN as usize];
        read_exact_at(&file, &mut h, 0)?;
        let little = match (h[212], h[213]) {
            (0x44, 0x44 | 0x41) => true,
            (0x11, 0x11) => false,
            _ => byte_order(&h).ok_or_else(|| Error::Corrupt("not an MRC file".into()))?,
        };
        let i32_at = |o: usize| i32_at(&h, o, little);
        let f32_at = |o: usize| f32::from_bits(i32_at(o) as u32) as f64;
        let (nx, ny, nz) = (i32_at(0), i32_at(4), i32_at(8));
        if nx <= 0 || ny <= 0 || nz <= 0 {
            return Err(Error::Corrupt(format!("MRC sizes {nx} x {ny} x {nz}")));
        }
        let version = i32_at(108);
        let imod = i32_at(152) == IMOD_STAMP;
        let (pixel_type, mode, bytes) = match i32_at(12) {
            // MRC2014 and IMOD's signed-byte flag make mode 0 signed.
            0 if version >= 20140 || (imod && i32_at(156) & 1 != 0) => (PixelType::I8, Mode::Plain, 1),
            0 => (PixelType::U8, Mode::Plain, 1),
            1 => (PixelType::I16, Mode::Plain, 2),
            2 => (PixelType::F32, Mode::Plain, 4),
            6 => (PixelType::U16, Mode::Plain, 2),
            12 => (PixelType::F32, Mode::Half, 2),
            101 => (PixelType::U8, Mode::Nibble, 0),
            m @ (3 | 4) => return Err(Error::UnsupportedVariant(format!("MRC complex pixel mode {m}"))),
            m => return Err(Error::UnsupportedVariant(format!("MRC pixel mode {m}"))),
        };
        let row_bytes = if mode == Mode::Nibble { (nx as u64).div_ceil(2) } else { nx as u64 * bytes as u64 };
        let data = HEADER_LEN + i32_at(92).max(0) as u64;
        if data + row_bytes * ny as u64 * nz as u64 > len {
            return Err(Error::Corrupt("MRC file shorter than its sections".into()));
        }
        // Sampling along each axis; a volume stack has nz / mz volumes.
        let (mx, my, mz) = (i32_at(28), i32_at(32), i32_at(36));
        let (size_z, size_t) = if i32_at(88) == 401 && mz > 0 && nz % mz == 0 { (mz as u32, (nz / mz) as u32) } else { (nz as u32, 1) };
        let spacing = |cell: usize, n: i32, fallback: i32| {
            let n = if n > 0 { n } else { fallback };
            Some(f32_at(cell) / n as f64).filter(|v| v.is_finite() && *v > 0.0).map(|a| a * 0.1)
        };
        let meta = SeriesMeta {
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()),
            size_c: 1,
            size_z,
            size_t,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: nx as u64, size_y: ny as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize {
                x_nm: spacing(40, mx, nx),
                y_nm: spacing(44, my, ny),
                z_nm: spacing(48, mz, size_z as i32).filter(|_| size_z > 1),
            },
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        let labels = i32_at(220).clamp(0, 10) as usize;
        for i in 0..labels {
            let l = String::from_utf8_lossy(&h[224 + 80 * i..304 + 80 * i]).trim_end_matches(['\0', ' ']).to_string();
            if !l.is_empty() {
                original.pairs.push((format!("Label {}", i + 1), l));
            }
        }
        for (key, o) in [("Minimum", 76), ("Maximum", 80), ("Mean", 84), ("RMS", 216)] {
            original.pairs.push((key.into(), f32_at(o).to_string()));
        }
        original.pairs.push(("Space group".into(), i32_at(88).to_string()));
        original.pairs.push(("Axis order".into(), format!("{} {} {}", i32_at(64), i32_at(68), i32_at(72))));
        if version > 0 {
            original.pairs.push(("Version".into(), version.to_string()));
        }
        let ext_type = String::from_utf8_lossy(&h[104..108]).trim_end_matches(['\0', ' ']).to_string();
        if !ext_type.is_empty() {
            original.pairs.push(("Extended header type".into(), ext_type));
        }
        Ok(Mrc { file, little, mode, bytes, data, row_bytes, metas: [meta], original })
    }
}

/// Converts an IEEE half float to `f32`.
fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) as u32) << 31;
    let exp = ((h >> 10) & 0x1f) as u32;
    let man = (h & 0x3ff) as u32;
    match exp {
        0 if man == 0 => f32::from_bits(sign),
        // Subnormal: man * 2^-24.
        0 => {
            let v = man as f32 * f32::from_bits(0x3380_0000);
            if sign != 0 { -v } else { v }
        }
        31 => f32::from_bits(sign | 0x7f80_0000 | man << 13),
        _ => f32::from_bits(sign | (exp + 112) << 23 | man << 13),
    }
}

impl Backend for Mrc {
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
        let m = &self.metas[0];
        let height = m.levels[0].size_y;
        let section = (plane.z + m.size_z * plane.t) as u64;
        let base = self.data + section * height * self.row_bytes;
        let w = region.width as usize;
        let (first, stored) = match self.mode {
            // Whole bytes covering samples x .. x + w.
            Mode::Nibble => (region.x / 2, ((region.x + w as u64).div_ceil(2) - region.x / 2) as usize),
            _ => (region.x * self.bytes as u64, w * self.bytes),
        };
        let swap = self.bytes > 1 && self.little != cfg!(target_endian = "little");
        scratch::with(stored, |row| {
            for r in 0..region.height as usize {
                // Rows are stored bottom-up (origin at the lower left).
                let y = height - 1 - (region.y + r as u64);
                read_exact_at(&self.file, row, base + y * self.row_bytes + first)?;
                sink.write_with(r, 0, w, |d| {
                    match self.mode {
                        Mode::Plain => {
                            d.copy_from_slice(row);
                            if swap {
                                for s in d.chunks_exact_mut(self.bytes) {
                                    s.reverse();
                                }
                            }
                        }
                        Mode::Half => {
                            for (o, s) in d.chunks_exact_mut(4).zip(row.chunks_exact(2)) {
                                let b = [s[0], s[1]];
                                let v = if self.little { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) };
                                o.copy_from_slice(&half_to_f32(v).to_ne_bytes());
                            }
                        }
                        Mode::Nibble => {
                            let skip = (region.x % 2) as usize;
                            for (i, o) in d.iter_mut().enumerate() {
                                let b = row[(i + skip) / 2];
                                *o = if (i + skip).is_multiple_of(2) { b & 0x0f } else { b >> 4 };
                            }
                        }
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn header(little: bool, mode: i32) -> Vec<u8> {
        let mut h = vec![0u8; 1024];
        for (i, v) in [4i32, 3, 2, mode].iter().enumerate() {
            h[4 * i..4 * i + 4].copy_from_slice(&if little { v.to_le_bytes() } else { v.to_be_bytes() });
        }
        h[208..212].copy_from_slice(b"MAP ");
        h[212..214].copy_from_slice(if little { &[0x44, 0x44] } else { &[0x11, 0x11] });
        h
    }

    #[test]
    fn detects_both_byte_orders() {
        let p = Path::new("x.bin");
        assert!(matches(p, &header(true, 2)));
        assert!(matches(p, &header(false, 6)));
        assert!(!matches(p, &header(true, 3)));
        let mut old = header(true, 1);
        old[208..214].fill(0);
        assert!(!matches(p, &old));
        assert!(matches(Path::new("x.mrc"), &old));
    }

    #[test]
    fn converts_half_floats() {
        assert_eq!(half_to_f32(0x3c00), 1.0);
        assert_eq!(half_to_f32(0xc000), -2.0);
        assert_eq!(half_to_f32(0x0001), 2f32.powi(-24));
        assert!(half_to_f32(0x7e00).is_nan());
        assert_eq!(half_to_f32(0x7bff), 65504.0);
    }
}

//! # dv
//!
//! DeltaVision (`.dv`, `.r3d`) reader: softWoRx / Priism files.
//!
//! A 1024-byte MRC-like header (byte order from the identifier -16224 at
//! byte 96), an extended header with one record per section (integers,
//! then floats: time stamp, stage position, exposure, wavelengths), then
//! the sections, each one `width x height` plane stored bottom row first
//! (returned top row first). The section order is
//! Z-T-wavelength, wavelength-Z-T or Z-wavelength-T. Channels are the
//! wavelengths; pixel sizes come from the header spacing (micrometres),
//! channel wavelengths and exposure from the extended header of each
//! channel's first section.
//!
//! Provenance: Priism IVE image file format reference (UCSF, archived in
//! `doc/deltavision-dv/`); OpenReadout DeltaVision notes (MIT OR
//! Apache-2.0); own analysis of the OME sample files (CC BY 4.0); checked
//! against Bio-Formats as a black box. No GPL code consulted.
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
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, PlaneMeta,
    Position, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::DeltaVision, "DeltaVision", "1");

const HEADER_LEN: u64 = 1024;
const DV_ID: i16 = -16224;
/// Largest extended header read.
const MAX_EXT: u64 = 1 << 28;
/// Floats of a section record that are interpreted.
const SECTION_FLOATS: usize = 14;

/// Returns `true` if `head` holds the DeltaVision identifier at byte 96.
pub(super) fn matches(head: &[u8]) -> bool {
    if head.len() < 98 {
        return false;
    }
    let little = i16::from_le_bytes([head[96], head[97]]) == DV_ID;
    if !little && i16::from_be_bytes([head[96], head[97]]) != DV_ID {
        return false;
    }
    // A plausible header: positive sizes and a known pixel mode.
    let i32_at = |o: usize| {
        let b: [u8; 4] = head[o..o + 4].try_into().expect("4 bytes");
        if little { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) }
    };
    (0..3).all(|i| i32_at(4 * i) > 0) && matches!(i32_at(12), 0..=2 | 4..=7)
}

/// Section order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// Z fastest, then T, then wavelength.
    Ztw,
    /// Wavelength fastest, then Z, then T.
    Wzt,
    /// Z fastest, then wavelength, then T.
    Zwt,
}

/// DeltaVision file.
#[derive(Debug)]
pub(super) struct Dv {
    file: File,
    little: bool,
    data: u64,
    plane_bytes: u64,
    bytes: usize,
    order: Order,
    /// Time stamp (s) of each section.
    times: Vec<Option<f64>>,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
}

impl Dv {
    /// Opens a DeltaVision file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers and
    /// [`Error::UnsupportedVariant`] for complex pixel modes.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut h = [0u8; HEADER_LEN as usize];
        if len < HEADER_LEN {
            return Err(Error::Corrupt("DeltaVision file too short".into()));
        }
        read_exact_at(&file, &mut h, 0)?;
        let little = i16::from_le_bytes([h[96], h[97]]) == DV_ID;
        if !little && i16::from_be_bytes([h[96], h[97]]) != DV_ID {
            return Err(Error::Corrupt("not a DeltaVision file".into()));
        }
        let i32_at = |o: usize| {
            let b: [u8; 4] = h[o..o + 4].try_into().expect("4 bytes");
            if little { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) }
        };
        let i16_at = |o: usize| {
            let b = [h[o], h[o + 1]];
            if little { i16::from_le_bytes(b) } else { i16::from_be_bytes(b) }
        };
        let f32_at = |o: usize| f32::from_bits(i32_at(o) as u32) as f64;
        let (width, height, sections) = (i32_at(0), i32_at(4), i32_at(8));
        if width <= 0 || height <= 0 || sections <= 0 {
            return Err(Error::Corrupt(format!("DeltaVision sizes {width} x {height} x {sections}")));
        }
        let (pixel_type, bytes) = match i32_at(12) {
            0 => (PixelType::U8, 1),
            1 | 5 => (PixelType::I16, 2),
            2 => (PixelType::F32, 4),
            6 => (PixelType::U16, 2),
            7 => (PixelType::I32, 4),
            m => return Err(Error::UnsupportedVariant(format!("DeltaVision pixel mode {m}"))),
        };
        let ext_len = i32_at(92).max(0) as u64;
        let (ext_ints, ext_floats) = (i16_at(128).max(0) as usize, i16_at(130).max(0) as usize);
        let size_c = i16_at(196).clamp(1, 5) as u32;
        let size_t = i16_at(180).max(1) as u32;
        let order = match i16_at(182) {
            0 => Order::Ztw,
            1 => Order::Wzt,
            2 => Order::Zwt,
            s => return Err(Error::UnsupportedVariant(format!("DeltaVision section order {s}"))),
        };
        let per_z = size_c as i64 * size_t as i64;
        if sections as i64 % per_z != 0 {
            return Err(Error::Corrupt("DeltaVision sections are not whole Z stacks".into()));
        }
        let size_z = (sections as i64 / per_z) as u32;
        let plane_bytes = width as u64 * height as u64 * bytes as u64;
        let data = HEADER_LEN + ext_len.min(MAX_EXT);
        if data + plane_bytes * sections as u64 > len {
            return Err(Error::Corrupt("DeltaVision file shorter than its sections".into()));
        }
        // Extended header records: ext_ints i32 then ext_floats f32 per section.
        let record = 4 * (ext_ints + ext_floats);
        let floats: Vec<Vec<f64>> = if record > 0 && ext_floats >= SECTION_FLOATS && (record as u64) * (sections as u64) <= ext_len.min(MAX_EXT) {
            let mut ext = vec![0u8; record * sections as usize];
            read_exact_at(&file, &mut ext, HEADER_LEN)?;
            ext.chunks_exact(record)
                .map(|r| {
                    r[4 * ext_ints..4 * ext_ints + 4 * SECTION_FLOATS]
                        .chunks_exact(4)
                        .map(|b| {
                            let b: [u8; 4] = b.try_into().expect("4 bytes");
                            (if little { f32::from_le_bytes(b) } else { f32::from_be_bytes(b) }) as f64
                        })
                        .collect()
                })
                .collect()
        } else {
            Vec::new()
        };
        let section = |c: u32, z: u32, t: u32| -> usize {
            (match order {
                Order::Ztw => z + size_z * (t + size_t * c),
                Order::Wzt => c + size_c * (z + size_z * t),
                Order::Zwt => z + size_z * (c + size_c * t),
            }) as usize
        };
        let times: Vec<Option<f64>> = (0..sections as usize).map(|s| floats.get(s).map(|f| f[1]).filter(|t| t.is_finite() && *t >= 0.0 && *t < 1e8)).collect();
        let channels = (0..size_c)
            .map(|c| {
                let wavelength = i16_at(198 + 2 * c as usize) as f64;
                let f = floats.get(section(c, 0, 0));
                ChannelMeta {
                    name: (wavelength != 0.0).then(|| format!("{wavelength}")),
                    exposure_s: f.map(|f| f[8]).filter(|v| *v > 0.0 && *v < 1e5),
                    excitation_wavelength_nm: f.map(|f| f[10]).filter(|v| *v > 0.0).or((wavelength > 0.0).then_some(wavelength)),
                    emission_wavelength_nm: f.map(|f| f[11]).filter(|v| *v > 0.0),
                    ..Default::default()
                }
            })
            .collect();
        // Time increment: median step of the first channel and Z.
        let mut steps: Vec<f64> = (1..size_t).filter_map(|t| Some(times[section(0, 0, t)]? - times[section(0, 0, t - 1)]?)).filter(|d| *d > 0.0).collect();
        steps.sort_by(f64::total_cmp);
        let spacing = |o: usize| Some(f32_at(o)).filter(|v| v.is_finite() && *v > 0.0).map(|v| v * 1e3);
        let titles: Vec<String> = (0..10)
            .map(|i| String::from_utf8_lossy(&h[224 + 80 * i..304 + 80 * i]).trim_end_matches(['\0', ' ']).to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let stage = floats.first().map(|f| Position { x_nm: Some(f[2] * 1e3), y_nm: Some(f[3] * 1e3), z_nm: Some(f[4] * 1e3) });
        let meta = SeriesMeta {
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()),
            size_c,
            size_z,
            size_t,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: spacing(40), y_nm: spacing(44), z_nm: spacing(48).filter(|_| size_z > 1) },
            time_increment_s: steps.get(steps.len() / 2).copied().filter(|_| size_t > 1),
            channels,
            stage_position: stage,
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        for (i, t) in titles.iter().enumerate() {
            original.pairs.push((format!("Title {}", i + 1), t.clone()));
        }
        original.pairs.push(("Lens ID".into(), i16_at(162).to_string()));
        original.pairs.push(("Image type".into(), i16_at(160).to_string()));
        Ok(Dv { file, little, data, plane_bytes, bytes, order, times, metas: [meta], original })
    }

    fn section(&self, plane: Plane) -> u64 {
        let m = &self.metas[0];
        let (c, z, t) = (plane.c, plane.z, plane.t);
        (match self.order {
            Order::Ztw => z + m.size_z * (t + m.size_t * c),
            Order::Wzt => c + m.size_c * (z + m.size_z * t),
            Order::Zwt => z + m.size_z * (c + m.size_c * t),
        }) as u64
    }
}

impl Backend for Dv {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        let first = self.times.first().copied().flatten();
        let t = self.times.get(self.section(plane) as usize).copied().flatten();
        Ok(PlaneMeta { delta_t_s: t.zip(first).map(|(t, f)| t - f), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let (width, height) = (self.metas[0].levels[0].size_x, self.metas[0].levels[0].size_y);
        let base = self.data + self.section(plane) * self.plane_bytes;
        let n = self.bytes;
        let w = region.width as usize;
        scratch::with(w * n, |row| {
            for r in 0..region.height as usize {
                // Rows are stored bottom-up (origin at the lower left).
                let at = base + ((height - 1 - (region.y + r as u64)) * width + region.x) * n as u64;
                read_exact_at(&self.file, row, at)?;
                // Samples as little- or big-endian, returned in native order.
                if n > 1 && self.little != cfg!(target_endian = "little") {
                    for s in row.chunks_exact_mut(n) {
                        s.reverse();
                    }
                }
                sink.write_with(r, 0, w, |d| {
                    d.copy_from_slice(row);
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

    #[test]
    fn detects_both_byte_orders() {
        let mut h = [0u8; 98];
        for (i, v) in [4i32, 3, 2, 6].iter().enumerate() {
            h[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
        }
        h[96..98].copy_from_slice(&DV_ID.to_le_bytes());
        assert!(matches(&h));
        for (i, v) in [4i32, 3, 2, 6].iter().enumerate() {
            h[4 * i..4 * i + 4].copy_from_slice(&v.to_be_bytes());
        }
        h[96..98].copy_from_slice(&DV_ID.to_be_bytes());
        assert!(matches(&h));
        h[96] = 0;
        assert!(!matches(&h));
    }
}

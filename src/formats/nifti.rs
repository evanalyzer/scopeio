//! # nifti
//!
//! NIfTI-1 and NIfTI-2 reader: single files (`.nii`, magic `n+1` / `n+2`),
//! header and image pairs (`.hdr` + `.img`, magic `ni1` / `ni2`) and
//! gzip-compressed single files (`.nii.gz`, needs the `deflate` feature).
//! Also reads Mayo Analyze 7.5 pairs, whose 348-byte header NIfTI-1 extends.
//!
//! The header (348 bytes for NIfTI-1, 540 for NIfTI-2; byte order from
//! `sizeof_hdr`) gives the dimensions (`dim[1..7]`: x, y, z, t, then vector
//! components and further dimensions), the data type, the voxel sizes
//! (`pixdim`) and their units (`xyzt_units`), and the data offset
//! (`vox_offset`). Voxels are stored x fastest, then y, z, t, u; planes
//! are returned as stored (no reorientation by `qform` / `sform`). The
//! fifth dimension becomes channels, the sixth and seventh time points.
//! Stored values are returned unscaled; `scl_slope` / `scl_inter` are in
//! the original metadata. Compressed files are decompressed into memory
//! on the first read.
//!
//! Analyze 7.5 (`.hdr` + `.img`, no magic): a header of 348 bytes with
//! `dim`, `datatype` and `pixdim` where NIfTI-1 has them, recognised by
//! `sizeof_hdr`, plausible dimensions and a known data type. The voxels
//! start at `vox_offset` in the `.img` file; the spatial unit is the
//! `vox_units` text (none if empty or unknown); the SPM scale factor
//! (`funused1`) is in the original metadata. Voxels are returned as stored
//! (`orient` is not applied).
//!
//! Provenance: NIfTI-1 data format specification (`nifti1.h`, NIfTI Data
//! Format Working Group, public domain) and the NIfTI-2 header definition
//! (`nifti2.h`, public domain) in `doc/nifti/`; Mayo Analyze 7.5 format
//! document and G. Wideman's Analyze field compilation in `doc/analyze/`;
//! official NIfTI test data (public domain); checked against Bio-Formats as
//! a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Nifti, "NIfTI", "1").extensions(&["nii", "nii.gz", "hdr", "img"]);
pub(super) static ANALYZE_FORMAT: FormatInfo = FormatInfo::new(FormatId::Analyze, "Analyze 7.5", "1").extensions(&["hdr", "img"]);

const NIFTI1_LEN: usize = 348;
const NIFTI2_LEN: usize = 540;
/// Largest decompressed `.nii.gz` held in memory.
const MAX_DECOMPRESSED: u64 = 4 << 30;

/// Parsed header fields.
#[derive(Debug)]
struct Header {
    little: bool,
    /// `true` for a single file (`n+1` / `n+2`).
    single: bool,
    /// 1 or 2 for NIfTI, 0 for Analyze 7.5.
    version: u8,
    datatype: i16,
    dim: [i64; 8],
    pixdim: [f64; 8],
    vox_offset: u64,
    scl_slope: f64,
    scl_inter: f64,
    xyzt_units: u8,
    /// Analyze 7.5: nanometres per `vox_units`.
    vox_unit_nm: Option<f64>,
    descrip: String,
    intent_name: String,
    intent_code: i32,
    qform_code: i32,
    sform_code: i32,
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

impl Header {
    /// Parses a NIfTI-1 or NIfTI-2 header, or with `analyze` also an
    /// Analyze 7.5 header; `None` if `h` is none.
    fn parse(h: &[u8], analyze: bool) -> Option<Header> {
        if h.len() < NIFTI1_LEN {
            return None;
        }
        let size = |little: bool| {
            let b: [u8; 4] = h[..4].try_into().expect("4 bytes");
            if little { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) }
        };
        let (little, version) = [true, false].into_iter().find_map(|l| match size(l) {
            348 if matches!(&h[344..348], b"n+1\0" | b"ni1\0") => Some((l, 1)),
            540 if h.len() >= NIFTI2_LEN && matches!(&h[4..8], b"n+2\0" | b"ni2\0") => Some((l, 2)),
            348 if analyze => Some((l, 0)),
            _ => None,
        })?;
        let i16_at = |o: usize| {
            let b = [h[o], h[o + 1]];
            if little { i16::from_le_bytes(b) } else { i16::from_be_bytes(b) }
        };
        let i32_at = |o: usize| {
            let b: [u8; 4] = h[o..o + 4].try_into().expect("4 bytes");
            if little { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) }
        };
        let i64_at = |o: usize| {
            let b: [u8; 8] = h[o..o + 8].try_into().expect("8 bytes");
            if little { i64::from_le_bytes(b) } else { i64::from_be_bytes(b) }
        };
        let f32_at = |o: usize| f32::from_bits(i32_at(o) as u32) as f64;
        let f64_at = |o: usize| f64::from_bits(i64_at(o) as u64);
        if version == 0 {
            // Analyze has no magic: require sane dimensions and a known type.
            let ndim = i16_at(40);
            if !(1..=7).contains(&ndim) || (1..=ndim as usize).any(|i| i16_at(40 + 2 * i) < 1) || !matches!(i16_at(70), 2 | 4 | 8 | 16 | 64 | 128) {
                return None;
            }
            let unit = text(&h[56..60]);
            return Some(Header {
                little,
                single: false,
                version,
                datatype: i16_at(70),
                dim: std::array::from_fn(|i| i16_at(40 + 2 * i) as i64),
                pixdim: std::array::from_fn(|i| f32_at(76 + 4 * i)),
                // A negative offset applies to every image; the first is at its magnitude.
                vox_offset: f32_at(108).abs().min(u32::MAX as f64) as u64,
                scl_slope: f32_at(112),
                scl_inter: 0.0,
                xyzt_units: 0,
                vox_unit_nm: match unit.to_ascii_lowercase().as_str() {
                    "mm" => Some(1e6),
                    "um" | "µm" | "micron" => Some(1e3),
                    "nm" => Some(1.0),
                    "cm" => Some(1e7),
                    "m" => Some(1e9),
                    _ => None,
                },
                descrip: text(&h[148..228]),
                intent_name: String::new(),
                intent_code: 0,
                qform_code: 0,
                sform_code: 0,
            });
        }
        Some(if version == 1 {
            Header {
                little,
                single: &h[344..348] == b"n+1\0",
                version,
                datatype: i16_at(70),
                dim: std::array::from_fn(|i| i16_at(40 + 2 * i) as i64),
                pixdim: std::array::from_fn(|i| f32_at(76 + 4 * i)),
                vox_offset: f32_at(108).max(0.0) as u64,
                scl_slope: f32_at(112),
                scl_inter: f32_at(116),
                xyzt_units: h[123],
                vox_unit_nm: None,
                descrip: text(&h[148..228]),
                intent_name: text(&h[328..344]),
                intent_code: i16_at(68) as i32,
                qform_code: i16_at(252) as i32,
                sform_code: i16_at(254) as i32,
            }
        } else {
            Header {
                little,
                single: &h[4..8] == b"n+2\0",
                version,
                datatype: i16_at(12),
                dim: std::array::from_fn(|i| i64_at(16 + 8 * i)),
                pixdim: std::array::from_fn(|i| f64_at(104 + 8 * i)),
                vox_offset: i64_at(168).max(0) as u64,
                scl_slope: f64_at(176),
                scl_inter: f64_at(184),
                xyzt_units: i32_at(500) as u8,
                vox_unit_nm: None,
                descrip: text(&h[240..320]),
                intent_name: text(&h[508..524]),
                intent_code: i32_at(504),
                qform_code: i32_at(344),
                sform_code: i32_at(348),
            }
        })
    }
}

fn lower_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// Header bytes of `path`: the start of the file, decompressed for
/// `.nii.gz`; for an `.img`, the start of its `.hdr`.
fn header_bytes(path: &Path, head: &[u8]) -> Option<Vec<u8>> {
    let name = lower_name(path);
    if head.starts_with(&[0x1F, 0x8B]) {
        let mut out = vec![0u8; NIFTI2_LEN];
        let n = crate::codecs::gzip(head, &mut out).ok()?;
        out.truncate(n);
        return Some(out);
    }
    // The probe passes fewer bytes than a NIfTI-2 header has.
    let file = if name.ends_with(".img") { path.with_extension("hdr") } else { path.to_path_buf() };
    let mut h = vec![0u8; NIFTI2_LEN];
    let n = File::open(file).ok()?.take(NIFTI2_LEN as u64).read(&mut h).ok()?;
    h.truncate(n);
    Some(h)
}

/// Analyze headers are accepted for `.hdr` and `.img` files only.
fn is_pair_name(path: &Path) -> bool {
    let name = lower_name(path);
    name.ends_with(".hdr") || name.ends_with(".img")
}

/// Detects a NIfTI file or Analyze 7.5 pair (header or image file).
pub(super) fn probe(path: &Path, head: &[u8]) -> Option<FormatId> {
    let h = Header::parse(&header_bytes(path, head)?, is_pair_name(path))?;
    Some(if h.version == 0 { FormatId::Analyze } else { FormatId::Nifti })
}

/// Where the voxels are.
#[derive(Debug)]
enum Data {
    File { file: File, offset: u64 },
    /// `.nii.gz`: the compressed file, decompressed on first use.
    Gzip { path: PathBuf, len: u64, offset: u64, data: OnceLock<std::result::Result<Vec<u8>, String>> },
}

/// NIfTI file.
#[derive(Debug)]
pub(super) struct Nifti {
    data: Data,
    little: bool,
    /// Stored bytes per sample.
    bytes: usize,
    plane_bytes: u64,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
    format: &'static FormatInfo,
}

impl Nifti {
    /// Opens a NIfTI file (`.nii`, `.nii.gz`, `.hdr` or `.img`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers or short files and
    /// [`Error::UnsupportedVariant`] for complex or 64-bit integer data.
    pub fn open(path: &Path) -> Result<Self> {
        let mut head = vec![0u8; 512];
        let n = std::io::Read::read(&mut File::open(path)?, &mut head)?;
        head.truncate(n);
        let gz = head.starts_with(&[0x1F, 0x8B]);
        let raw = header_bytes(path, &head).ok_or_else(|| Error::Corrupt("unreadable NIfTI header".into()))?;
        let h = Header::parse(&raw, is_pair_name(path)).ok_or_else(|| Error::Corrupt("not a NIfTI header".into()))?;
        let (pixel_type, samples, bytes) = match h.datatype {
            2 => (PixelType::U8, 1, 1),
            4 => (PixelType::I16, 1, 2),
            8 => (PixelType::I32, 1, 4),
            16 => (PixelType::F32, 1, 4),
            64 => (PixelType::F64, 1, 8),
            256 => (PixelType::I8, 1, 1),
            512 => (PixelType::U16, 1, 2),
            768 => (PixelType::U32, 1, 4),
            128 => (PixelType::U8, 3, 1),
            2304 => (PixelType::U8, 4, 1),
            d => return Err(Error::UnsupportedVariant(format!("NIfTI data type {d}"))),
        };
        let ndim = h.dim[0].clamp(1, 7) as usize;
        let size = |i: usize| if i <= ndim { h.dim[i].max(1) } else { 1 };
        let (nx, ny, nz) = (size(1), size(2), size(3));
        let size_c = size(5);
        let size_t = size(4).saturating_mul(size(6)).saturating_mul(size(7));
        if [nx, ny, nz, size_c, size_t].iter().any(|v| *v > u32::MAX as i64) {
            return Err(Error::Corrupt("NIfTI dimensions too large".into()));
        }
        let plane_bytes = nx as u64 * ny as u64 * samples as u64 * bytes as u64;
        let total = plane_bytes
            .checked_mul(nz as u64 * size_t as u64)
            .and_then(|v| v.checked_mul(size_c as u64))
            .ok_or_else(|| Error::Corrupt("NIfTI volume too large".into()))?;
        let data = if gz {
            let need = h.vox_offset.checked_add(total).filter(|v| *v <= MAX_DECOMPRESSED);
            let need = need.ok_or_else(|| Error::UnsupportedVariant("compressed NIfTI volume above 4 GiB".into()))?;
            Data::Gzip { path: path.to_path_buf(), len: need, offset: h.vox_offset, data: OnceLock::new() }
        } else {
            let file = if h.single { path.to_path_buf() } else { path.with_extension("img") };
            let file = File::open(&file)?;
            let offset = if h.single || h.version == 0 { h.vox_offset } else { 0 };
            if offset.saturating_add(total) > file.metadata()?.len() {
                return Err(Error::Corrupt("NIfTI file shorter than its voxels".into()));
            }
            Data::File { file, offset }
        };
        // Units: bits 0-2 space, bits 3-5 time.
        let space_nm = match h.xyzt_units & 7 {
            1 => Some(1e9),
            2 => Some(1e6),
            3 => Some(1e3),
            _ => None,
        };
        let time_s = match h.xyzt_units & 0x38 {
            8 => Some(1.0),
            16 => Some(1e-3),
            24 => Some(1e-6),
            _ => None,
        };
        let space_nm = space_nm.or(h.vox_unit_nm);
        let spacing = |i: usize| space_nm.map(|u| h.pixdim[i].abs() * u).filter(|v| v.is_finite() && *v > 0.0);
        let meta = SeriesMeta {
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()),
            description: Some(h.descrip.clone()).filter(|d| !d.is_empty()),
            size_c: size_c as u32,
            size_z: nz as u32,
            size_t: size_t as u32,
            pixel_type,
            samples_per_pixel: samples,
            levels: vec![LevelMeta { size_x: nx as u64, size_y: ny as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: spacing(1), y_nm: spacing(2), z_nm: spacing(3).filter(|_| nz > 1) },
            time_increment_s: time_s.map(|u| h.pixdim[4].abs() * u).filter(|v| v.is_finite() && *v > 0.0 && size_t > 1),
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        original.pairs.push(("Version".into(), if h.version == 0 { "Analyze 7.5".into() } else { format!("NIfTI-{}", h.version) }));
        original.pairs.push(("Dimensions".into(), h.dim[1..=ndim].iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" x ")));
        original.pairs.push(("Data type".into(), h.datatype.to_string()));
        if h.scl_slope != 0.0 && h.scl_slope.is_finite() {
            original.pairs.push(("Scale slope".into(), h.scl_slope.to_string()));
            original.pairs.push(("Scale intercept".into(), h.scl_inter.to_string()));
        }
        original.pairs.push(("qform code".into(), h.qform_code.to_string()));
        original.pairs.push(("sform code".into(), h.sform_code.to_string()));
        if h.intent_code != 0 {
            original.pairs.push(("Intent code".into(), h.intent_code.to_string()));
        }
        if !h.intent_name.is_empty() {
            original.pairs.push(("Intent name".into(), h.intent_name.clone()));
        }
        let format = if h.version == 0 { &ANALYZE_FORMAT } else { &FORMAT };
        Ok(Nifti { data, little: h.little, bytes, plane_bytes, metas: [meta], original, format })
    }

    /// Copies `out.len()` voxel bytes at `at` (from the start of the data).
    fn read_at(&self, out: &mut [u8], at: u64) -> Result<()> {
        match &self.data {
            Data::File { file, offset } => read_exact_at(file, out, offset + at),
            Data::Gzip { path, len, offset, data } => {
                let data = data.get_or_init(|| {
                    let load = || -> Result<Vec<u8>> {
                        let src = std::fs::read(path)?;
                        let mut out = vec![0u8; *len as usize];
                        let n = crate::codecs::gzip(&src, &mut out)?;
                        if n < out.len() {
                            return Err(Error::Corrupt("compressed NIfTI file shorter than its voxels".into()));
                        }
                        Ok(out)
                    };
                    load().map_err(|e| e.to_string())
                });
                let data = data.as_ref().map_err(|e| Error::Corrupt(e.clone()))?;
                let start = (offset + at) as usize;
                out.copy_from_slice(&data[start..start + out.len()]);
                Ok(())
            }
        }
    }
}

impl Backend for Nifti {
    fn format(&self) -> &'static FormatInfo {
        self.format
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[0];
        // Stored order: z, then t, then vector components.
        let section = plane.z as u64 + m.size_z as u64 * (plane.t as u64 + m.size_t as u64 * plane.c as u64);
        let pixel = self.bytes * m.samples_per_pixel as usize;
        let width = m.levels[0].size_x;
        let w = region.width as usize;
        let swap = self.bytes > 1 && self.little != cfg!(target_endian = "little");
        scratch::with(w * pixel, |row| {
            for r in 0..region.height as usize {
                let at = section * self.plane_bytes + ((region.y + r as u64) * width + region.x) * pixel as u64;
                self.read_at(row, at)?;
                if swap {
                    for s in row.chunks_exact_mut(self.bytes) {
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
    fn parses_both_versions() {
        let mut h = vec![0u8; NIFTI2_LEN];
        h[..4].copy_from_slice(&348i32.to_be_bytes());
        h[344..348].copy_from_slice(b"n+1\0");
        let p = Header::parse(&h, false).unwrap();
        assert!(!p.little && p.single && p.version == 1);
        let mut h = vec![0u8; NIFTI2_LEN];
        h[..4].copy_from_slice(&540i32.to_le_bytes());
        h[4..8].copy_from_slice(b"ni2\0");
        let p = Header::parse(&h, false).unwrap();
        assert!(p.little && !p.single && p.version == 2);
        h[4] = b'x';
        assert!(Header::parse(&h, false).is_none());
        // Analyze 7.5: no magic, accepted only for pair names and sane fields.
        let mut h = vec![0u8; NIFTI1_LEN];
        h[..4].copy_from_slice(&348i32.to_le_bytes());
        h[40..44].copy_from_slice(&[2, 0, 3, 0]);
        h[44..46].copy_from_slice(&[2, 0]);
        h[56..58].copy_from_slice(b"mm");
        h[70] = 4;
        assert!(Header::parse(&h, false).is_none());
        let p = Header::parse(&h, true).unwrap();
        assert_eq!((p.version, p.vox_unit_nm), (0, Some(1e6)));
        h[70] = 3;
        assert!(Header::parse(&h, true).is_none());
    }
}

//! # lsm
//!
//! Zeiss LSM reader (LSM 510 and later confocal files).
//!
//! An LSM file is a little-endian TIFF: one image directory per plane (z
//! fastest, then time), each followed by a thumbnail directory
//! (NewSubfileType 1). The samples of a plane are the channels, stored
//! planar with one strip each. The CZ_LSMINFO tag (34412) of the first
//! directory holds the dimensions, voxel sizes, scan type and offsets to
//! the channel names and colours and the time stamps. Strip offsets of
//! files above 4 GiB wrap around 2^32 and are unwrapped by their order.
//!
//! Provenance: Carl Zeiss "File Format Description - LSM 5xx Release 2.0"
//! (`doc/zeiss-lsm/Lsmfile.doc`); own analysis of sample files. No GPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::slide::{PlaneImage, Slide, SlideSeries};
use super::tiff::ifd::{TiffFile, tag};
use super::tiff::image::{Samples, TiffImage};
use super::tiff::{level_meta, original_meta};
use crate::io::read_exact_at;
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, OpenOptions, PhysicalSize, PlaneMeta, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Lsm, "Zeiss LSM", "1").extensions(&["lsm"]);

/// Private tag with the CZ_LSMINFO structure.
pub(super) const TAG_LSMINFO: u16 = 34412;
/// Size of the parsed part of CZ_LSMINFO.
const INFO_LEN: usize = 128;

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    f64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

/// Reads `len` bytes at `at`, `None` beyond the end of the file.
fn read(tiff: &TiffFile, at: u64, len: usize) -> Option<Vec<u8>> {
    if at == 0 || at.checked_add(len as u64)? > tiff.len || len > 1 << 24 {
        return None;
    }
    let mut b = vec![0u8; len];
    read_exact_at(&tiff.file, &mut b, at).ok()?;
    Some(b)
}

/// Channel names and colours (`u32OffsetChannelColors`).
fn channel_colors(tiff: &TiffFile, at: u64, channels: usize) -> Vec<ChannelMeta> {
    let mut out = vec![ChannelMeta::default(); channels];
    let Some(head) = read(tiff, at, 24) else { return out };
    let size = i32_at(&head, 0).clamp(0, 1 << 20) as usize;
    let Some(b) = read(tiff, at, size.max(24)) else { return out };
    let (n_colors, n_names) = (i32_at(&b, 4).max(0) as usize, i32_at(&b, 8).max(0) as usize);
    let (colors_at, names_at) = (i32_at(&b, 12).max(0) as usize, i32_at(&b, 16).max(0) as usize);
    for (c, ch) in out.iter_mut().enumerate().take(n_colors) {
        if let Some(v) = b.get(colors_at + 4 * c..colors_at + 4 * c + 4) {
            // 0x00BBGGRR: red in the lowest byte.
            ch.color = Some([v[0], v[1], v[2], 255]);
        }
    }
    if let Some(names) = b.get(names_at..) {
        // Each name may be preceded by a 4-byte length in later versions.
        let list: Vec<String> = names
            .split(|&x| x == 0)
            .map(|s| String::from_utf8_lossy(s).trim_matches(|c: char| c.is_control() || c == ' ').to_string())
            .filter(|s| !s.is_empty())
            .take(n_names.min(channels))
            .collect();
        for (ch, name) in out.iter_mut().zip(list) {
            ch.name = Some(name);
        }
    }
    out
}

/// Time stamps in seconds (`u32OffsetTimeStamps`).
fn time_stamps(tiff: &TiffFile, at: u64) -> Vec<f64> {
    let Some(head) = read(tiff, at, 8) else { return Vec::new() };
    let n = i32_at(&head, 4).clamp(0, 1 << 20) as usize;
    read(tiff, at + 8, 8 * n).map(|b| b.chunks_exact(8).map(|v| f64::from_le_bytes(v.try_into().expect("8 bytes"))).collect()).unwrap_or_default()
}

/// Opens an LSM file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for a missing CZ_LSMINFO or too few planes.
pub(super) fn open(path: &Path, _opts: &OpenOptions) -> Result<Slide> {
    let tiff = TiffFile::open(path)?;
    let ifd0 = &tiff.ifds[0];
    // The tag's value (BYTE array) is the structure.
    let entry = ifd0.entry(TAG_LSMINFO).ok_or_else(|| Error::Corrupt("LSM without CZ_LSMINFO".into()))?;
    let mut info = tiff.raw(entry)?;
    if info.len() < 64 {
        return Err(Error::Corrupt("LSM CZ_LSMINFO too short".into()));
    }
    info.resize(INFO_LEN.max(info.len()), 0);
    let dims = |at: usize| i32_at(&info, at).max(1) as u32;
    let (size_z, channels, size_t) = (dims(16), dims(20), dims(24));
    let data_type = i32_at(&info, 28);
    let voxel = |at: usize| Some(f64_at(&info, at)).filter(|v| v.is_finite() && *v > 1e-12 && *v < 1.0).map(|v| v * 1e9);

    // Pixel planes, thumbnails and the unwrapping of 32-bit strip offsets.
    let mut planes: Vec<TiffImage> = Vec::new();
    let mut thumbnail = None;
    let (mut wraps, mut last) = (0u64, 0u64);
    for ifd in &tiff.ifds {
        let subfile = tiff.uint(ifd, tag::NEW_SUBFILE_TYPE)?.unwrap_or(0);
        let first = tiff.uints(ifd, tag::STRIP_OFFSETS)?.and_then(|v| v.first().copied()).unwrap_or(0);
        if first != 0 {
            if first + (wraps << 32) < last && tiff.len > 1 << 32 {
                wraps += 1;
            }
            last = first + (wraps << 32);
        }
        // Samples are intensities; the photometric tag (RGB, palette) is for display.
        let photometric = if subfile & 1 != 0 { None } else { Some(1) };
        let Ok(image) = TiffImage::with_photometric(&tiff, ifd, photometric) else { continue };
        let image = if wraps > 0 { image.shifted(wraps << 32) } else { image };
        if subfile & 1 != 0 {
            thumbnail.get_or_insert(image);
        } else {
            planes.push(image);
        }
    }
    let first = planes.first().ok_or_else(|| Error::Corrupt("LSM without image planes".into()))?;
    let spp = first.spp as u32;
    let size_c = if spp == channels { channels } else { spp };
    let pages = planes.len() as u32;
    // Fewer stored planes than the dimensions say: keep complete time points.
    let (size_z, size_t) = if size_z * size_t <= pages {
        (size_z, size_t)
    } else if size_z <= pages {
        (size_z, (pages / size_z).max(1))
    } else {
        (pages, 1)
    };
    if planes.iter().take((size_z * size_t) as usize).any(|p| p.width != first.width || p.height != first.height || p.pixel_type != first.pixel_type || p.spp != first.spp) {
        return Err(Error::UnsupportedVariant("LSM planes with different layouts".into()));
    }
    let mut level = Vec::with_capacity((size_c * size_z * size_t) as usize);
    for t in 0..size_t {
        for z in 0..size_z {
            for c in 0..size_c {
                let image = planes[(t * size_z + z) as usize].clone();
                level.push(PlaneImage { image, samples: Samples::One(c as u16), restart: None });
            }
        }
    }
    let at = |o: usize| u32_at(&info, o) as u64;
    let mut channels_meta = channel_colors(&tiff, at(84), size_c as usize);
    let stamps = time_stamps(&tiff, at(108));
    let interval = f64_at(&info, 88);
    let increment = if stamps.len() > 1 {
        Some((stamps[stamps.len() - 1] - stamps[0]) / (stamps.len() - 1) as f64)
    } else {
        Some(interval).filter(|v| v.is_finite() && *v > 0.0)
    };
    let significant_bits = match data_type {
        2 => Some(12),
        _ => None,
    }
    .filter(|_| first.pixel_type == crate::PixelType::U16);
    channels_meta.truncate(size_c as usize);
    let mut plane_meta = Vec::new();
    if stamps.len() >= size_t as usize && size_t > 1 {
        for t in 0..size_t as usize {
            for _ in 0..(size_z * size_c) {
                plane_meta.push(PlaneMeta { delta_t_s: Some(stamps[t] - stamps[0]), ..Default::default() });
            }
        }
    }
    let meta = SeriesMeta {
        size_c,
        size_z,
        size_t,
        pixel_type: first.pixel_type,
        significant_bits,
        samples_per_pixel: 1,
        levels: vec![level_meta(first, first.width)],
        physical_size: PhysicalSize { x_nm: voxel(40), y_nm: voxel(48), z_nm: voxel(56) },
        time_increment_s: increment.filter(|_| size_t > 1),
        channels: channels_meta,
        ..Default::default()
    };
    let mut slide = Slide::new(&FORMAT, tiff);
    slide.original.pairs = original_meta(&slide.tiff, &slide.tiff.ifds[0])?;
    slide.original.pairs.extend([
        ("DimensionZ".into(), dims(16).to_string()),
        ("DimensionChannels".into(), dims(20).to_string()),
        ("DimensionTime".into(), dims(24).to_string()),
        ("DataType".into(), data_type.to_string()),
        ("ScanType".into(), u32_at(&info, 64).to_string()),
    ]);
    slide.metas.push(meta);
    slide.series.push(SlideSeries { levels: vec![level], plane_meta });
    if let Some(t) = thumbnail {
        slide.add_associated(AssociatedKind::Thumbnail, t);
    }
    slide.validate()?;
    Ok(slide)
}

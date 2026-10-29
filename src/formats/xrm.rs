//! # xrm
//!
//! Zeiss Xradia reader: `.txm` (reconstructed volume), `.txrm` (projection
//! series) and `.xrm` (single image).
//!
//! The files are OLE2 compound files (see [`cfb`](super::cfb)). Storage
//! `ImageInfo` holds the metadata as small little-endian streams:
//! `NoOfImages`, `ImageWidth`, `ImageHeight` (u32), `DataType` (5 uint16,
//! 10 float32), `PixelSize` (f32, micrometres) and per image the arrays
//! `Angles` (degrees), `XPosition` / `YPosition` / `ZPosition`
//! (micrometres) and `ExpTimes` (seconds). Image `n` (from 1) is the stream
//! `ImageData<ceil(n / 100)>/Image<n>`: rows of little-endian samples.
//! Images are z planes (volume slices or projections); a region read
//! touches only the rows it needs.
//!
//! Provenance: Library of Congress format description fdd000581 (OLE2
//! container, `doc/zeiss-xrm-txrm/`); `dxchange` (BSD-3-Clause, UChicago
//! Argonne; read as documentation, notice in THIRD_PARTY_NOTICES.md) for
//! the stream names, data types and the image grouping; own analysis of a
//! public Xradia Versa reconstruction (Zenodo 4432446, CC BY 4.0); checked
//! against Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::cfb::{self, Cfb};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, PlaneMeta,
    Position, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Xrm, "Zeiss Xradia XRM", "1").extensions(&["txm", "txrm", "xrm"]);

/// Most images of one file, against corrupt counts.
const MAX_IMAGES: u32 = 1 << 20;

/// Returns `true` for a compound file with Xradia image information.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    cfb::matches(head) && matches!(ext.as_str(), "txm" | "txrm" | "xrm") && Cfb::open(path).is_ok_and(|c| c.stream("ImageInfo/ImageWidth").is_some())
}

/// Zeiss Xradia file.
#[derive(Debug)]
pub(super) struct Xrm {
    cfb: Cfb,
    /// Stream path of each image.
    images: Vec<String>,
    plane_meta: Vec<PlaneMeta>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Xrm {
    /// Opens a TXM, TXRM or XRM file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for missing image information and
    /// [`Error::UnsupportedVariant`] for unknown data types.
    pub fn open(path: &Path) -> Result<Self> {
        let cfb = Cfb::open(path)?;
        let read = |name: &str| -> Option<Vec<u8>> {
            let s = cfb.stream(&format!("ImageInfo/{name}"))?;
            (s.size <= 1 << 26).then(|| cfb.read(s).ok()).flatten()
        };
        let u32_of = |name: &str| read(name).filter(|b| b.len() >= 4).map(|b| u32::from_le_bytes(b[..4].try_into().expect("4 bytes")));
        let f32s = |name: &str| -> Vec<f32> { read(name).map(|b| b.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().expect("4 bytes"))).collect()).unwrap_or_default() };
        let (Some(width), Some(height)) = (u32_of("ImageWidth"), u32_of("ImageHeight")) else {
            return Err(Error::Corrupt("Xradia file without image size".into()));
        };
        let count = u32_of("NoOfImages").unwrap_or(1).max(1);
        if count > MAX_IMAGES || width == 0 || height == 0 {
            return Err(Error::Corrupt(format!("Xradia file of {count} images of {width} x {height}")));
        }
        let pixel_type = match u32_of("DataType").unwrap_or(5) {
            5 => PixelType::U16,
            10 => PixelType::F32,
            d => return Err(Error::UnsupportedVariant(format!("Xradia data type {d}"))),
        };
        let plane_len = width as u64 * height as u64 * pixel_type.bytes() as u64;
        let mut images = Vec::with_capacity(count as usize);
        for n in 1..=count {
            let name = format!("ImageData{}/Image{n}", n.div_ceil(100));
            match cfb.stream(&name) {
                Some(s) if s.size >= plane_len => images.push(s.path.clone()),
                // Images not written (an aborted scan) end the series.
                _ => break,
            }
        }
        if images.is_empty() {
            return Err(Error::Corrupt("Xradia file without image data".into()));
        }
        let (angles, xs, ys, zs, exposures) = (f32s("Angles"), f32s("XPosition"), f32s("YPosition"), f32s("ZPosition"), f32s("ExpTimes"));
        let um = |v: &[f32], i: usize| v.get(i).map(|&v| v as f64 * 1e3).filter(|v| v.is_finite());
        let plane_meta = (0..images.len())
            .map(|i| {
                let position = Position { x_nm: um(&xs, i), y_nm: um(&ys, i), z_nm: um(&zs, i) };
                PlaneMeta {
                    delta_t_s: None,
                    exposure_s: exposures.get(i).map(|&v| v as f64).filter(|v| v.is_finite() && *v > 0.0),
                    position: (position != Position::default()).then_some(position),
                }
            })
            .collect();
        let pixel_nm = f32s("PixelSize").first().map(|&v| v as f64 * 1e3).filter(|v| v.is_finite() && *v > 0.0);
        let meta = SeriesMeta {
            name: path.file_stem().map(|s| s.to_string_lossy().into_owned()),
            size_c: 1,
            size_z: images.len() as u32,
            size_t: 1,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: pixel_nm, y_nm: pixel_nm, z_nm: None },
            channels: vec![ChannelMeta::default()],
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        for key in ["NoOfImages", "ImagesTaken", "DataType", "FileType", "CameraBinning", "CameraNo"] {
            if let Some(v) = u32_of(key) {
                original.pairs.push((key.into(), v.to_string()));
            }
        }
        for key in ["PixelSize", "Voltage", "Current", "CamPixelSize", "DetToRA", "SrcToRA"] {
            if let Some(v) = f32s(key).first() {
                original.pairs.push((key.into(), v.to_string()));
            }
        }
        for key in ["CameraName", "ObjectiveName", "ReferenceFile"] {
            if let Some(b) = read(key) {
                let text = String::from_utf8_lossy(&b).split('\0').next().unwrap_or_default().trim().to_string();
                if !text.is_empty() {
                    original.pairs.push((key.into(), text));
                }
            }
        }
        if !angles.is_empty() {
            let shown: Vec<String> = angles.iter().take(images.len()).map(|a| a.to_string()).collect();
            original.pairs.push(("Angles (degrees)".into(), shown.join(" ")));
        }
        Ok(Xrm { cfb, images, plane_meta, metas: vec![meta], original })
    }
}

impl Backend for Xrm {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        Ok(self.plane_meta.get(plane.z as usize).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[0];
        let stream = self.cfb.stream(&self.images[plane.z as usize]).ok_or_else(|| Error::Corrupt("Xradia image stream missing".into()))?;
        let bytes = m.pixel_type.bytes();
        let row = m.levels[0].size_x as usize * bytes;
        let w = region.width as usize;
        let little = cfg!(target_endian = "little");
        let fix = |d: &mut [u8]| {
            if bytes > 1 && !little {
                d.chunks_exact_mut(bytes).for_each(<[u8]>::reverse);
            }
        };
        match self.cfb.sectors(stream)? {
            // The rows of the region in one read, then row by row to the sink.
            Some(sectors) => {
                let (h, first) = (region.height as usize, region.y as usize * row + region.x as usize * bytes);
                let span = (h - 1) * row + w * bytes;
                scratch::with(span, |block| -> Result<()> {
                    self.cfb.read_at(&sectors, first as u64, block)?;
                    for r in 0..h {
                        sink.write_with(r, 0, w, |d| {
                            d.copy_from_slice(&block[r * row..r * row + w * bytes]);
                            fix(d);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
            }
            // Small images live in the mini stream: read them whole.
            None => {
                let data = self.cfb.read(stream)?;
                for r in 0..region.height as usize {
                    let at = (region.y as usize + r) * row + region.x as usize * bytes;
                    let src = data.get(at..at + w * bytes).ok_or_else(|| Error::Corrupt("Xradia image stream too short".into()))?;
                    sink.write_with(r, 0, w, |d| {
                        d.copy_from_slice(src);
                        fix(d);
                        Ok(())
                    })?;
                }
            }
        }
        Ok(())
    }
}

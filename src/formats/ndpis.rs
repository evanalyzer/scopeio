//! # ndpis
//!
//! Hamamatsu NDPIS reader: a fluorescence image set, one NDPI file per
//! channel.
//!
//! The `.ndpis` file is INI text (`[NanoZoomer Digital Pathology Image
//! Set]`, `NoImages`, `Image0..`) naming the channel files next to it.
//! Each channel file is a pseudo-coloured RGB slide whose signal lies in one
//! component (blue for DAPI, green for FITC, ...); it is chosen once from
//! the smallest pyramid level and returned as a gray channel. Channel
//! names come from tag 65434, the wavelengths from the metadata block
//! (`ExcitationWaveLength`, `EmissionWaveLength`).
//!
//! Provenance: openslide.org Hamamatsu page (prose, CC BY-SA 4.0); own
//! analysis of sample files, checked against Bio-Formats as a black box. No
//! GPL/LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::Path;

use super::slide::Slide;
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage};
use crate::{
    AssociatedKind, Error, FormatId, FormatInfo, Layout, OpenOptions, OriginalMeta, OutputType, PixelType, Plane,
    PlaneMeta, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Ndpis, "Hamamatsu NDPIS", "1");

const HEADER: &[u8] = b"[NanoZoomer Digital Pathology Image Set]";
/// Channel name tag of NDPI fluorescence files.
const TAG_CHANNEL: u16 = 65434;
const TAG_METADATA: u16 = 65449;

/// Returns `true` for an NDPIS set file.
pub(super) fn matches(head: &[u8]) -> bool {
    let head = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);
    head.starts_with(HEADER)
}

/// Hamamatsu NDPIS image set.
#[derive(Debug)]
pub(super) struct Ndpis {
    /// Each channel's slide and the RGB component holding its signal.
    channels: Vec<(Slide, usize)>,
    metas: Vec<SeriesMeta>,
    names: Vec<String>,
}

/// `key=value` of an NDPI metadata block (CR-separated, keys may start with `;`).
fn metadata_value(text: &str, key: &str) -> Option<f64> {
    text.split(['\r', '\n']).find_map(|l| l.trim().trim_start_matches(';').strip_prefix(key)?.strip_prefix('=')?.trim().parse().ok())
}

impl Ndpis {
    /// Opens an NDPIS set and its channel files.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for a set without images or channel files
    /// whose pyramids differ, [`Error::Io`] for missing channel files.
    pub fn open(path: &Path, opts: &OpenOptions) -> Result<Self> {
        let text = std::fs::read(path)?;
        if text.len() > 1 << 20 {
            return Err(Error::Corrupt("NDPIS file too large".into()));
        }
        let text = String::from_utf8_lossy(&text);
        let mut files: Vec<(u32, String)> = text
            .lines()
            .filter_map(|l| {
                let (k, v) = l.split_once('=')?;
                let i = k.trim().strip_prefix("Image")?.parse().ok()?;
                Some((i, v.trim().to_string()))
            })
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(Error::Corrupt("NDPIS without images".into()));
        }
        let dir = path.parent().unwrap_or(Path::new("."));
        let mut channels = Vec::new();
        let mut names = Vec::new();
        for (_, name) in &files {
            let slide = super::ndpi::open(&dir.join(name), opts)?;
            names.push(name.clone());
            channels.push(slide);
        }
        let first = &channels[0].metas[0];
        for s in &channels[1..] {
            let m = &s.metas[0];
            let same = m.levels.len() == first.levels.len() && m.levels.iter().zip(&first.levels).all(|(a, b)| a.size_x == b.size_x && a.size_y == b.size_y);
            if !same || m.pixel_type != PixelType::U8 || m.samples_per_pixel != first.samples_per_pixel || m.size_z != first.size_z {
                return Err(Error::Corrupt("NDPIS channel files with different pyramids".into()));
            }
        }
        let mut meta = first.clone();
        meta.size_c = channels.len() as u32;
        meta.samples_per_pixel = 1;
        meta.channels = channels
            .iter()
            .map(|s| {
                let ifd = &s.tiff.ifds[0];
                let name = s.tiff.string(ifd, TAG_CHANNEL).ok().flatten().map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
                let md = s.tiff.string(ifd, TAG_METADATA).ok().flatten().unwrap_or_default();
                crate::ChannelMeta {
                    name,
                    excitation_wavelength_nm: metadata_value(&md, "ExcitationWaveLength").filter(|v| *v > 0.0),
                    emission_wavelength_nm: metadata_value(&md, "EmissionWaveLength").filter(|v| *v > 0.0),
                    ..Default::default()
                }
            })
            .collect();
        let mut out = Vec::with_capacity(channels.len());
        for s in channels {
            let component = if s.metas[0].samples_per_pixel >= 3 { dominant(&s)? } else { 0 };
            out.push((s, component));
        }
        Ok(Ndpis { channels: out, metas: vec![meta], names })
    }
}

/// The RGB component with the most signal on the smallest level.
fn dominant(s: &Slide) -> Result<usize> {
    let m = &s.metas[0];
    let level = m.levels.len() - 1;
    let l = &m.levels[level];
    let region = Region::new(0, 0, l.size_x, l.size_y);
    let (w, h) = (l.size_x as usize, l.size_y as usize);
    if w * h > 1 << 24 {
        return Ok(0);
    }
    scratch::with(w * h * 3, |buf| {
        let mut sink = Sink::new(buf, Layout::Hwc, w * 3, 0, 3, 0, &[0, 1, 2], 3, PixelType::U8, OutputType::Native);
        s.read_native(0, level, Plane::default(), region, &mut sink)?;
        let mut sums = [0u64; 3];
        for px in buf.chunks_exact(3) {
            for k in 0..3 {
                sums[k] += px[k] as u64;
            }
        }
        Ok((0..3).max_by_key(|&k| sums[k]).unwrap_or(0))
    })
}

impl Backend for Ndpis {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        self.channels[plane.c as usize].0.plane_meta(series, Plane { c: 0, ..plane })
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let mut meta = self.channels[0].0.original_metadata(series)?;
        for (i, n) in self.names.iter().enumerate() {
            meta.pairs.push((format!("Image{i}"), n.clone()));
        }
        Ok(meta)
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        self.channels[0].0.associated_images()
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        self.channels[0].0.read_associated_native(kind)
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let (slide, component) = &self.channels[plane.c as usize];
        let spp = slide.metas[0].samples_per_pixel as usize;
        let inner = Plane { c: 0, ..plane };
        if spp == 1 {
            return slide.read_native(series, level, inner, region, sink);
        }
        let (w, h) = (region.width as usize, region.height as usize);
        // The colour region in scratch, then the signal component.
        scratch::with(w * h * spp, |buf| {
            let map: Vec<u8> = (0..spp as u8).collect();
            let mut rgb = Sink::new(buf, Layout::Hwc, w * spp, 0, spp, 0, &map, spp, PixelType::U8, OutputType::Native);
            slide.read_native(series, level, inner, region, &mut rgb)?;
            for y in 0..h {
                let row = &buf[y * w * spp..(y + 1) * w * spp];
                sink.write_with(y, 0, w, |d| {
                    for (o, px) in d.iter_mut().zip(row.chunks_exact(spp)) {
                        *o = px[*component];
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

    #[test]
    fn header_and_metadata() {
        assert!(matches(b"[NanoZoomer Digital Pathology Image Set]\r\nNoImages=3"));
        assert!(!matches(b"[Other]"));
        assert_eq!(metadata_value("a=1\r;EmissionWaveLength=455\r", "EmissionWaveLength"), Some(455.0));
    }
}

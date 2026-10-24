//! # nd
//!
//! MetaMorph ND (multi-dimensional acquisition) reader.
//!
//! An `.nd` file is text, one `"Key", value` per line: whether and how many
//! time points (`DoTimelapse`, `NTimePoints`), stage positions (`DoStage`,
//! `NStagePositions`, `Stage<n>`), wavelengths (`DoWave`, `NWavelengths`,
//! `WaveName<n>`, `WaveInFileName`) and z steps (`DoZSeries`, `NZSteps`)
//! were acquired. The data files sit next to it, named
//! `<base>[_w<n>[<wave name>]][_s<n>][_t<n>].TIF` (`.STK` for z series,
//! whose planes are the z steps). Each stage position is one series; a
//! missing file reads as zeros. Pixel sizes come from the first data file.
//!
//! Provenance: own analysis of the public ND acquisitions on the OME sample
//! server (CC0 / CC BY 4.0); checked against Bio-Formats as a black box. No
//! GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::tiff::Tiff;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, OpenOptions, OriginalMeta, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Nd, "MetaMorph ND", "1");

/// Most files of one acquisition, against corrupt counts.
const MAX_FILES: usize = 1 << 20;

/// Returns `true` for an ND file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("nd")) && head.starts_with(b"\"NDInfoFile\"")
}

/// `"Key", value` pairs (value unquoted).
fn parse(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(',')?;
            Some((k.trim().trim_matches('"').to_string(), v.trim().trim_matches('"').to_string()))
        })
        .collect()
}

/// The file of `base` + suffix, with any of the usual extensions, matched
/// without regard to case.
fn find(dir: &Path, stem: &str) -> Option<PathBuf> {
    for ext in ["TIF", "tif", "STK", "stk", "TIFF", "tiff"] {
        let p = dir.join(format!("{stem}.{ext}"));
        if p.exists() {
            return Some(p);
        }
    }
    let lower = stem.to_ascii_lowercase();
    std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
        p.file_stem().is_some_and(|s| s.to_string_lossy().to_ascii_lowercase() == lower)
            && p.extension().is_some_and(|e| matches!(e.to_string_lossy().to_ascii_lowercase().as_str(), "tif" | "tiff" | "stk"))
    })
}

/// MetaMorph ND acquisition.
#[derive(Debug)]
pub(super) struct Nd {
    /// Data file of each series by `wavelength + waves * t`.
    files: Vec<Vec<Option<PathBuf>>>,
    waves: u32,
    /// Channels inside one data file.
    file_c: u32,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Nd {
    /// Opens an ND file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid ND file and
    /// [`Error::UnsupportedVariant`] when no data file can be read.
    pub fn open(path: &Path, opts: &OpenOptions) -> Result<Self> {
        if std::fs::metadata(path)?.len() > 1 << 20 {
            return Err(Error::Corrupt("ND file too large".into()));
        }
        let text = String::from_utf8_lossy(&std::fs::read(path)?).into_owned();
        let kv = parse(&text);
        let yes = |k: &str| kv.get(k).is_some_and(|v| v.eq_ignore_ascii_case("TRUE"));
        let count = |k: &str| kv.get(k).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let times = if yes("DoTimelapse") { count("NTimePoints").max(1) } else { 1 };
        let stages = if yes("DoStage") { count("NStagePositions").max(1) } else { 1 };
        let waves = if yes("DoWave") { count("NWavelengths").max(1) } else { 1 };
        if times.saturating_mul(stages).saturating_mul(waves) > MAX_FILES {
            return Err(Error::Corrupt("ND file with too many data files".into()));
        }
        let names: Vec<String> = (1..=waves).map(|w| kv.get(&format!("WaveName{w}")).cloned().unwrap_or_default()).collect();
        let in_name = !kv.get("WaveInFileName").is_some_and(|v| v.eq_ignore_ascii_case("FALSE"));
        let dir = path.parent().unwrap_or(Path::new("."));
        let base = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut files = Vec::new();
        for s in 1..=stages {
            let mut list = Vec::with_capacity(waves * times);
            for t in 1..=times {
                for (w, name) in names.iter().enumerate() {
                    let mut stem = base.clone();
                    if yes("DoWave") {
                        stem += &format!("_w{}{}", w + 1, if in_name { name.as_str() } else { "" });
                    }
                    if yes("DoStage") {
                        stem += &format!("_s{s}");
                    }
                    if yes("DoTimelapse") {
                        stem += &format!("_t{t}");
                    }
                    list.push(find(dir, &stem));
                }
            }
            files.push(list);
        }
        // Geometry from the first data file.
        let first = files.iter().flatten().flatten().next().ok_or_else(|| Error::UnsupportedVariant("ND acquisition without data files".into()))?;
        let reader = Tiff::open(first, opts)?;
        let fm = reader.series().first().ok_or_else(|| Error::Corrupt("ND data file without image".into()))?.clone();
        let mut metas = Vec::new();
        for s in 1..=stages {
            let mut m = fm.clone();
            m.name = Some(kv.get(&format!("Stage{s}")).cloned().filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{base} position {s}")));
            m.size_c = waves as u32 * fm.size_c;
            m.size_t = times as u32;
            m.channels = names
                .iter()
                .flat_map(|n| (0..fm.size_c).map(move |_| ChannelMeta { name: (!n.is_empty()).then(|| n.clone()), ..Default::default() }))
                .collect();
            metas.push(m);
        }
        let mut original = OriginalMeta::default();
        for l in text.lines() {
            if let Some((k, v)) = l.split_once(',') {
                original.pairs.push((k.trim().trim_matches('"').to_string(), v.trim().trim_matches('"').to_string()));
            }
        }
        Ok(Nd { files, waves: waves as u32, file_c: fm.size_c.max(1), metas, original })
    }
}

impl Backend for Nd {
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
        let (w, c) = (plane.c / self.file_c, plane.c % self.file_c);
        let index = (w + self.waves * plane.t) as usize;
        let file = self.files[series].get(index).cloned().flatten();
        let reader = file.and_then(|f| Tiff::open(&f, &OpenOptions::default()).ok());
        let Some(reader) = reader else {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        reader.read_native(0, 0, Plane::new(c, plane.z, 0), region, sink)
    }
}

//! # metaxpress
//!
//! Molecular Devices MetaXpress plate reader (`.HTD` and its TIFF files).
//!
//! The `.HTD` file is text, one `"Key", value[, value…]` per line:
//! `XWells` / `YWells` (plate columns and rows), `WellsSelection<row>`
//! (one TRUE / FALSE per column), `Sites` with `XSites` / `YSites` and
//! `SiteSelection<row>`, `Waves` with `NWavelengths` and `WaveName<n>`,
//! `TimePoints`, `ZSeries` with `ZSteps`. The images are named
//! `<plate>_<well>[_s<site>][_w<wave>][_<GUID>].TIF` (thumbnails
//! `…_thumb…`), sites and waves counted from 1, in the plate folder or in
//! `TimePoint_<t>` folders below it, z steps in `ZStep_<z>` folders below
//! those (`ZStep_0` holds projections and is skipped when there are
//! others). The `.HTD` may sit in the plate folder or in a time point
//! folder. Each selected well and site is one series in row, column and
//! site order; channels are the waves. Pixel size, exposure and stage
//! position come from the MetaMorph metadata of the TIFF files. Missing
//! images read as zeros.
//!
//! Provenance: MetaXpress 6 batch import / export guide and MetaXpress
//! analysis guide (Molecular Devices, `doc/metaxpress-htd/`; file and
//! folder naming); own analysis of the public plates on the OME sample
//! server (IDR); checked against Bio-Formats and bioformats-rs as black
//! boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::tiff::Tiff;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, OpenOptions, OriginalMeta, Plane, PlaneMeta, PlateMeta, Region, Result, SeriesMeta, WellSample,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::MetaXpress, "Molecular Devices MetaXpress", "1").extensions(&["htd"]);

/// Most planes of one plate, against corrupt counts.
const MAX_PLANES: usize = 1 << 24;

/// Returns `true` for an HTD file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("htd")) && head.starts_with(b"\"HTSInfoFile\"")
}

/// `"Key", values…` lines, values unquoted.
fn parse(text: &str) -> HashMap<String, Vec<String>> {
    text.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(',')?;
            Some((k.trim().trim_matches('"').to_string(), v.split(',').map(|v| v.trim().trim_matches('"').to_string()).collect()))
        })
        .collect()
}

/// Well row and column (from 0) of a token such as `A01` or `AB12`.
fn well(token: &str) -> Option<(u32, u32)> {
    let letters = token.bytes().take_while(u8::is_ascii_uppercase).count();
    if letters == 0 || letters > 2 || letters == token.len() {
        return None;
    }
    let row = token[..letters].bytes().fold(0u32, |a, b| a * 26 + (b - b'A' + 1) as u32) - 1;
    let col: u32 = token[letters..].parse().ok().filter(|&c| c > 0)?;
    Some((row, col - 1))
}

/// Well, site and wave (from 0) of an image file name, `None` for
/// thumbnails and other files.
fn parse_name(name: &str, base: &str) -> Option<((u32, u32), u32, u32)> {
    let (stem, ext) = name.rsplit_once('.')?;
    if !matches!(ext.to_ascii_lowercase().as_str(), "tif" | "tiff") {
        return None;
    }
    let tokens: Vec<&str> = match stem.get(..base.len()).filter(|p| p.eq_ignore_ascii_case(base)).and_then(|_| stem[base.len()..].strip_prefix('_')) {
        Some(rest) => rest.split('_').collect(),
        // Renamed plate: tokens after the last well token.
        None => {
            let all: Vec<&str> = stem.split('_').collect();
            let at = all.iter().rposition(|t| well(t).is_some())?;
            all[at..].to_vec()
        }
    };
    let pos = well(tokens.first()?)?;
    let (mut site, mut wave) = (0, 0);
    for t in &tokens[1..] {
        let lower = t.to_ascii_lowercase();
        let number = |p: &str| lower.strip_prefix(p).and_then(|v| v.parse::<u32>().ok()).filter(|&v| v > 0).map(|v| v - 1);
        if lower.starts_with("thumb") {
            return None;
        } else if let Some(s) = number("s") {
            site = s;
        } else if let Some(w) = number("w") {
            wave = w;
        }
    }
    Some((pos, site, wave))
}

/// (well, site, wave, z, t) of an image file.
type ImageKey = ((u32, u32), u32, u32, u32, u32);

/// Subfolders `<prefix><n>` of `dir` by `n`, matched without regard to case.
fn numbered(dir: &Path, prefix: &str) -> BTreeMap<u32, PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else { return BTreeMap::new() };
    read.filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            Some((name.strip_prefix(prefix)?.parse().ok()?, e.path()))
        })
        .collect()
}

/// MetaXpress plate.
#[derive(Debug)]
pub(super) struct MetaXpress {
    /// Image files of each series by `wave + waves * (z + size_z * t)`.
    files: Vec<Vec<Option<PathBuf>>>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    original: OriginalMeta,
}

impl MetaXpress {
    /// Opens an HTD file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid HTD file and
    /// [`Error::UnsupportedVariant`] when no image file can be read.
    pub fn open(path: &Path, opts: &OpenOptions) -> Result<Self> {
        if std::fs::metadata(path)?.len() > 1 << 20 {
            return Err(Error::Corrupt("HTD file too large".into()));
        }
        // Descriptions may hold Latin-1 bytes.
        let raw = std::fs::read(path)?;
        let text: String = match std::str::from_utf8(&raw) {
            Ok(s) => s.to_string(),
            Err(_) => raw.iter().map(|&b| b as char).collect(),
        };
        let kv = parse(&text);
        let first = |k: &str| kv.get(k).and_then(|v| v.first());
        let yes = |k: &str| first(k).is_some_and(|v| v.eq_ignore_ascii_case("TRUE"));
        let count = |k: &str| first(k).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let (cols, rows) = (count("XWells"), count("YWells"));
        let base = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();

        // Image files below the plate folder.
        let dir = path.parent().unwrap_or(Path::new("."));
        let in_time_folder = dir.file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("timepoint_"));
        let root = if in_time_folder { dir.parent().unwrap_or(dir) } else { dir };
        let mut time_dirs = numbered(root, "timepoint_");
        if time_dirs.is_empty() {
            time_dirs.insert(1, root.to_path_buf());
        }
        // (well, site, wave, z, t) by file; z and t from 0.
        let mut found: HashMap<ImageKey, PathBuf> = HashMap::new();
        let (mut size_z, mut size_t) = (1u32, 1u32);
        for (ti, (_, tdir)) in time_dirs.iter().enumerate() {
            let mut z_dirs = numbered(tdir, "zstep_");
            if z_dirs.len() > 1 {
                z_dirs.remove(&0);
            }
            if z_dirs.is_empty() {
                z_dirs.insert(1, tdir.clone());
            }
            for (zi, (_, zdir)) in z_dirs.iter().enumerate() {
                let Ok(read) = std::fs::read_dir(zdir) else { continue };
                let mut names: Vec<String> = read.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
                names.sort();
                for name in names {
                    let Some((pos, site, wave)) = parse_name(&name, &base) else { continue };
                    found.entry((pos, site, wave, zi as u32, ti as u32)).or_insert_with(|| zdir.join(&name));
                }
                size_z = size_z.max(zi as u32 + 1);
            }
            size_t = size_t.max(ti as u32 + 1);
        }
        size_z = size_z.max(if yes("ZSeries") { count("ZSteps") } else { 1 });
        size_t = size_t.max(count("TimePoints").min(time_dirs.len() as u32).max(1));

        // Wells: the selected ones, else those with files.
        let mut wells: Vec<(u32, u32)> = Vec::new();
        for r in 0..rows.min(1 << 10) {
            if let Some(sel) = kv.get(&format!("WellsSelection{}", r + 1)) {
                wells.extend(sel.iter().enumerate().filter(|(_, v)| v.eq_ignore_ascii_case("TRUE")).map(|(c, _)| (r, c as u32)));
            }
        }
        if !kv.keys().any(|k| k.starts_with("WellsSelection")) {
            wells = found.keys().map(|k| k.0).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        }
        // Sites: the number selected, else all (or one without sites).
        let sites = if yes("Sites") {
            let selected: usize = (1..=count("YSites").min(1 << 10))
                .filter_map(|r| kv.get(&format!("SiteSelection{r}")))
                .map(|v| v.iter().filter(|v| v.eq_ignore_ascii_case("TRUE")).count())
                .sum();
            if selected > 0 { selected as u32 } else { count("XSites").max(1).saturating_mul(count("YSites").max(1)) }
        } else {
            1
        };
        let sites = sites.max(found.keys().map(|k| k.1 + 1).max().unwrap_or(1));
        let waves = if yes("Waves") { count("NWavelengths").max(1) } else { 1 };
        let waves = waves.max(found.keys().map(|k| k.2 + 1).max().unwrap_or(1));
        let per_series = (waves as usize).saturating_mul(size_z as usize).saturating_mul(size_t as usize);
        if per_series.saturating_mul(sites as usize).saturating_mul(wells.len()) > MAX_PLANES {
            return Err(Error::Corrupt("HTD plate with too many images".into()));
        }

        // Geometry from the first readable image file.
        let mut paths: Vec<&PathBuf> = found.values().collect();
        paths.sort();
        let fm = paths
            .iter()
            .find_map(|p| Tiff::open(p, opts).ok().and_then(|r| r.series().first().cloned()))
            .ok_or_else(|| Error::UnsupportedVariant("MetaXpress plate without readable image files".into()))?;
        if fm.size_c != 1 || fm.size_z != 1 || fm.size_t != 1 || fm.samples_per_pixel != 1 {
            return Err(Error::UnsupportedVariant("MetaXpress image files with several planes".into()));
        }
        let channels: Vec<ChannelMeta> = (1..=waves)
            .map(|w| ChannelMeta { name: first(&format!("WaveName{w}")).filter(|n| !n.is_empty()).cloned(), ..Default::default() })
            .collect();

        let mut files = Vec::new();
        let mut metas = Vec::new();
        let mut samples = Vec::new();
        for &(r, c) in &wells {
            for site in 0..sites {
                let mut list = vec![None; per_series];
                for t in 0..size_t {
                    for z in 0..size_z {
                        for w in 0..waves {
                            list[(w + waves * (z + size_z * t)) as usize] = found.get(&((r, c), site, w, z, t)).cloned();
                        }
                    }
                }
                let well_name = format!("{}{:02}", (b'A' + (r % 26) as u8) as char, c + 1);
                samples.push(WellSample { row: r, col: c, field: site, series: metas.len(), position: None });
                metas.push(SeriesMeta {
                    name: Some(format!("well {well_name} field {}", site + 1)),
                    size_c: waves,
                    size_z,
                    size_t,
                    channels: channels.clone(),
                    stage_position: None,
                    ..fm.clone()
                });
                files.push(list);
            }
        }
        let name = first("Description").filter(|d| !d.is_empty()).cloned().or_else(|| Some(base.clone()));
        let plates = vec![PlateMeta { name, rows, cols, wells: samples }];
        let mut original = OriginalMeta::default();
        for l in text.lines() {
            if let Some((k, v)) = l.split_once(',') {
                original.pairs.push((k.trim().trim_matches('"').to_string(), v.trim().trim_matches('"').to_string()));
            }
        }
        Ok(MetaXpress { files, metas, plates, original })
    }

    fn file(&self, series: usize, plane: Plane) -> Option<&PathBuf> {
        let m = &self.metas[series];
        self.files[series].get((plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize)?.as_ref()
    }
}

impl Backend for MetaXpress {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        // Exposure and stage position of the image file.
        let Some(reader) = self.file(series, plane).and_then(|f| Tiff::open(f, &OpenOptions::default()).ok()) else {
            return Ok(PlaneMeta::default());
        };
        reader.plane_meta(0, Plane::new(0, 0, 0))
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let reader = self.file(series, plane).and_then(|f| Tiff::open(f, &OpenOptions::default()).ok());
        let Some(reader) = reader else {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let im = reader.series().first().ok_or_else(|| Error::Corrupt("MetaXpress image file without image".into()))?;
        if (im.levels[0].size_x, im.levels[0].size_y, im.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt("MetaXpress image differs from the first image".into()));
        }
        reader.read_native(0, 0, Plane::new(0, 0, 0), region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_image_names() {
        assert_eq!(parse_name("plate 11001_A01_s10_w2.TIF", "plate 11001"), Some(((0, 0), 9, 1)));
        assert_eq!(parse_name("p_A01_s1_[A192FCC1-DC1A].tif", "p"), Some(((0, 0), 0, 0)));
        assert_eq!(parse_name("p_A01_s1_thumb_[AEF3].tif", "p"), None);
        assert_eq!(parse_name("BSF-1A_P24_w2.TIF", "BSF-1A"), Some(((15, 23), 0, 1)));
        assert_eq!(parse_name("renamed_B03_w1.tif", "other"), Some(((1, 2), 0, 0)));
        assert_eq!(parse_name("p.HTD", "p"), None);
        assert_eq!(well("AB12"), Some((27, 11)));
        assert_eq!(well("A0"), None);
    }
}

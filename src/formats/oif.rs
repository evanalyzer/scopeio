//! # oif
//!
//! Olympus FluoView OIB and OIF reader (FV1000/FV1200 confocal systems).
//!
//! An acquisition is a main settings file (`<name>.oif`) plus a folder of
//! members: one TIFF per plane named by its indices (`s_C001Z002T003.tif`),
//! a `.pty` property file per plane, LUTs, ROIs and a thumbnail. OIF keeps
//! them as files next to each other (`<name>.oif.files/`); OIB packs them
//! into one compound file whose `OibInfo.txt` maps file names to streams.
//! Settings files are INI text (UTF-16LE with byte-order mark, or UTF-8);
//! some numbers use a decimal comma (`0,207.0` is 0.207).
//!
//! The planes of the largest group of TIFF names with axis indices form the
//! series; lambda (`L`) bands are channels (channel-major); reference
//! images (`-R###`) and planes of other groups are not read. Missing planes
//! read as zeros. Pixel sizes come from the first plane's `.pty` (else the
//! main file), the z step from the `Interval` of the Z axis.
//!
//! Provenance: OpenReadout OIF notes (MIT OR Apache-2.0) in
//! `doc/olympus-oif-oib/`; `oiffile` (BSD-3-Clause, Christoph Gohlke, see
//! THIRD_PARTY_NOTICES.md): container mapping and plane name convention;
//! Microsoft [MS-CFB]; own analysis of the OME sample file; checked against
//! Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::cfb::{self, Cfb};
use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::io::Source;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane,
    Region, Result, SeriesMeta,
};

pub(super) static OIB_FORMAT: FormatInfo = FormatInfo::new(FormatId::Oib, "Olympus FluoView OIB", "1");
pub(super) static OIF_FORMAT: FormatInfo = FormatInfo::new(FormatId::Oif, "Olympus FluoView OIF", "1");

/// Largest settings file parsed.
const MAX_SETTINGS: u64 = 16 << 20;

/// Returns `true` for an OIF main settings file: `.oif` and settings text.
pub(super) fn matches_oif(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("oif")) && (head.starts_with(b"\xFF\xFE[\0") || head.starts_with(b"["))
}

/// Returns `true` for an OIB file: a compound file with `OibInfo.txt`.
pub(super) fn matches_oib(path: &Path, head: &[u8]) -> bool {
    cfb::matches(head) && Cfb::open(path).is_ok_and(|c| c.stream("OibInfo.txt").is_some())
}

// --- Settings text ----------------------------------------------------------

/// A parsed settings file: sections of `key=value` entries in file order.
#[derive(Debug, Default)]
struct Settings {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Settings {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let text = if let Some(b) = bytes.strip_prefix(b"\xFF\xFE") {
            let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        } else if bytes.starts_with(b"[") {
            String::from_utf8_lossy(bytes).into_owned()
        } else {
            return Err(Error::Corrupt("not a FluoView settings file".into()));
        };
        let mut s = Settings::default();
        for line in text.lines() {
            let line = line.trim();
            if line == "[ColorLUTData]" {
                // A binary LUT follows.
                break;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                s.sections.push((name.to_string(), Vec::new()));
            } else if let (Some((k, v)), Some(sec)) = (line.split_once('='), s.sections.last_mut()) {
                sec.1.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        Ok(s)
    }

    fn raw(&self, section: &str, key: &str) -> Option<&str> {
        let (_, entries) = self.sections.iter().find(|(n, _)| n == section)?;
        entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Unquoted text; empty text is `None`.
    fn text(&self, section: &str, key: &str) -> Option<String> {
        let v = self.raw(section, key)?;
        let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\''))).unwrap_or(v);
        (!v.is_empty()).then(|| v.to_string())
    }

    /// A number; `<int>,<frac>[.0]` is read as `<int>.<frac>`.
    fn number(&self, section: &str, key: &str) -> Option<f64> {
        number(&self.text(section, key)?)
    }

    /// The section `Axis n Parameters Common` whose `AxisCode` is `code`.
    fn axis(&self, code: &str) -> Option<String> {
        (0..16).map(|i| format!("Axis {i} Parameters Common")).find(|s| self.text(s, "AxisCode").as_deref() == Some(code))
    }
}

fn number(v: &str) -> Option<f64> {
    if let Some((int, frac)) = v.split_once(',') {
        let frac = frac.split_once('.').map_or(frac, |(f, _)| f);
        return format!("{int}.{frac}").parse().ok();
    }
    v.parse().ok()
}

/// Nanometres of `value` in `unit`.
fn nm(value: f64, unit: &str) -> Option<f64> {
    let f = match unit {
        "nm" => 1.0,
        "um" | "µm" => 1e3,
        "mm" => 1e6,
        _ => return None,
    };
    (value.is_finite() && value > 0.0).then_some(value * f)
}

// --- Container --------------------------------------------------------------

/// Where the members of the acquisition live.
#[derive(Debug)]
enum Store {
    /// Compound file and the stream path of each member (lower-case name).
    Oib(Cfb, HashMap<String, String>),
    /// Member files by lower-case name.
    Oif(HashMap<String, PathBuf>),
}

impl Store {
    fn names(&self) -> Vec<String> {
        match self {
            Store::Oib(_, m) => m.keys().cloned().collect(),
            Store::Oif(m) => m.keys().cloned().collect(),
        }
    }

    fn read(&self, name: &str) -> Result<Vec<u8>> {
        let missing = || Error::Corrupt(format!("FluoView member {name} missing"));
        match self {
            Store::Oib(c, m) => {
                let stream = c.stream(m.get(&name.to_lowercase()).ok_or_else(missing)?).ok_or_else(missing)?;
                c.read(stream)
            }
            Store::Oif(m) => {
                let path = m.get(&name.to_lowercase()).ok_or_else(missing)?;
                if std::fs::metadata(path)?.len() > 1 << 31 {
                    return Err(Error::Corrupt(format!("FluoView member {name} too large")));
                }
                Ok(std::fs::read(path)?)
            }
        }
    }

    fn settings(&self, name: &str) -> Result<Settings> {
        let b = self.read(name)?;
        if b.len() as u64 > MAX_SETTINGS {
            return Err(Error::Corrupt(format!("FluoView settings {name} too large")));
        }
        Settings::parse(&b)
    }

    /// The TIFF of a plane member.
    fn tiff(&self, name: &str) -> Result<TiffFile> {
        match self {
            Store::Oib(..) => TiffFile::from_source(Source::Memory(Arc::from(self.read(name)?)), 4),
            Store::Oif(m) => {
                let path = m.get(&name.to_lowercase()).ok_or_else(|| Error::Corrupt(format!("FluoView member {name} missing")))?;
                TiffFile::from_file(std::fs::File::open(path)?, 4)
            }
        }
    }
}

/// A plane member: its name and (axis letter, number) pairs.
type PlaneFile = (String, Vec<(char, u32)>);

/// Plane indices from a member name `<prefix>_C001Z002T003.tif`: the
/// prefix and (axis letter, number) pairs; `None` for other names and
/// reference images (`-R###`).
fn plane_name(name: &str) -> Option<(String, Vec<(char, u32)>)> {
    let stem = name.strip_suffix(".tif").or_else(|| name.strip_suffix(".TIF"))?;
    let (prefix, axes) = stem.rsplit_once('_')?;
    // Member names are compared without case.
    let axes = axes.to_ascii_uppercase();
    let mut out = Vec::new();
    let mut chars = axes.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        let mut digits = String::new();
        while let Some(d) = chars.next_if(|d| d.is_ascii_digit()) {
            digits.push(d);
        }
        if digits.is_empty() || out.iter().any(|(a, _)| *a == c) {
            return None;
        }
        out.push((c, digits.parse().ok()?));
    }
    (!out.is_empty()).then(|| (prefix.to_string(), out))
}

/// FluoView OIB or OIF acquisition.
#[derive(Debug)]
pub(super) struct Oif {
    format: &'static FormatInfo,
    store: Store,
    /// Plane member by index `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<String>>,
    metas: [SeriesMeta; 1],
    original: OriginalMeta,
}

impl Oif {
    /// Opens an OIB file or an OIF main settings file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for missing members or planes and
    /// [`Error::UnsupportedVariant`] for planes that are not gray images.
    pub fn open(path: &Path, id: FormatId) -> Result<Self> {
        let (format, store, main_name) = if id == FormatId::Oib {
            let c = Cfb::open(path)?;
            let info_stream = c.stream("OibInfo.txt").ok_or_else(|| Error::Corrupt("OIB without OibInfo.txt".into()))?.clone();
            let info = Settings::parse(&c.read(&info_stream)?)?;
            let section = "OibSaveInfo";
            let main_stream = info.text(section, "MainFileName").ok_or_else(|| Error::Corrupt("OIB without main file".into()))?;
            let mut members = HashMap::new();
            let mut main_name = None;
            for (k, v) in info.sections.iter().filter(|(n, _)| n == section).flat_map(|(_, e)| e) {
                if !k.starts_with("Stream") {
                    continue;
                }
                // `StreamNNNNN=Storage00001/<file>` lives at `Storage00001/StreamNNNNN`.
                let (folder, file) = v.rsplit_once('/').unwrap_or(("", v.as_str()));
                let stream = if folder.is_empty() { k.clone() } else { format!("{folder}/{k}") };
                if *k == main_stream {
                    main_name = Some(file.to_string());
                }
                members.insert(file.to_lowercase(), stream);
            }
            let main_name = main_name.ok_or_else(|| Error::Corrupt("OIB main file not listed".into()))?;
            (&OIB_FORMAT, Store::Oib(c, members), main_name)
        } else {
            let main_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let folder = path.with_file_name(format!("{main_name}.files"));
            let mut members = HashMap::new();
            members.insert(main_name.to_lowercase(), path.to_path_buf());
            for e in std::fs::read_dir(&folder).map_err(|_| Error::Corrupt(format!("OIF folder {} missing", folder.display())))? {
                let e = e?;
                members.insert(e.file_name().to_string_lossy().to_lowercase(), e.path());
            }
            (&OIF_FORMAT, Store::Oif(members), main_name)
        };
        let main = store.settings(&main_name)?;

        // Plane groups by prefix and axis letters; the largest is the series.
        let mut groups: BTreeMap<(String, Vec<char>), Vec<PlaneFile>> = BTreeMap::new();
        for name in store.names() {
            if let Some((prefix, axes)) = plane_name(&name) {
                let letters = axes.iter().map(|(a, _)| *a).collect();
                groups.entry((prefix, letters)).or_default().push((name, axes));
            }
        }
        let files = groups.into_values().max_by_key(|g| g.len()).ok_or_else(|| Error::Corrupt("FluoView without image planes".into()))?;
        let numbers = |axis: char| -> Vec<u32> {
            let set: BTreeSet<u32> = files.iter().filter_map(|(_, a)| a.iter().find(|(c, _)| *c == axis).map(|(_, n)| *n)).collect();
            if set.is_empty() { vec![1] } else { set.into_iter().collect() }
        };
        let (cs, ls, zs, ts) = (numbers('C'), numbers('L'), numbers('Z'), numbers('T'));
        let (size_c, size_z, size_t) = (cs.len() * ls.len(), zs.len(), ts.len());
        let mut planes = vec![None; size_c * size_z * size_t];
        let ord = |list: &[u32], axes: &[(char, u32)], axis: char| -> Option<usize> {
            let n = axes.iter().find(|(c, _)| *c == axis).map_or(1, |(_, n)| *n);
            list.iter().position(|&v| v == n)
        };
        for (name, axes) in &files {
            if axes.iter().any(|(c, n)| !matches!(c, 'C' | 'L' | 'Z' | 'T') && *n != 1) {
                continue;
            }
            let (Some(c), Some(l), Some(z), Some(t)) = (ord(&cs, axes, 'C'), ord(&ls, axes, 'L'), ord(&zs, axes, 'Z'), ord(&ts, axes, 'T')) else { continue };
            let i = c * ls.len() + l + size_c * (z + size_z * t);
            planes[i].get_or_insert_with(|| name.clone());
        }
        // Geometry from the first plane.
        let first = planes.iter().flatten().next().ok_or_else(|| Error::Corrupt("FluoView without image planes".into()))?.clone();
        let tiff = store.tiff(&first)?;
        let img = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if img.samples_out != 1 || img.spp != 1 {
            return Err(Error::UnsupportedVariant("FluoView planes with several samples".into()));
        }
        let pty = store.settings(&format!("{}.pty", first.trim_end_matches(".tif"))).ok();
        let size = |s: &Settings, sec: &str| -> (Option<f64>, Option<f64>) {
            let u = |k: &str| s.text(sec, k).unwrap_or_default();
            (
                s.number(sec, "WidthConvertValue").and_then(|v| nm(v, &u("WidthUnit"))),
                s.number(sec, "HeightConvertValue").and_then(|v| nm(v, &u("HeightUnit"))),
            )
        };
        let (mut x_nm, mut y_nm) = pty.as_ref().map_or((None, None), |p| size(p, "Image Parameters"));
        if x_nm.is_none() && y_nm.is_none() {
            (x_nm, y_nm) = size(&main, "Reference Image Parameter");
        }
        let z_nm = main.axis("Z").filter(|_| size_z > 1).and_then(|sec| {
            let unit = main.text(&sec, "PixUnit").unwrap_or_default();
            main.number(&sec, "Interval").and_then(|v| nm(v.abs(), &unit))
        });
        let channels = cs
            .iter()
            .flat_map(|c| ls.iter().map(move |l| (*c, *l)))
            .map(|(c, _l)| {
                let sec = format!("Channel {c} Parameters");
                ChannelMeta {
                    name: main.text(&sec, "CH Name"),
                    fluor: main.text(&sec, "DyeName").filter(|d| d != "None"),
                    excitation_wavelength_nm: main.number(&sec, "ExcitationWavelength").filter(|v| *v > 0.0),
                    emission_wavelength_nm: main.number(&sec, "EmissionWavelength").filter(|v| *v > 0.0),
                    contrast_method: main.text(&sec, "LightType"),
                    ..Default::default()
                }
            })
            .collect();
        let objective = pty.as_ref().and_then(|p| {
            let sec = "Acquisition Parameters Common";
            let o = ObjectiveMeta {
                model: p.text(sec, "ObjectiveLens Name").map(|n| n.split_whitespace().collect::<Vec<_>>().join(" ")),
                magnification: p.number(sec, "Magnification").filter(|v| *v > 0.0 && *v < 9999.0),
                numerical_aperture: p.number(sec, "ObjectiveLens NAValue").filter(|v| *v > 0.0 && *v < 9999.0),
                ..Default::default()
            };
            (o != ObjectiveMeta::default()).then_some(o)
        });
        let date = main.text("Acquisition Parameters Common", "ImageCaputreDate").map(|d| d.replacen(' ', "T", 1));
        let bits = pty.as_ref().and_then(|p| p.number("Image Parameters", "ValidBitCounts")).or_else(|| main.number("Reference Image Parameter", "ValidBitCounts"));
        let meta = SeriesMeta {
            name: main.text("File Info", "DataName"),
            acquisition_date: date,
            size_c: size_c as u32,
            size_z: size_z as u32,
            size_t: size_t as u32,
            pixel_type: img.pixel_type,
            significant_bits: bits.filter(|b| *b > 0.0 && *b < img.pixel_type.bytes() as f64 * 8.0).map(|b| b as u8),
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: img.width as u64, size_y: img.height as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm, y_nm, z_nm },
            channels,
            objective,
            ..Default::default()
        };
        let mut original = OriginalMeta::default();
        for (sec, entries) in &main.sections {
            for (k, v) in entries {
                original.pairs.push((format!("[{sec}] {k}"), v.clone()));
            }
        }
        Ok(Oif { format, store, planes, metas: [meta], original })
    }
}

impl Backend for Oif {
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
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let Some(name) = &self.planes[index] else {
            // Missing plane: zeros.
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let tiff = self.store.tiff(name)?;
        let img = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if (img.width as u64, img.height as u64, img.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt(format!("FluoView plane {name} differs from the first plane")));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_numbers_and_settings() {
        assert_eq!(plane_name("s_C001Z002T003.tif"), Some(("s".into(), vec![('C', 1), ('Z', 2), ('T', 3)])));
        assert_eq!(plane_name("s_C001-R001.tif"), None);
        assert_eq!(plane_name("s_Thumb.tif"), None);
        assert_eq!(plane_name("s_c001z002.tif"), Some(("s".into(), vec![('C', 1), ('Z', 2)])));
        assert_eq!(number("0,207.0"), Some(0.207));
        assert_eq!(number("211,761.0"), Some(211.761));
        assert_eq!(number("510"), Some(510.0));
        let text: Vec<u8> = "\u{feff}[A]\r\nX=\"um\"\r\nN=1,35.0\r\n".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let s = Settings::parse(&text).unwrap();
        assert_eq!((s.text("A", "X").as_deref(), s.number("A", "N")), (Some("um"), Some(1.35)));
    }
}

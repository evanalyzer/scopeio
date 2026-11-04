//! # cv7000
//!
//! Yokogawa CellVoyager CV7000 / CV8000 reader (`.wpi` and its TIFF files).
//!
//! A measurement folder holds XML index files in the `bts` namespace:
//! the plate (`<plate>.wpi`, `WellPlate` with rows and columns),
//! `MeasurementDetail.mrf` (`MeasurementDetail` with the field, z and time
//! point counts and begin time, one `MeasurementChannel` per channel
//! number with the pixel size and image size, the setting file name),
//! the setting file (`.mes`, `ChannelList` with target, objective,
//! filter, exposure, binning, light sources and colour) and
//! `MeasurementData.mlf` with one `MeasurementRecord` per image: `Type`
//! `IMG` (others are errors), `Row` / `Column` / `FieldIndex` /
//! `TimePoint` / `ZIndex` / `TimelineIndex` / `ActionIndex` (all from 1),
//! channel number `Ch`, stage `X` / `Y` / `Z` in µm, `Time` and the TIFF
//! file name as text. Each (well, field) is one series in row, column and
//! field order; a well is listed when one of its image files exists and
//! then has every field of the measurement. Channels are the distinct
//! (timeline, action, channel number) triples; an action may take several
//! channels at once. A channel imaged in 2-D within a z stack (only
//! `ZIndex` 1, e.g. fluorescence next to a brightfield stack) serves its
//! plane at every z. Missing images read as zeros.
//!
//! Provenance: CellVoyager CV7000 user guide (Yokogawa / NCI,
//! `doc/yokogawa-cv/`); own analysis of the public CV7000 / CV8000 plates
//! on the OME sample server (idr0088, idr0093, cpg0016); checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use super::tiff::ifd::TiffFile;
use super::tiff::image::{Samples, TiffImage};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, Plane, PlaneMeta,
    PlateMeta, Position, Region, Result, SeriesMeta, WellSample,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Cv7000, "Yokogawa CellVoyager CV7000 / CV8000", "1")
    .extensions(&["wpi"])
    .file_names(&["MeasurementData.mlf", "MeasurementDetail.mrf"]);

/// Largest index file parsed.
const MAX_INDEX: u64 = 256 << 20;

/// Returns `true` for a CV7000 plate or measurement index file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let text = String::from_utf8_lossy(head);
    (name.ends_with(".wpi") && text.contains("WellPlate") || name.ends_with(".mlf") && text.contains("MeasurementData") || name.ends_with(".mrf") && text.contains("MeasurementDetail"))
        && text.contains("yokogawa.co.jp/BTS")
}

/// Reads an index file as text, without a byte order mark.
fn read_text(path: &Path) -> Result<String> {
    if std::fs::metadata(path)?.len() > MAX_INDEX {
        return Err(Error::Corrupt(format!("CV7000 index {} too large", path.display())));
    }
    let raw = std::fs::read(path)?;
    let text = match String::from_utf8(raw) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    };
    Ok(text.strip_prefix('\u{feff}').map(String::from).unwrap_or(text))
}

fn parse<'a>(text: &'a str, what: &str) -> Result<Document<'a>> {
    Document::parse(text).map_err(|e| Error::Corrupt(format!("CV7000 {what}: {e}")))
}

fn attr<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.name() == name).map(|a| a.value())
}

fn num(n: Node, name: &str) -> Option<f64> {
    attr(n, name)?.trim().parse().ok().filter(|v: &f64| v.is_finite())
}

fn index(n: Node, name: &str) -> u32 {
    num(n, name).map_or(1, |v| v.clamp(1.0, u32::MAX as f64) as u32)
}

fn elements<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.descendants().filter(move |c| c.tag_name().name() == name)
}

/// Seconds since 1970 of an ISO 8601 time `YYYY-MM-DDThh:mm:ss[.f][Z|±hh:mm]`.
fn iso_seconds(s: &str) -> Option<f64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.splitn(3, '-').map(|v| v.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let split = time.find(['Z', '+']).or_else(|| time.rfind('-')).unwrap_or(time.len());
    let (clock, zone) = time.split_at(split);
    let mut c = clock.splitn(3, ':');
    let (h, min, sec): (f64, f64, f64) = (c.next()?.parse().ok()?, c.next()?.parse().ok()?, c.next()?.parse().ok()?);
    let offset = match zone.as_bytes().first() {
        Some(b'+' | b'-') => {
            let (oh, om) = zone[1..].split_once(':').unwrap_or((&zone[1..], "0"));
            let v = oh.parse::<f64>().ok()? * 3600.0 + om.parse::<f64>().ok()? * 60.0;
            if zone.starts_with('-') { -v } else { v }
        }
        _ => 0.0,
    };
    // Days from civil (proleptic Gregorian).
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Some(days as f64 * 86400.0 + h * 3600.0 + min * 60.0 + sec - offset)
}

/// ISO 8601 time without fraction and zone, as written by the instrument.
fn local_time(s: &str) -> String {
    let clock = s.find('T').map_or(0, |t| t + 1);
    let end = s[clock..].find(['.', 'Z', '+', '-']).map_or(s.len(), |e| clock + e);
    s[..end].to_string()
}

/// `#AARRGGBB` as RGBA.
fn colour(s: &str) -> Option<[u8; 4]> {
    let v = u32::from_str_radix(s.strip_prefix('#')?, 16).ok().filter(|_| s.len() == 9)?;
    let [a, r, g, b] = v.to_be_bytes();
    Some([r, g, b, a])
}

/// One image record.
#[derive(Debug, Clone)]
struct Record {
    file: PathBuf,
    channel: (u32, u32, u32),
    z: u32,
    t: u32,
    time_s: Option<f64>,
    position: Position,
}

#[derive(Debug)]
struct Series {
    /// Image files by `c + size_c * (z + size_z * t)`.
    planes: Vec<Option<PathBuf>>,
    plane_meta: Vec<PlaneMeta>,
}

/// CV7000 measurement.
#[derive(Debug)]
pub(super) struct Cv7000 {
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
    original: OriginalMeta,
}

impl Cv7000 {
    /// Opens a `.wpi`, `MeasurementData.mlf` or `MeasurementDetail.mrf` file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or no image records and
    /// [`Error::UnsupportedVariant`] when no image file can be read.
    pub fn open(path: &Path) -> Result<Self> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let mut original = OriginalMeta::default();

        // Plate geometry and name.
        let wpi = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("wpi")) {
            Some(path.to_path_buf())
        } else {
            std::fs::read_dir(dir).ok().and_then(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wpi"))))
        };
        let (mut rows, mut cols, mut plate_name) = (0, 0, None);
        if let Some(wpi) = &wpi {
            let text = read_text(wpi)?;
            let doc = parse(&text, "plate")?;
            let root = doc.root_element();
            rows = num(root, "Rows").map_or(0, |v| v.max(0.0) as u32);
            cols = num(root, "Columns").map_or(0, |v| v.max(0.0) as u32);
            plate_name = attr(root, "Name").filter(|s| !s.is_empty()).map(String::from);
        }

        // Measurement detail: counts, channel pixel sizes, setting file.
        let (mut field_count, mut z_count, mut t_count, mut begin, mut setting) = (1, 1, 1, None, None);
        let mut pixel_um: HashMap<u32, (Option<f64>, Option<f64>)> = HashMap::new();
        if let Ok(text) = read_text(&dir.join("MeasurementDetail.mrf")) {
            let doc = parse(&text, "measurement detail")?;
            let root = doc.root_element();
            field_count = index(root, "FieldCount");
            z_count = index(root, "ZCount");
            t_count = index(root, "TimePointCount");
            begin = attr(root, "BeginTime").map(local_time);
            setting = attr(root, "MeasurementSettingFileName").and_then(|s| Path::new(&s.replace('\\', "/")).file_name().map(|f| dir.join(f)));
            for a in root.attributes() {
                original.pairs.push((a.name().to_string(), a.value().to_string()));
            }
            rows = rows.max(num(root, "RowCount").map_or(0, |v| v.max(0.0) as u32));
            cols = cols.max(num(root, "ColumnCount").map_or(0, |v| v.max(0.0) as u32));
            for c in elements(root, "MeasurementChannel") {
                let size = |a: &str| num(c, a).filter(|v| *v > 0.0);
                pixel_um.insert(index(c, "Ch"), (size("HorizontalPixelDimension"), size("VerticalPixelDimension")));
            }
        }

        // Measurement setting: channel descriptions by channel number.
        let mut described: HashMap<u32, ChannelMeta> = HashMap::new();
        let mut objective = None;
        if let Some(text) = setting.as_deref().and_then(|p| read_text(p).ok()) {
            let doc = parse(&text, "measurement setting")?;
            let root = doc.root_element();
            let lasers: HashMap<&str, f64> = elements(root, "LightSource").filter_map(|l| Some((attr(l, "Name")?, num(l, "WaveLength").filter(|v| *v > 0.0)?))).collect();
            for c in elements(root, "Channel").filter(|c| attr(*c, "Ch").is_some()) {
                let binning = num(c, "Binning").filter(|v| *v >= 1.0 && *v <= u16::MAX as f64).map(|v| (v as u16, v as u16));
                // Band-pass emission filter "BP<centre>/<width>".
                let emission = attr(c, "Acquisition").and_then(|a| a.strip_prefix("BP")).and_then(|a| a.split('/').next()?.trim().parse().ok());
                let excitation = c.children().filter(|n| n.tag_name().name() == "LightSourceName").find_map(|n| lasers.get(n.text()?.trim()).copied());
                described.insert(index(c, "Ch"), ChannelMeta {
                    name: attr(c, "Target").filter(|s| !s.is_empty()).map(String::from),
                    color: attr(c, "Color").and_then(colour),
                    excitation_wavelength_nm: excitation,
                    emission_wavelength_nm: emission,
                    contrast_method: attr(c, "Kind").filter(|s| !s.is_empty()).map(String::from),
                    pinhole_size_nm: num(c, "PinholeDiameter").filter(|v| *v > 0.0).map(|um| um * 1e3),
                    exposure_s: num(c, "ExposureTime").filter(|v| *v > 0.0).map(|ms| ms / 1e3),
                    binning,
                    ..Default::default()
                });
                objective = objective.or_else(|| {
                    Some(ObjectiveMeta {
                        model: attr(c, "Objective").filter(|s| !s.is_empty()).map(String::from),
                        magnification: num(c, "Magnification").filter(|v| *v > 0.0),
                        ..Default::default()
                    })
                });
            }
        }

        // Image records by (row, column, field).
        let text = read_text(&dir.join("MeasurementData.mlf"))?;
        let doc = parse(&text, "measurement data")?;
        let mut groups: BTreeMap<(u32, u32, u32), Vec<Record>> = BTreeMap::new();
        for r in elements(doc.root_element(), "MeasurementRecord") {
            if attr(r, "Type") != Some("IMG") {
                continue;
            }
            // Only the file name: never leave the measurement folder.
            let Some(name) = r.text().and_then(|t| Path::new(&t.trim().replace('\\', "/")).file_name().map(|f| f.to_os_string())) else { continue };
            let um = |a: &str| num(r, a).map(|v| v * 1e3);
            let record = Record {
                file: dir.join(name),
                channel: (index(r, "TimelineIndex"), index(r, "ActionIndex"), index(r, "Ch")),
                z: index(r, "ZIndex"),
                t: index(r, "TimePoint"),
                time_s: attr(r, "Time").and_then(iso_seconds),
                position: Position { x_nm: um("X"), y_nm: um("Y"), z_nm: um("Z") },
            };
            groups.entry((index(r, "Row"), index(r, "Column"), index(r, "FieldIndex"))).or_default().push(record);
        }
        if groups.is_empty() {
            return Err(Error::Corrupt("CV7000 measurement without image records".into()));
        }
        let records = || groups.values().flatten();
        // Geometry and sample type from the first readable image file.
        let (pixel_type, width, height) = records()
            .find_map(|e| {
                let tiff = TiffFile::open_first(&e.file).ok()?;
                let img = TiffImage::new(&tiff, &tiff.ifds[0]).ok()?;
                (img.samples_out == 1).then_some((img.pixel_type, img.width as u64, img.height as u64))
            })
            .ok_or_else(|| Error::UnsupportedVariant("CV7000 measurement without readable image files".into()))?;
        let channels: Vec<(u32, u32, u32)> = records().map(|e| e.channel).collect::<BTreeSet<_>>().into_iter().collect();
        let size_c = channels.len();
        let size_z = records().map(|e| e.z).max().unwrap_or(1).max(z_count) as usize;
        let size_t = records().map(|e| e.t).max().unwrap_or(1).max(t_count) as usize;
        let flat: Vec<bool> = channels.iter().map(|&ch| records().filter(|e| e.channel == ch).all(|e| e.z == 1)).collect();
        let fields = groups.keys().map(|k| k.2).max().unwrap_or(1).max(field_count);
        let n = size_c.checked_mul(size_z).and_then(|v| v.checked_mul(size_t)).filter(|&v| v.saturating_mul(groups.len()) <= 1 << 26);
        let n = n.ok_or_else(|| Error::Corrupt("CV7000 measurement with too many planes".into()))?;

        // Wells with at least one image file on disk.
        let wells: BTreeSet<(u32, u32)> = groups.iter().filter(|(_, rs)| rs.iter().any(|r| r.file.is_file())).map(|(k, _)| (k.0, k.1)).collect();
        let channel_meta: Vec<ChannelMeta> = channels.iter().map(|&(_, _, ch)| described.get(&ch).cloned().unwrap_or_else(|| ChannelMeta { name: Some(format!("Ch{ch}")), ..Default::default() })).collect();
        let (x_um, y_um) = channels.iter().find_map(|(_, _, ch)| pixel_um.get(ch).copied()).unwrap_or((None, None));
        // Z step from the stage positions of the first stack.
        let z_step = groups.values().find_map(|rs| {
            let second = rs.iter().find(|r| r.z == 2)?;
            let first = rs.iter().find(|r| r.z == 1 && r.channel == second.channel && r.t == second.t)?;
            Some((second.position.z_nm? - first.position.z_nm?).abs()).filter(|v| *v > 0.0)
        });

        let mut series = Vec::new();
        let mut metas = Vec::new();
        let mut samples = Vec::new();
        let empty = Vec::new();
        for &(row, col) in &wells {
            for field in 1..=fields {
                let rs = groups.get(&(row, col, field)).unwrap_or(&empty);
                let mut s = Series { planes: vec![None; n], plane_meta: vec![PlaneMeta::default(); n] };
                let t0 = rs.iter().filter_map(|r| r.time_s).reduce(f64::min);
                for r in rs {
                    let Ok(c) = channels.binary_search(&r.channel) else { continue };
                    let i = c + size_c * (r.z as usize - 1 + size_z * (r.t as usize - 1));
                    s.planes[i].get_or_insert_with(|| r.file.clone());
                    s.plane_meta[i] = PlaneMeta {
                        delta_t_s: r.time_s.zip(t0).map(|(v, t0)| v - t0),
                        exposure_s: channel_meta[c].exposure_s,
                        position: Some(r.position),
                    };
                }
                // 2-D channels within a z stack: their plane at every z.
                for c in (0..size_c).filter(|&c| flat[c]) {
                    for t in 0..size_t {
                        let first = c + size_c * size_z * t;
                        for z in 1..size_z {
                            let i = c + size_c * (z + size_z * t);
                            if s.planes[i].is_none() {
                                s.planes[i] = s.planes[first].clone();
                                s.plane_meta[i] = s.plane_meta[first].clone();
                            }
                        }
                    }
                }
                let position = rs.first().map(|r| Position { z_nm: None, ..r.position });
                let well = format!("{}{:02}", (b'A' + (row.saturating_sub(1) % 26) as u8) as char, col);
                samples.push(WellSample { row: row - 1, col: col - 1, field: field - 1, series: metas.len(), position });
                metas.push(SeriesMeta {
                    name: Some(format!("well {well} field {field}")),
                    acquisition_date: begin.clone(),
                    size_c: size_c as u32,
                    size_z: size_z as u32,
                    size_t: size_t as u32,
                    pixel_type,
                    samples_per_pixel: 1,
                    levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
                    physical_size: PhysicalSize { x_nm: x_um.map(|v| v * 1e3), y_nm: y_um.map(|v| v * 1e3), z_nm: z_step.filter(|_| size_z > 1) },
                    channels: channel_meta.clone(),
                    objective: objective.clone(),
                    stage_position: position,
                    ..Default::default()
                });
                series.push(s);
            }
        }
        let plates = vec![PlateMeta { name: plate_name, rows, cols, wells: samples }];
        Ok(Cv7000 { series, metas, plates, original })
    }
}

impl Backend for Cv7000 {
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
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        Ok(self.series[series].plane_meta.get(i).cloned().unwrap_or_default())
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let i = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let zeros = |sink: &mut Sink| -> Result<()> {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            Ok(())
        };
        let Some(file) = &self.series[series].planes[i] else { return zeros(sink) };
        let Ok(tiff) = TiffFile::open_first(file) else { return zeros(sink) };
        let img = TiffImage::new(&tiff, &tiff.ifds[0])?;
        if (img.width as u64, img.height as u64, img.pixel_type) != (m.levels[0].size_x, m.levels[0].size_y, m.pixel_type) {
            return Err(Error::Corrupt(format!("CV7000 image {} differs from the first image", file.display())));
        }
        img.read_native(&tiff, region, Samples::One(0), sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_times_and_colours() {
        let a = iso_seconds("2017-09-16T15:09:27.897+02:00").unwrap();
        let b = iso_seconds("2017-09-16T13:09:28.897Z").unwrap();
        assert!((b - a - 1.0).abs() < 1e-6);
        assert_eq!(iso_seconds("1970-01-02T00:00:00").unwrap(), 86400.0);
        assert_eq!(local_time("2017-09-16T15:09:05.5083628+02:00"), "2017-09-16T15:09:05");
        assert_eq!(local_time("2017-09-16T15:09:05-05:00"), "2017-09-16T15:09:05");
        assert_eq!(colour("#FF002FFF"), Some([0x00, 0x2f, 0xff, 0xff]));
        assert_eq!(colour("#12"), None);
    }
}

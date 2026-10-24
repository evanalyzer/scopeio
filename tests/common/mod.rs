//! # common
//!
//! Helpers shared by the format tests: loading reference results generated
//! by `tests/scripts/oracle.py` and comparing scopeio's output with them.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#![allow(dead_code)]

pub mod crc32;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use scopeio::{ChannelSelect, ImageReader, OutputSpec, Plane, Region, RegionRequest};

/// Repository root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Reads a tab-separated file with a header line into one map per row.
pub fn read_tsv(path: &Path) -> Vec<HashMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines().filter(|l| !l.is_empty() && !l.starts_with('#'));
    let header: Vec<String> = lines.next().expect("header line").split('\t').map(String::from).collect();
    lines
        .map(|l| header.iter().cloned().zip(l.split('\t').map(String::from)).collect())
        .collect()
}

/// Pixel data of one series in the layout of the reference ICS output:
/// samples of all channels interleaved per pixel, then x, y, z, t.
pub struct Series {
    pub width: u64,
    pub height: u64,
    pub size_z: u32,
    pub size_t: u32,
    pub channels: usize,
    pub pixel_type: String,
    pub phys_x_nm: Option<f64>,
    pub phys_y_nm: Option<f64>,
    pub pixels: Vec<u8>,
}

/// Decodes every series of a file at full resolution.
pub fn decode(reader: &dyn ImageReader) -> scopeio::Result<Vec<Series>> {
    (0..reader.series_count()).map(|s| decode_level(reader, s, 0)).collect()
}

/// All samples of all channels, alpha included, interleaved per pixel.
fn all_channels(size_c: u32) -> (ChannelSelect, OutputSpec) {
    (ChannelSelect::Channels((0..size_c).collect()), OutputSpec { keep_alpha: true, ..Default::default() })
}

/// Native bytes of channel `plane.c` (all samples, alpha kept) of `region`.
pub fn read(reader: &dyn ImageReader, series: usize, level: usize, plane: Plane, region: Region) -> scopeio::Result<Vec<u8>> {
    let req = RegionRequest {
        series,
        level,
        z: plane.z,
        t: plane.t,
        channels: ChannelSelect::Channel(plane.c),
        region,
        output: OutputSpec { keep_alpha: true, ..Default::default() },
    };
    Ok(reader.read_region(&req)?.as_bytes().to_vec())
}

/// Decodes `region` of one level of one series.
pub fn decode_region(reader: &dyn ImageReader, s: usize, level: usize, region: Region) -> scopeio::Result<Series> {
    let meta = reader.series_meta(s)?;
    let mut req = RegionRequest::new(s, level, region);
    (req.channels, req.output) = all_channels(meta.size_c);
    let mut pixels = Vec::new();
    for t in 0..meta.size_t {
        for z in 0..meta.size_z {
            (req.z, req.t) = (z, t);
            pixels.extend_from_slice(reader.read_region(&req)?.as_bytes());
        }
    }
    Ok(Series {
        width: region.width,
        height: region.height,
        size_z: meta.size_z,
        size_t: meta.size_t,
        channels: meta.samples_per_pixel as usize * meta.size_c as usize,
        pixel_type: format!("{:?}", meta.pixel_type),
        phys_x_nm: meta.physical_size.x_nm,
        phys_y_nm: meta.physical_size.y_nm,
        pixels,
    })
}

/// Decodes one resolution level of one series.
pub fn decode_level(reader: &dyn ImageReader, s: usize, level_index: usize) -> scopeio::Result<Series> {
    let meta = reader.series_meta(s)?;
    let level = &meta.levels[level_index];
    let mut req = RegionRequest::new(s, level_index, Region::new(0, 0, level.size_x, level.size_y));
    (req.channels, req.output) = all_channels(meta.size_c);
    let mut pixels = Vec::new();
    for t in 0..meta.size_t {
        for z in 0..meta.size_z {
            (req.z, req.t) = (z, t);
            pixels.extend_from_slice(reader.read_region(&req)?.as_bytes());
        }
    }
    Ok(Series {
        width: level.size_x,
        height: level.size_y,
        size_z: meta.size_z,
        size_t: meta.size_t,
        channels: meta.samples_per_pixel as usize * meta.size_c as usize,
        pixel_type: format!("{:?}", meta.pixel_type),
        phys_x_nm: meta.physical_size.x_nm,
        phys_y_nm: meta.physical_size.y_nm,
        pixels,
    })
}

/// Block means per channel as in `grid()` of `tests/scripts/oracle.py`.
pub fn grid(pixels: &[u8], pixel_type: &str, channels: usize, width: usize, height: usize) -> Vec<f64> {
    const GRID: usize = 8;
    const SAMPLES: usize = 64;
    let bytes = match pixel_type {
        "U8" | "I8" => 1,
        "U16" | "I16" => 2,
        "U32" | "I32" | "F32" => 4,
        _ => 8,
    };
    let value = |i: usize| -> f64 {
        let b = &pixels[i * bytes..(i + 1) * bytes];
        match pixel_type {
            "U8" => b[0] as f64,
            "I8" => b[0] as i8 as f64,
            "U16" => u16::from_ne_bytes([b[0], b[1]]) as f64,
            "I16" => i16::from_ne_bytes([b[0], b[1]]) as f64,
            "U32" => u32::from_ne_bytes(b.try_into().unwrap()) as f64,
            "I32" => i32::from_ne_bytes(b.try_into().unwrap()) as f64,
            "F32" => f32::from_ne_bytes(b.try_into().unwrap()) as f64,
            _ => f64::from_ne_bytes(b.try_into().unwrap()),
        }
    };
    let (sx, sy) = (width.div_ceil(SAMPLES), height.div_ceil(SAMPLES));
    let plane_len = width * height * channels;
    let planes = pixels.len() / bytes / plane_len.max(1);
    let mut sums = vec![0.0; channels * GRID * GRID];
    let mut counts = vec![0usize; channels * GRID * GRID];
    for p in 0..planes {
        for y in (0..height).step_by(sy) {
            let gy = y * GRID / height;
            for x in (0..width).step_by(sx) {
                let gx = x * GRID / width;
                let at = p * plane_len + (y * width + x) * channels;
                for c in 0..channels {
                    let i = (c * GRID + gy) * GRID + gx;
                    sums[i] += value(at + c);
                    counts[i] += 1;
                }
            }
        }
    }
    sums.iter().zip(&counts).map(|(s, &n)| if n > 0 { s / n as f64 } else { f64::NAN }).collect()
}

pub fn crc(data: &[u8]) -> String {
    let mut c = crc32::Crc32::new();
    c.update(data);
    format!("{:08x}", c.finish())
}

/// Differences between one decoded series and one reference row.
///
/// With `tolerance`, pixels are compared by block means instead of CRC-32.
fn compare(ours: &Series, r: &HashMap<String, String>, tolerance: Option<f64>) -> Vec<String> {
    let mut diffs = Vec::new();
    let mut field = |name: &str, ours: String| {
        if r[name] != ours {
            diffs.push(format!("{name}: ours {ours}, reference {}", r[name]));
        }
    };
    field("width", ours.width.to_string());
    field("height", ours.height.to_string());
    field("size_z", ours.size_z.to_string());
    field("size_t", ours.size_t.to_string());
    field("pixel_type", ours.pixel_type.clone());
    for (name, value) in [("phys_x_nm", ours.phys_x_nm), ("phys_y_nm", ours.phys_y_nm)] {
        if let (Ok(reference), Some(v)) = (r[name].parse::<f64>(), value) {
            if (reference - v).abs() > 1e-3 * reference.abs().max(1.0) {
                diffs.push(format!("{name}: ours {v}, reference {reference}"));
            }
        } else if r[name] != "-" && value.is_none() {
            diffs.push(format!("{name}: ours unknown, reference {}", r[name]));
        }
    }
    let ref_channels: usize = r["channels"].parse().unwrap_or(0);
    let pixels = if ours.channels == 1 && ref_channels == 3 {
        // Gray output against a reference that expanded a gray palette to RGB.
        ours.pixels.iter().flat_map(|&v| [v, v, v]).collect()
    } else {
        if ours.channels != ref_channels {
            diffs.push(format!("channels: ours {}, reference {ref_channels}", ours.channels));
        }
        ours.pixels.clone()
    };
    if let Some(tol) = tolerance {
        let channels = ref_channels.max(1);
        let ours_grid = grid(&pixels, &ours.pixel_type, channels, ours.width as usize, ours.height as usize);
        let ref_grid: Vec<f64> = r["grid"].split(',').map(|v| v.parse().unwrap_or(f64::NAN)).collect();
        let worst = ours_grid
            .iter()
            .zip(&ref_grid)
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        if ours_grid.len() != ref_grid.len() || worst > tol {
            diffs.push(format!("block means differ by up to {worst:.2}, tolerance {tol}"));
        }
        return diffs;
    }
    let ours_crc = crc(&pixels);
    if ours_crc != r["crc32"] {
        diffs.push(format!("crc32: ours {ours_crc}, reference {}", r["crc32"]));
    }
    diffs
}

/// Level `series - (count - 1)` of the last of `count` series, if it exists.
/// Series `series` of a reference that lists every pyramid level as a
/// series of its own (Bio-Formats' default), when `total` reference series
/// equal the levels of all our series.
fn flattened_level(path: &Path, total: usize, series: usize) -> Option<Series> {
    let reader = scopeio::open(path).ok()?;
    let flat: Vec<(usize, usize)> = (0..reader.series_count())
        .flat_map(|s| (0..reader.series_meta(s).map(|m| m.levels.len()).unwrap_or(1)).map(move |l| (s, l)))
        .collect();
    if flat.len() != total || flat.len() == reader.series_count() {
        return None;
    }
    let &(s, l) = flat.get(series)?;
    decode_level(reader.as_ref(), s, l).ok()
}

fn level_of_last_series(path: &Path, count: usize, series: usize) -> Option<Series> {
    let reader = scopeio::open(path).ok()?;
    let last = count.checked_sub(1)?;
    let level = series - last;
    if level >= reader.series_meta(last).ok()?.levels.len() {
        return None;
    }
    decode_level(reader.as_ref(), last, level).ok()
}

/// Compares scopeio with all reference rows of a format.
///
/// Reads `tests/expected/<format>.tsv` and `<format>_divergences.tsv`.
/// A (file, tool) pair listed as divergence without tolerance must differ;
/// with a tolerance, its block means must agree within it (lossy codecs).
/// Every other successful reference must match exactly. A (file, `scopeio`)
/// entry marks a file scopeio cannot read yet. Missing external sample
/// files are skipped.
///
/// # Returns
///
/// One message per failure.
pub fn check_against_oracles(format: &str) -> Vec<String> {
    let expected = root().join("tests/expected");
    let rows = read_tsv(&expected.join(format!("{format}.tsv")));
    // Keyed by file, tool and series; an optional `series` column limits an
    // entry to one reference series.
    let divergence_rows = read_tsv(&expected.join(format!("{format}_divergences.tsv")));
    let divergences: HashMap<(String, String), Option<f64>> = divergence_rows
        .iter()
        .filter(|d| d.get("series").is_none_or(|s| s.is_empty() || s == "-"))
        .map(|d| ((d["file"].clone(), d["tool"].clone()), d.get("tolerance").and_then(|t| t.parse().ok())))
        .collect();
    let series_divergences: HashMap<(String, String, String), Option<f64>> = divergence_rows
        .iter()
        .filter_map(|d| {
            let s = d.get("series").filter(|s| !s.is_empty() && *s != "-")?;
            Some(((d["file"].clone(), d["tool"].clone(), s.clone()), d.get("tolerance").and_then(|t| t.parse().ok())))
        })
        .collect();

    let mut failures = Vec::new();
    let mut decoded: HashMap<String, Result<Vec<Series>, String>> = HashMap::new();
    let mut skipped = 0;
    for r in &rows {
        let path = root().join(&r["file"]);
        if !path.exists() {
            skipped += 1;
            continue;
        }
        let key = (r["file"].clone(), r["tool"].clone());
        let series_key = (r["file"].clone(), r["tool"].clone(), r["series"].clone());
        let entry = series_divergences.get(&series_key).or(divergences.get(&key));
        let tolerance = entry.copied().flatten();
        let listed = entry.is_some() && tolerance.is_none();
        // A (file, "scopeio") divergence marks files scopeio cannot read yet.
        let unsupported = divergences.contains_key(&(r["file"].clone(), "scopeio".to_string()));
        let cropped = r.get("crop").is_some_and(|c| c.as_str() != "-")
            || r.get("ours_series").is_some_and(|v| v.starts_with("assoc:") || v.contains('/'));
        if (r["status"] != "ok" || cropped) && !unsupported {
            if r["status"] != "ok" {
                continue;
            }
        } else {
            // Decode whole files only when a row needs them.
            let ours = decoded.entry(r["file"].clone()).or_insert_with(|| {
                scopeio::open(&path).and_then(|reader| decode(reader.as_ref())).map_err(|e| e.to_string())
            });
            match (ours, unsupported) {
                (Ok(_), true) => {
                    failures.push(format!("{}: listed as unreadable by scopeio but opens now", r["file"]));
                    continue;
                }
                (Err(_), true) => continue,
                (Ok(_), false) => {}
                (Err(e), false) => {
                    failures.push(format!("{}: scopeio failed: {e}", r["file"]));
                    continue;
                }
            }
            if r["status"] != "ok" {
                continue;
            }
        }
        // Associated images: `assoc:<kind>`.
        if let Some(kind) = r.get("ours_series").and_then(|v| v.strip_prefix("assoc:")) {
            let kind = match kind {
                "label" => scopeio::AssociatedKind::Label,
                "macro" => scopeio::AssociatedKind::Macro,
                "thumbnail" => scopeio::AssociatedKind::Thumbnail,
                other => scopeio::AssociatedKind::Other(other.to_string()),
            };
            let spec = OutputSpec { keep_alpha: true, ..Default::default() };
            let diffs = match scopeio::open(&path).and_then(|reader| reader.read_associated(&kind, &spec)) {
                Ok(img) => {
                    let s = Series {
                        width: img.width,
                        height: img.height,
                        size_z: 1,
                        size_t: 1,
                        channels: img.channels as usize,
                        pixel_type: format!("{:?}", img.data.pixel_type()),
                        phys_x_nm: None,
                        phys_y_nm: None,
                        pixels: img.as_bytes().to_vec(),
                    };
                    // Associated images have no pixel size of their own.
                    let mut row = r.clone();
                    row.insert("phys_x_nm".into(), "-".into());
                    row.insert("phys_y_nm".into(), "-".into());
                    compare(&s, &row, tolerance)
                }
                Err(e) => vec![format!("associated {kind:?}: {e}")],
            };
            match (diffs.is_empty(), listed) {
                (false, false) => failures.push(format!("{} vs {} {kind:?}: {}", r["file"], r["tool"], diffs.join("; "))),
                (true, true) => failures.push(format!("{} vs {}: listed as divergence but matches now", r["file"], r["tool"])),
                _ => {}
            }
            continue;
        }
        // `ours_series` is a series index or `S/L` for level L of series S.
        let (series, level): (usize, usize) = match r.get("ours_series").map(String::as_str) {
            Some(v) if v.contains('/') => {
                let (s, l) = v.split_once('/').expect("S/L");
                (s.parse().expect("ours_series index"), l.parse().expect("ours_series level"))
            }
            Some(v) if v != "-" => (v.parse().expect("ours_series index"), 0),
            _ => (r["series"].parse().expect("series index"), 0),
        };
        // Cropped references compare the same region of the level.
        if let Some(crop) = r.get("crop").filter(|c| c.as_str() != "-") {
            let n: Vec<u64> = crop.split(',').map(|v| v.parse().expect("crop x,y,w,h")).collect();
            let region = Region::new(n[0], n[1], n[2], n[3]);
            let diffs = match scopeio::open(&path).and_then(|reader| decode_region(reader.as_ref(), series, level, region)) {
                Ok(s) => compare(&s, r, tolerance),
                Err(e) => vec![format!("crop {crop}: {e}")],
            };
            match (diffs.is_empty(), listed) {
                (false, false) => failures.push(format!("{} vs {} crop {crop}: {}", r["file"], r["tool"], diffs.join("; "))),
                (true, true) => failures.push(format!("{} vs {}: listed as divergence but matches now", r["file"], r["tool"])),
                _ => {}
            }
            continue;
        }
        if level > 0 {
            let diffs = match scopeio::open(&path).and_then(|reader| decode_level(reader.as_ref(), series, level).map(|s| (s, reader))) {
                Ok((mut s, reader)) => {
                    // References give a level the pixel size of level 0 or its own.
                    let first = compare(&s, r, tolerance);
                    let f = reader.series_meta(series).map(|m| m.levels[level].downsample).unwrap_or(1.0);
                    // Within 1 %: references may use the level's own rounded resolution.
                    let near = |ours: Option<f64>, key: &str| {
                        let scaled = ours.map(|v| v * f);
                        match (scaled, r[key].parse::<f64>()) {
                            (Some(v), Ok(reference)) if (v - reference).abs() <= 0.01 * reference.abs() => Some(reference),
                            _ => scaled,
                        }
                    };
                    s.phys_x_nm = near(s.phys_x_nm, "phys_x_nm");
                    s.phys_y_nm = near(s.phys_y_nm, "phys_y_nm");
                    let scaled = compare(&s, r, tolerance);
                    if scaled.len() < first.len() { scaled } else { first }
                }
                Err(e) => vec![format!("level {level}: {e}")],
            };
            match (diffs.is_empty(), listed) {
                (false, false) => failures.push(format!("{} vs {} level {level}: {}", r["file"], r["tool"], diffs.join("; "))),
                (true, true) => failures.push(format!("{} vs {}: listed as divergence but matches now", r["file"], r["tool"])),
                _ => {}
            }
            continue;
        }
        let ours = match &decoded[&r["file"]] {
            Ok(series) => series,
            Err(_) => continue,
        };
        let total = rows.iter().filter(|o| o["file"] == r["file"] && o["tool"] == r["tool"] && o.get("crop").is_none_or(|c| c == "-")).count();
        let diffs = match flattened_level(&path, total, series).as_ref().or(ours.get(series)) {
            Some(s) => compare(s, r, tolerance),
            // Bio-Formats returns pyramid levels as extra series by default;
            // compare them with the levels of the last series instead.
            None => match level_of_last_series(&path, ours.len(), series) {
                Some(s) => compare(&s, r, tolerance),
                None => vec![format!("series {series} missing, ours has {}", ours.len())],
            },
        };
        match (diffs.is_empty(), listed) {
            (false, false) => failures.push(format!("{} vs {}: {}", r["file"], r["tool"], diffs.join("; "))),
            (true, true) => failures.push(format!(
                "{} vs {}: listed as divergence but matches now; remove it from {format}_divergences.tsv",
                r["file"], r["tool"]
            )),
            _ => {}
        }
    }
    if skipped > 0 {
        eprintln!("{format}: skipped {skipped} reference rows of files not present locally");
    }
    failures
}

/// Opens and decodes truncated and byte-flipped copies of `path`; any
/// panic fails the calling test. Errors are expected.
#[allow(dead_code)]
pub fn corrupt_sweep(path: &Path, step: usize) {
    let data = std::fs::read(path).unwrap();
    let dir = std::env::temp_dir().join(format!("scopeio-sweep-{}-{}", std::process::id(), path.file_name().unwrap().to_string_lossy()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join(path.file_name().unwrap());
    for at in (0..data.len()).step_by(step.max(1)) {
        let mut d = data.clone();
        if at % 2 == 0 {
            d.truncate(at.max(8));
        } else {
            d[at] ^= 0xA5;
        }
        std::fs::write(&target, &d).unwrap();
        if let Ok(r) = scopeio::open(&target) {
            let _ = decode(r.as_ref());
            for s in 0..r.series_count() {
                if let Ok(m) = r.series_meta(s) {
                    for l in 1..m.levels.len() {
                        let _ = decode_level(r.as_ref(), s, l);
                    }
                }
            }
            for kind in r.associated_images().to_vec() {
                let _ = r.read_associated(&kind, &Default::default());
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

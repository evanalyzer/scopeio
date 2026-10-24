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

pub mod crc32;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use scopeio::{ImageReader, Plane, Region};

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
    let mut out = Vec::new();
    for s in 0..reader.series_count() {
        let meta = reader.series_meta(s)?;
        let level = &meta.levels[0];
        let region = Region::new(0, 0, level.width as u32, level.height as u32);
        let plane_len = meta.buffer_size(region);
        let pixel = meta.color.samples() * meta.pixel_type.bytes();
        let size_c = meta.size_c as usize;
        let mut pixels = vec![0u8; plane_len * size_c * (meta.size_z * meta.size_t) as usize];
        let mut plane = vec![0u8; plane_len];
        let mut offset = 0;
        for t in 0..meta.size_t {
            for z in 0..meta.size_z {
                let block = &mut pixels[offset..offset + plane_len * size_c];
                for c in 0..meta.size_c {
                    reader.read_region_into(s, 0, Plane::new(c, z, t), region, &mut plane)?;
                    for (i, px) in plane.chunks_exact(pixel).enumerate() {
                        let at = (i * size_c + c as usize) * pixel;
                        block[at..at + pixel].copy_from_slice(px);
                    }
                }
                offset += plane_len * size_c;
            }
        }
        out.push(Series {
            width: level.width,
            height: level.height,
            size_z: meta.size_z,
            size_t: meta.size_t,
            channels: meta.color.samples() * size_c,
            pixel_type: format!("{:?}", meta.pixel_type),
            phys_x_nm: meta.physical_size.x_nm,
            phys_y_nm: meta.physical_size.y_nm,
            pixels,
        });
    }
    Ok(out)
}

pub fn crc(data: &[u8]) -> String {
    let mut c = crc32::Crc32::new();
    c.update(data);
    format!("{:08x}", c.finish())
}

/// Differences between one decoded series and one reference row.
fn compare(ours: &Series, r: &HashMap<String, String>) -> Vec<String> {
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
    let ours_crc = crc(&pixels);
    if ours_crc != r["crc32"] {
        diffs.push(format!("crc32: ours {ours_crc}, reference {}", r["crc32"]));
    }
    diffs
}

/// Compares scopeio with all reference rows of a format.
///
/// Reads `tests/expected/<format>.tsv` and `<format>_divergences.tsv`.
/// A (file, tool) pair listed as divergence must differ, every other
/// successful reference must match. Missing external sample files are
/// skipped.
///
/// # Returns
///
/// One message per failure.
pub fn check_against_oracles(format: &str) -> Vec<String> {
    let expected = root().join("tests/expected");
    let rows = read_tsv(&expected.join(format!("{format}.tsv")));
    let divergences: HashMap<(String, String), String> = read_tsv(&expected.join(format!("{format}_divergences.tsv")))
        .into_iter()
        .map(|d| ((d["file"].clone(), d["tool"].clone()), d["reason"].clone()))
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
        let ours = decoded.entry(r["file"].clone()).or_insert_with(|| {
            scopeio::open(&path).and_then(|reader| decode(reader.as_ref())).map_err(|e| e.to_string())
        });
        let key = (r["file"].clone(), r["tool"].clone());
        let listed = divergences.contains_key(&key);
        let ours = match ours {
            Ok(series) => series,
            Err(e) => {
                failures.push(format!("{}: scopeio failed: {e}", r["file"]));
                continue;
            }
        };
        if r["status"] != "ok" {
            continue;
        }
        let series: usize = r["series"].parse().expect("series index");
        let diffs = match ours.get(series) {
            Some(s) => compare(s, r),
            None => vec![format!("series {series} missing, ours has {}", ours.len())],
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

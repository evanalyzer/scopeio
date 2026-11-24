//! # gatan
//!
//! Gatan DigitalMicrograph reader tests against the reference readers and
//! synthetic files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("gatan");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{ElectronMicroscopyMode, FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/gatan").join(name)
}

#[test]
fn synthetic_images_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Gatan, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    // The thumbnail is not a series.
    let r = scopeio::open(&sample("stack.dm3")).unwrap();
    assert_eq!(r.series_count(), 2);
    let m = r.series_meta(0).unwrap();
    assert_eq!(
        (m.name.as_deref(), m.size_z, m.pixel_type, m.physical_size.x_nm, m.physical_size.z_nm),
        (Some("z stack"), 3, PixelType::I16, Some(0.5), Some(2.0))
    );
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.physical_size.x_nm), (4, 1, Some(1500.0)));
    let r = scopeio::open(&sample("rgba.dm4")).unwrap();
    assert_eq!((r.series_meta(0).unwrap().samples_per_pixel, r.series_meta(1).unwrap().size_t), (4, 2));
    assert!((r.series_meta(1).unwrap().physical_size.x_nm.unwrap() - 0.3).abs() < 1e-12);
}

#[test]
fn electron_microscope_settings() {
    // TEM imaging: voltage, nominal magnification, spot size; no SEM fields.
    let r = scopeio::open(&sample("stack.dm3")).unwrap();
    let em = r.series_meta(0).unwrap().electron_microscopy.clone().unwrap();
    assert_eq!(
        (em.mode, em.accelerating_voltage_v, em.magnification, em.spot_size, em.working_distance_mm, em.probe_current_na),
        (Some(ElectronMicroscopyMode::Tem), Some(200000.0), Some(5000.0), Some(3), None, None)
    );
    // No Microscope Info: no section.
    assert_eq!(r.series_meta(1).unwrap().electron_microscopy, None);
    // SEM scanning: working distance and probe current (`SEM Probe Current`,
    // not the zero `Probe Current`).
    let r = scopeio::open(&sample("sem.dm4")).unwrap();
    let em = r.series_meta(0).unwrap().electron_microscopy.clone().unwrap();
    assert_eq!(
        (em.mode, em.accelerating_voltage_v, em.magnification, em.spot_size, em.working_distance_mm, em.probe_current_na),
        (Some(ElectronMicroscopyMode::Sem), Some(5000.0), Some(1200.0), Some(2), Some(7.5), Some(0.4))
    );
    // 8-bit tag names are Latin-1.
    let o = r.original_metadata(0).unwrap();
    assert!(o.pairs.iter().any(|(k, _)| k == "ImageTags.Microscope Info.Emission Current (µA)"), "{:?}", o.pairs);
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("stack.dm3"), 7);
    common::corrupt_sweep(&sample("rgba.dm4"), 7);
    common::corrupt_sweep(&sample("sem.dm4"), 7);
}

/// Electron microscope settings against the `Microscope Info` tags as
/// RosettaSciIO parses them (black box, `tests/scripts/oracle_rsciio.py
/// gatan`): an independent DM tag parser for the values behind
/// `electron_microscopy`. RosettaSciIO 0.15.0 itself maps none of these
/// files' tags to its structured TEM / SEM metadata, so only the tag values
/// are compared. RosettaSciIO signal `i` is scopeio series `i` (their pixels
/// match in `matches_reference_readers`).
///
/// Dumps of the synthetic files are committed (`tests/expected/
/// gatan_rsciio/`); those of the downloaded OME samples are compared when
/// present (`tests/reference/gatan_rsciio/`).
#[test]
fn electron_microscope_settings_match_rsciio_tags() {
    use serde_json::Value;
    let rows = read_tsv(&root().join("tests/expected/gatan.tsv"));
    let mut files: Vec<String> = rows.iter().filter(|r| r["tool"] == "rsciio" && r["status"] == "ok").map(|r| r["file"].clone()).collect();
    files.dedup();
    let mut compared = 0;
    for file in files {
        let path = root().join(&file);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let dir = if file.starts_with("tests/data/") { "tests/expected/gatan_rsciio" } else { "tests/reference/gatan_rsciio" };
        let dump = root().join(dir).join(format!("{name}.json"));
        if !path.exists() || !dump.exists() {
            continue;
        }
        let dump: Value = serde_json::from_str(&std::fs::read_to_string(dump).unwrap()).unwrap();
        let r = scopeio::open(&path).unwrap();
        for (i, signal) in dump.as_array().unwrap().iter().enumerate() {
            let info = &signal["original_metadata"]["ImageList"]["TagGroup0"]["ImageTags"]["Microscope Info"];
            let em = r.series_meta(i).unwrap().electron_microscopy.clone();
            if info.is_null() {
                assert_eq!(em, None, "{name} {i}");
                continue;
            }
            let em = em.unwrap_or_else(|| panic!("{name} {i}: no electron_microscopy"));
            let num = |k: &str| info[k].as_f64().filter(|v| *v > 0.0);
            let mode = match (info["Illumination Mode"].as_str(), info["Operation Mode"].as_str()) {
                (Some("TEM"), Some("IMAGING")) => Some(ElectronMicroscopyMode::Tem),
                (Some("SEM"), Some("SCANNING")) => Some(ElectronMicroscopyMode::Sem),
                _ => None,
            };
            let sem = mode == Some(ElectronMicroscopyMode::Sem);
            assert_eq!(em.mode, mode, "{name} {i}: mode");
            assert_eq!(em.accelerating_voltage_v, num("Voltage"), "{name} {i}: voltage");
            assert_eq!(em.magnification, num("Indicated Magnification").filter(|_| mode.is_some()), "{name} {i}: magnification");
            assert_eq!(em.spot_size, num("Spot Size").map(|v| v as u32), "{name} {i}: spot size");
            assert_eq!(em.working_distance_mm, num("WD (mm)").filter(|_| sem), "{name} {i}: working distance");
            assert_eq!(em.probe_current_na, num("SEM Probe Current (nA)").filter(|_| sem), "{name} {i}: probe current");
            assert!(signal["metadata"]["Acquisition_instrument"].is_null(), "{name} {i}: RosettaSciIO maps TEM / SEM metadata now; compare it");
            compared += 1;
        }
    }
    // The committed synthetic files alone: the TEM z stack and the SEM image.
    assert!(compared >= 2, "compared {compared}");
}

//! # prz
//!
//! CEOS Panta Rhei reader tests against synthetic files, a real sample in
//! `tests/external/` when present, and RosettaSciIO as a black-box oracle
//! (Bio-Formats and bioformats-rs cannot open the format).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

/// Pixels, shape, type and pixel size against RosettaSciIO (`rsciio` rows of
/// `tests/expected/prz.tsv`); the Bio-Formats and bioformats-rs rows record
/// their errors.
#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("prz");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane, PlaneMeta};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/prz").join(name)
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::PantaRhei, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("plain.prz")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.levels[0].size_x, m.levels[0].size_y, m.pixel_type), (1, 1, 1, 7, 5, PixelType::U16));
    let (x, y) = (m.physical_size.x_nm.unwrap(), m.physical_size.y_nm.unwrap());
    assert!((x - 0.5).abs() < 1e-9 && (y - 0.6).abs() < 1e-9, "{x} {y}");
    assert_eq!(m.acquisition_date.as_deref(), Some("2026-10-02T11:43:51.742615"));
    assert_eq!(m.channels[0].exposure_s, Some(1.5));
    assert_eq!(m.microscope.as_ref().and_then(|s| s.model.as_deref()), Some("TEM"));
    let stage = m.stage_position.unwrap();
    assert!((stage.x_nm.unwrap() - 1000.0).abs() < 1e-6 && (stage.y_nm.unwrap() + 2000.0).abs() < 1e-6);
    let o = r.original_metadata(0).unwrap();
    let get = |k: &str| o.pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(get("file_format_version"), Some("8"));
    assert_eq!(get("device.calib[0].unit"), Some("m"));
    assert_eq!(get("camera.binning"), Some("[2,2]"));
    assert!(o.documents.iter().any(|(n, _)| n == "data_model_json"));

    // One image: plane 0 carries the series values.
    let p = r.plane_meta(0, Plane::new(0, 0, 0)).unwrap();
    assert_eq!((p.delta_t_s, p.exposure_s), (Some(0.0), Some(1.5)));
    assert!((p.position.unwrap().x_nm.unwrap() - 1000.0).abs() < 1e-6);
    assert_eq!(m.time_increment_s, None);

    // A stack: per-frame time, exposure and stage position.
    let r = scopeio::open(&sample("deflated.prz")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.size_t, m.pixel_type), (3, PixelType::F32));
    assert_eq!(m.time_increment_s, Some(1.0));
    for t in 0..3 {
        let p = r.plane_meta(0, Plane::new(0, 0, t)).unwrap();
        assert_eq!((p.delta_t_s, p.exposure_s), (Some(t as f64), Some(1.5)), "frame {t}");
        assert!((p.position.unwrap().x_nm.unwrap() - (1000.0 + 500.0 * t as f64)).abs() < 1e-6, "frame {t}");
    }
    assert!(r.plane_meta(0, Plane::new(0, 0, 3)).is_err());

    let m = scopeio::open(&sample("fortran.prz")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.pixel_type, m.physical_size.x_nm.is_some()), (PixelType::I16, true));
    let r = scopeio::open(&sample("zip64.prz")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.pixel_type, m.physical_size.x_nm, m.acquisition_date, m.dataset), (PixelType::U8, None, None, None));
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 0)).unwrap(), PlaneMeta::default());

    // Detector and acquisition session.
    let m = scopeio::open(&sample("tem.prz")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!(m.channels[0].detector.as_ref().and_then(|d| d.model.as_deref()), Some("SyntheticCam"));
    assert_eq!((m.name.as_deref(), m.dataset.as_deref()), (Some("tem"), Some("synthetic_session.3")));
}

#[test]
fn real_sample() {
    // TVIPS camera on a JEOL TEM, 2048 x 2048 u16 (not committed).
    let path = root().join("tests/external/panta_rhei_01.prz");
    if !path.exists() {
        eprintln!("skipped: {} not present", path.display());
        return;
    }
    let r = scopeio::open(&path).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.pixel_type, m.size_t), (2048, 2048, PixelType::U16, 1));
    // 15.5 um detector pixels, binning 2, magnification 50000 x 1.18.
    assert!((m.physical_size.x_nm.unwrap() - 31e3 / 59000.0).abs() < 1e-9);
    assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), "1a8a6e96");
}

/// Flattens RosettaSciIO's `original_metadata` with the key scheme of
/// scopeio's original metadata: nested objects as dotted keys, lists of
/// objects as `key[i]`, nulls dropped.
fn flatten(prefix: &str, v: &serde_json::Value, out: &mut Vec<(String, serde_json::Value)>) {
    use serde_json::Value;
    match v {
        Value::Object(m) => m.iter().for_each(|(k, x)| flatten(&if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") }, x, out)),
        Value::Array(a) if a.iter().any(|x| x.is_object() || x.is_array()) => {
            a.iter().enumerate().for_each(|(i, x)| flatten(&format!("{prefix}[{i}]"), x, out));
        }
        Value::Null => {}
        x => out.push((prefix.to_string(), x.clone())),
    }
}

/// `ours` (a scopeio original-metadata value) equals `theirs`: as JSON
/// (numbers compared within 1e-12 relative), strings as text.
fn same(ours: &str, theirs: &serde_json::Value) -> bool {
    use serde_json::Value;
    if let Value::String(s) = theirs {
        return ours == s;
    }
    match (serde_json::from_str::<Value>(ours), theirs) {
        (Ok(Value::Number(a)), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
        }
        (Ok(a), b) => &a == b,
        // Text that is not JSON only equals a string, handled above.
        (Err(_), _) => false,
    }
}

/// Metadata against RosettaSciIO (black box, `tests/scripts/oracle_rsciio.py`).
///
/// - Every `original_metadata` value of RosettaSciIO is in scopeio's
///   original metadata with the same value. RosettaSciIO keeps only frame 0
///   of a stack; scopeio keeps all frames under `[i].` keys.
/// - RosettaSciIO's structured TEM fields have no `SeriesMeta` counterpart;
///   they are derived from keys scopeio keeps: beam energy (kV) from
///   `electron_gun.voltage` (V), convergence angle (mrad) from
///   `condenser.convergence_semi_angle` (rad), magnification from
///   `projector.magnification`.
/// - Not compared (documented divergences): RosettaSciIO's title is
///   `repo_id` without its last `.` suffix, scopeio's name the file stem;
///   RosettaSciIO's `acquisition_mode` is its own classification.
///
/// Dumps of the committed synthetic files are in `tests/expected/prz_rsciio/`;
/// `tem.prz` carries the full key set of a real TEM file with invented
/// values. Dumps of real samples stay uncommitted in
/// `tests/reference/prz_rsciio/` and are compared when present.
#[test]
fn metadata_matches_rsciio() {
    let synthetic = ["plain.prz", "deflated.prz", "fortran.prz", "zip64.prz", "tem.prz"]
        .map(|n| (sample(n), root().join("tests/expected/prz_rsciio").join(format!("{n}.json"))));
    let real = [(root().join("tests/external/panta_rhei_01.prz"), root().join("tests/reference/prz_rsciio/panta_rhei_01.prz.json"))];
    let mut tem_fields = 0;
    for (path, dump) in synthetic.into_iter().chain(real) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !path.exists() || !dump.exists() {
            continue;
        }
        let dump: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dump).unwrap()).unwrap();
        let signal = &dump[0];
        let r = scopeio::open(&path).unwrap();
        let ours = r.original_metadata(0).unwrap().pairs;
        let frames = r.series_meta(0).unwrap().size_t;
        let get = |k: &str| {
            let k0 = format!("[0].{k}");
            ours.iter().find(|(n, _)| n == k || (frames > 1 && *n == k0)).map(|(_, v)| v.clone())
        };
        // RosettaSciIO stores `device.calib` (without its trailing null)
        // and `ref_size` in NumPy axis order (y, x); the file and scopeio
        // keep x first. `image_size` and `detector_size` it leaves as stored.
        let mut original = signal["original_metadata"].clone();
        for key in ["device.calib", "ref_size"] {
            if let Some(v) = original.get_mut(key).and_then(|c| c.as_array_mut()) {
                v.reverse();
            }
        }
        let mut theirs = Vec::new();
        flatten("", &original, &mut theirs);
        for (k, v) in &theirs {
            match get(k) {
                Some(o) => assert!(same(&o, v), "{name}: {k}: scopeio {o}, rsciio {v}"),
                None => panic!("{name}: {k} missing in scopeio (rsciio {v})"),
            }
        }
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        let tem = &signal["metadata"]["Acquisition_instrument"]["TEM"];
        let close = |a: Option<f64>, b: Option<f64>| {
            assert!(matches!((a, b), (Some(a), Some(b)) if (a - b).abs() <= 1e-9 * a.abs()), "{name}: {a:?} vs {b:?}")
        };
        if let Some(kv) = tem["beam_energy"].as_f64() {
            close(num("electron_gun.voltage").map(|v| v / 1e3), Some(kv));
            tem_fields += 1;
        }
        if let Some(mrad) = tem["convergence_angle"].as_f64() {
            close(num("condenser.convergence_semi_angle").map(|v| v * 1e3), Some(mrad));
            tem_fields += 1;
        }
        if let Some(mag) = tem["magnification"].as_f64() {
            close(num("projector.magnification"), Some(mag));
            tem_fields += 1;
        }
        if name == "tem.prz" {
            assert_eq!(signal["metadata"]["General"]["title"], "synthetic_session", "title from repo_id");
            // scopeio adds `file_format_version` from its own entry.
            let ours_json = ours.iter().filter(|(k, _)| k != "file_format_version").count();
            assert_eq!(theirs.len(), ours_json, "tem.prz: every scopeio JSON key is also in RosettaSciIO's original metadata");
        }
    }
    // The committed tem.prz alone exercises all three structured fields.
    assert!(tem_fields >= 3, "TEM fields compared: {tem_fields}");
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("plain.prz"), 7);
    common::corrupt_sweep(&sample("deflated.prz"), 3);
    common::corrupt_sweep(&sample("fortran.prz"), 7);
    common::corrupt_sweep(&sample("zip64.prz"), 1);
    common::corrupt_sweep(&sample("tem.prz"), 13);
}

//! # nikon_tiff
//!
//! Nikon NIS-Elements TIFF reader tests against the reference readers and
//! a synthetic file.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("nikon-tiff");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/nikon-tiff").join(name)
}

#[test]
fn synthetic_sample_matches_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::NikonTiff);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("gray.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!(m.physical_size.x_nm.map(|v| v.round()), Some(325.0));
    let o = m.objective.as_ref().unwrap();
    assert_eq!((o.model.as_deref(), o.numerical_aperture), (Some("Plan Apo 20x"), Some(0.75)));
    let p = m.stage_position.unwrap();
    assert_eq!((p.x_nm.map(|v| v.round()), p.y_nm.map(|v| v.round()), p.z_nm.map(|v| v.round())), (Some(100_000.0), Some(-200_000.0), Some(3_000_000.0)));
    assert_eq!(m.channels[0].name.as_deref(), Some("DAPI"));
    assert!(m.acquisition_date.as_deref().is_some_and(|d| d.starts_with("2022-11-")));
}

#[test]
fn public_sample_metadata() {
    let path = root().join("tests/reference/zenodo-7677868/h4k20me002xy13c2.tif");
    if !path.exists() {
        return;
    }
    let m = scopeio::open(&path).unwrap().series_meta(0).unwrap().clone();
    assert_eq!(m.physical_size.x_nm.map(|v| (v * 10.0).round()), Some(1625.0));
    assert_eq!(m.objective.and_then(|o| o.numerical_aperture), Some(1.15));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("gray.tif"), 5);
}

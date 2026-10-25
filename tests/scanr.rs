//! # scanr
//!
//! Olympus scan^R reader tests against the reference readers.
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
    let failures = common::check_against_oracles("scanr");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};

fn plate() -> std::path::PathBuf {
    root().join("tests/data/scanr/plate/experiment_descriptor.xml")
}

#[test]
fn synthetic_plate_matches_spec_truth() {
    let r = scopeio::open(&plate()).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::ScanR);
    let truth = read_tsv(&root().join("tests/data/scanr/truth.tsv"));
    assert_eq!(r.series_count(), truth.len());
    for (s, t) in truth.iter().enumerate() {
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{}", t["series"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&plate()).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t), (2, 2, 2));
    assert_eq!((m.channels[1].name.as_deref(), m.channels[1].exposure_s), (Some("gfp"), Some(0.25)));
    let p = &r.plates()[0];
    assert_eq!((p.rows, p.cols, p.name.as_deref()), (8, 12, Some("synthetic plate")));
    // Well 14 of a 96-well plate is B02; two positions per well.
    let w = &p.wells[2];
    assert_eq!((w.row, w.col, w.field, w.series), (1, 1, 0, 2));
}

#[test]
fn public_plate_layout() {
    // idr0009: 357 imaged wells of a 384-well plate, as Bio-Formats lays them out.
    let path = root().join("tests/reference/ome/ScanR/idr0009/0307-10--2007-05-30/experiment_descriptor.xml");
    if !path.exists() {
        return;
    }
    let r = scopeio::open(&path).unwrap();
    let p = &r.plates()[0];
    assert_eq!((p.rows, p.cols, p.wells.len()), (16, 24, 357));
    assert_eq!([(p.wells[0].row, p.wells[0].col), (p.wells[2].row, p.wells[2].col)], [(0, 1), (0, 4)]);
}

#[test]
fn corrupt_descriptor_does_not_panic() {
    let dir = std::env::temp_dir().join(format!("scopeio-scanr-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let src = std::fs::read(plate()).unwrap();
    for cut in (0..src.len()).step_by(97) {
        std::fs::write(dir.join("experiment_descriptor.xml"), &src[..cut]).unwrap();
        let _ = scopeio::open(&dir.join("experiment_descriptor.xml"));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

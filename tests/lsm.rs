//! # lsm
//!
//! Zeiss LSM reader tests against the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("lsm");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode, read_tsv, root};
use scopeio::{AssociatedKind, Plane};

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/lsm");
    let r = scopeio::open(&dir.join("synthetic.lsm")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Lsm);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.significant_bits), (2, 3, 2, Some(12)));
    assert!((m.physical_size.x_nm.unwrap() - 200.0).abs() < 1e-6);
    assert!((m.physical_size.z_nm.unwrap() - 1500.0).abs() < 1e-6);
    assert_eq!(m.time_increment_s, Some(4.5));
    assert_eq!(m.channels[0].name.as_deref(), Some("Ch1-T1"));
    assert_eq!(m.channels[0].color, Some([0, 255, 0, 255]));
    assert_eq!(m.channels[1].color, Some([255, 0, 0, 255]));
    assert_eq!(r.plane_meta(0, Plane::new(1, 2, 1)).unwrap().delta_t_s, Some(4.5));
    assert_eq!(r.associated_images(), [AssociatedKind::Thumbnail]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        assert_eq!(crc(&decode(r.as_ref()).unwrap()[0].pixels), t["crc32"]);
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&root().join("tests/data/lsm/synthetic.lsm"), 7);
}

//! # ims
//!
//! Imaris IMS reader tests against the reference readers.
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
    let failures = common::check_against_oracles("ims");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::Plane;

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/ims");
    let r = scopeio::open(&dir.join("synthetic.ims")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Ims);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.levels.len()), (2, 2, 2, 2));
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.levels[1].size_x, m.levels[1].downsample), (8, 6, 4, 2.0));
    assert_eq!(m.levels[0].tile, None);
    assert_eq!(m.physical_size.x_nm, Some(500.0));
    assert_eq!(m.physical_size.z_nm, Some(1500.0));
    assert_eq!(m.name.as_deref(), Some("synthetic.lsm"));
    assert_eq!(m.channels[0].name.as_deref(), Some("DAPI"));
    assert_eq!(m.channels[0].color, Some([0, 0, 255, 255]));
    assert_eq!(m.channels[0].emission_wavelength_nm, Some(461.0));
    assert_eq!(m.objective.as_ref().and_then(|o| o.numerical_aperture), Some(1.4));
    assert_eq!(m.time_increment_s, Some(30.5));
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 1)).unwrap().delta_t_s, Some(30.5));
    for t in read_tsv(&dir.join("truth.tsv")) {
        let level: usize = t["image"].trim_start_matches("level").parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), 0, level).unwrap().pixels), t["crc32"], "{}", t["image"]);
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&root().join("tests/data/ims/synthetic.ims"), 5);
}

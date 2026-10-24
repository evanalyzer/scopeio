//! # dv
//!
//! DeltaVision reader tests against the reference readers and synthetic
//! files.
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
    let failures = common::check_against_oracles("dv");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/dv").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::DeltaVision);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("wzt_be.r3d")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 3, 2, PixelType::U16));
    assert_eq!((m.physical_size.x_nm.map(|v| v.round()), m.physical_size.z_nm.map(|v| v.round()), m.time_increment_s), (Some(100.0), Some(300.0), Some(2.5)));
    assert_eq!((m.channels[1].name.as_deref(), m.channels[1].emission_wavelength_nm, m.channels[1].excitation_wavelength_nm), (Some("610"), Some(610.0), Some(560.0)));
    assert_eq!(r.plane_meta(0, Plane::new(1, 2, 1)).unwrap().delta_t_s, Some(2.5));
    let f = scopeio::open(&sample("ztw_le.dv")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((f.size_z, f.pixel_type), (2, PixelType::F32));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("wzt_be.r3d"), 13);
}

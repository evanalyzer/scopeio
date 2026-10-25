//! # nifti
//!
//! NIfTI reader tests against the reference readers and synthetic files.
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
    let failures = common::check_against_oracles("nifti");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::PixelType;

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/nifti").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::Nifti);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let meta = |name: &str| scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone();
    let m = meta("le_u16_5d.nii");
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 3, 2, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.time_increment_s), (Some(500.0), Some(1500.0), Some(2.0)));
    let m = meta("rgb.nii");
    assert_eq!((m.samples_per_pixel, m.pixel_type), (3, PixelType::U8));
    assert_eq!(meta("v2_f64.nii.gz").pixel_type, PixelType::F64);
    // The image file of a pair opens through its header.
    assert_eq!(meta("v2_pair.img").pixel_type, PixelType::I8);
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("le_u16_5d.nii"), 17);
    common::corrupt_sweep(&sample("v2_f64.nii.gz"), 5);
}

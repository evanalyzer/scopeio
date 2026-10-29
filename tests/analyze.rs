//! # analyze
//!
//! Analyze 7.5 reader tests against the reference readers and synthetic
//! files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("analyze");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/analyze").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Analyze, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let meta = |name: &str| scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone();
    let m = meta("be_i16.hdr");
    assert_eq!((m.size_z, m.size_t, m.pixel_type), (3, 1, PixelType::I16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(500_000.0), Some(2_000_000.0)));
    // The image file opens through its header.
    let m = meta("le_u8_4d.img");
    assert_eq!((m.size_z, m.size_t, m.pixel_type, m.physical_size.x_nm), (2, 3, PixelType::U8, Some(250.0)));
    assert_eq!(meta("f32.hdr").physical_size.x_nm, None);
    assert_eq!(meta("rgb.hdr").samples_per_pixel, 3);
    let r = scopeio::open(&sample("le_u8_4d.hdr")).unwrap();
    assert!(r.original_metadata(0).unwrap().pairs.contains(&("Scale slope".to_string(), "2.5".to_string())));
}

#[test]
fn corrupt_files_do_not_panic() {
    // The sweep copies one file alone: the header without its image file.
    common::corrupt_sweep(&sample("be_i16.hdr"), 3);
}

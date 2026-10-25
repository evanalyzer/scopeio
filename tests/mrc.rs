//! # mrc
//!
//! MRC reader tests against the reference readers and synthetic files.
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
    let failures = common::check_against_oracles("mrc");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::PixelType;

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/mrc").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::Mrc);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let meta = |name: &str| scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone();
    let m = meta("i16_be.mrc");
    assert_eq!((m.size_z, m.size_t, m.pixel_type), (3, 1, PixelType::I16));
    assert_eq!((m.physical_size.x_nm.map(|v| (v * 1e6).round()), m.physical_size.z_nm.map(|v| (v * 1e6).round())), (Some(150000.0), Some(200000.0)));
    let m = meta("u16_stack.mrcs");
    assert_eq!((m.size_z, m.size_t, m.pixel_type), (2, 2, PixelType::U16));
    assert_eq!(m.physical_size.z_nm.map(|v| (v * 1e6).round()), Some(400000.0));
    assert_eq!(meta("u8_old.st").pixel_type, PixelType::U8);
    assert_eq!(meta("i8.mrc").pixel_type, PixelType::I8);
    assert_eq!(meta("f16.mrc").pixel_type, PixelType::F32);
    assert_eq!(meta("nibble.mrc").pixel_type, PixelType::U8);
    let labels = scopeio::open(&sample("i16_be.mrc")).unwrap().original_metadata(0).unwrap();
    assert!(labels.pairs.contains(&("Label 2".into(), "second label".into())));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("i16_be.mrc"), 13);
    common::corrupt_sweep(&sample("nibble.mrc"), 7);
}

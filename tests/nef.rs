//! # nef
//!
//! Nikon NEF reader tests against the reference readers and a synthetic
//! file.
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
    let failures = common::check_against_oracles("nef");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/nef").join(name)
}

#[test]
fn synthetic_raw_matches_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::Nef);
        let m = r.series_meta(0).unwrap();
        assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_c, m.pixel_type), (10, 6, 1, scopeio::PixelType::U16));
        assert_eq!(m.channels[0].name.as_deref(), Some("CFA RGGB"));
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn compressed_raw_is_unsupported() {
    let path = root().join("tests/reference/rawpixls/924.nef");
    if !path.exists() {
        return;
    }
    assert!(matches!(scopeio::open(&path), Err(scopeio::Error::UnsupportedVariant(_))));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("raw12.nef"), 7);
}

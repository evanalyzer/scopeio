//! # sis
//!
//! Olympus SIS TIFF reader tests against the reference readers.
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
    let failures = common::check_against_oracles("sis");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/sis").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::Sis);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("palette.tif")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.samples_per_pixel, m.size_c, m.pixel_type), (1, 1, scopeio::PixelType::U8));
    assert_eq!(m.physical_size.x_nm.map(|v| (v * 1e6).round() / 1e6), Some(250.0));
    assert_eq!((m.name.as_deref(), m.acquisition_date.as_deref()), (Some("synthetic sis"), Some("2021-03-04T05:06:00")));
    let o = r.original_metadata(0).unwrap();
    assert!(o.pairs.contains(&("Camera".into(), "Test Camera".into())));
    assert!(o.pairs.contains(&("Magnification".into(), "40".into())));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("palette.tif"), 7);
}

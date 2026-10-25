//! # fluoview
//!
//! Olympus FluoView TIFF reader tests against the reference readers and
//! synthetic files.
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
    let failures = common::check_against_oracles("fluoview");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::PixelType;

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/fluoview").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::FluoView);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("zc.tif")).unwrap();
    let m = r.series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 3, 1, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.name.as_deref()), (Some(200.0), Some(500.0), Some("synthetic zc")));
    let m = scopeio::open(&sample("ct.tif")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.time_increment_s), (2, 1, 3, Some(1.5)));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("zc.tif"), 11);
}

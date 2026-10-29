//! # xrm
//!
//! Zeiss Xradia (TXM, TXRM, XRM) reader tests against the reference readers
//! and synthetic files.
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
    let failures = common::check_against_oracles("xrm");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/xrm").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Xrm, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("projections.txrm")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_z, m.pixel_type, m.physical_size.x_nm), (150, PixelType::U16, Some(500.0)));
    let p = r.plane_meta(0, Plane::new(0, 4, 0)).unwrap();
    assert_eq!((p.exposure_s, p.position.and_then(|p| p.z_nm)), (Some(0.75), Some(1000.0)));
    let m = scopeio::open(&sample("single.xrm")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_z, m.pixel_type, m.physical_size.x_nm), (1, PixelType::F32, Some(1500.0)));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("single.xrm"), 61);
    common::corrupt_sweep(&sample("projections.txrm"), 97);
}

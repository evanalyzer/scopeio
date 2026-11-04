//! # obf
//!
//! Imspector OBF reader tests against the reference readers and a synthetic
//! file.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("obf");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/obf").join(name)
}

#[test]
fn synthetic_stacks_match_spec_truth() {
    let r = scopeio::open(&sample("two.obf")).unwrap();
    assert_eq!((r.format().id, r.series_count()), (FormatId::Obf, 2));
    for t in read_tsv(&sample("truth.tsv")) {
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "series {s}");
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("two.obf")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 1, 1, PixelType::U16));
    assert_eq!((m.name.as_deref(), m.physical_size.x_nm), (Some("Detector stack"), Some(100.0)));
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.pixel_type, m.physical_size.y_nm), (PixelType::F32, Some(200.0)));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("two.obf"), 17);
}

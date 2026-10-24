//! # stk
//!
//! MetaMorph STK reader tests against the reference readers.
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
    let failures = common::check_against_oracles("stk");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/stk").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, scopeio::FormatId::Stk);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let z = scopeio::open(&sample("zstack.stk")).unwrap().series_meta(0).unwrap().clone();
    // Calibration 25/100 um; z distance 3/2 um.
    assert_eq!((z.size_z, z.size_t, z.physical_size.x_nm, z.physical_size.z_nm), (4, 1, Some(250.0), Some(1500.0)));
    let t = scopeio::open(&sample("timeseries.stk")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((t.size_z, t.size_t, t.time_increment_s), (1, 3, Some(1.5)));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("zstack.stk"), 5);
}

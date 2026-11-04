//! # sdt
//!
//! Becker & Hickl SDT reader tests against the reference readers and
//! synthetic files.
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
    let failures = common::check_against_oracles("sdt");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/sdt").join(name)
}

#[test]
fn synthetic_blocks_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Sdt, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("image.sdt")).unwrap();
    assert_eq!(r.series_count(), 2);
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_t, m.pixel_type), (7, 5, 8, PixelType::U16));
    // Bin width tac_r / (tac_g * adc_re).
    assert!((m.time_increment_s.unwrap() - 5e-8 / 32.0).abs() < 1e-15);
    assert_eq!(m.acquisition_date.as_deref(), Some("2026-11-04T10:11:12"));
    let m = scopeio::open(&sample("decay.sdt")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_t), (1, 1, 16));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("image.sdt"), 11);
    common::corrupt_sweep(&sample("decay.sdt"), 5);
}

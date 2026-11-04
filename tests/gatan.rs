//! # gatan
//!
//! Gatan DigitalMicrograph reader tests against the reference readers and
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
    let failures = common::check_against_oracles("gatan");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/gatan").join(name)
}

#[test]
fn synthetic_images_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Gatan, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    // The thumbnail is not a series.
    let r = scopeio::open(&sample("stack.dm3")).unwrap();
    assert_eq!(r.series_count(), 2);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.name.as_deref(), m.size_z, m.pixel_type, m.physical_size.x_nm, m.physical_size.z_nm), (Some("z stack"), 3, PixelType::I16, Some(0.5), Some(2.0)));
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.physical_size.x_nm), (4, 1, Some(1500.0)));
    let r = scopeio::open(&sample("rgba.dm4")).unwrap();
    assert_eq!((r.series_meta(0).unwrap().samples_per_pixel, r.series_meta(1).unwrap().size_t), (4, 2));
    assert!((r.series_meta(1).unwrap().physical_size.x_nm.unwrap() - 0.3).abs() < 1e-12);
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("stack.dm3"), 7);
    common::corrupt_sweep(&sample("rgba.dm4"), 7);
}

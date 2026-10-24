//! # xlef
//!
//! Leica LOF and XLEF/XLCF/XLIF reader tests against the reference readers
//! and synthetic files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/xlef").join(name)
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("xlef");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        let got = decode_level(r.as_ref(), 0, 0).unwrap().pixels;
        assert_eq!(crc(&got), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let lof = scopeio::open(&sample("image.lof")).unwrap();
    assert_eq!(lof.format().id, FormatId::Lof);
    let m = lof.series_meta(0).unwrap().clone();
    assert_eq!((m.samples_per_pixel, m.name.as_deref(), m.physical_size.x_nm), (3, Some("rgb image"), Some(1000.0)));

    let x = scopeio::open(&sample("experiment/experiment.xlef")).unwrap();
    assert_eq!(x.format().id, FormatId::Xlef);
    let m = x.series_meta(0).unwrap().clone();
    // Named by the collection folder; frames in other files, listed out of order.
    assert_eq!((m.name.as_deref(), m.size_c, m.size_z, m.pixel_type), (Some("folder A/stack"), 2, 3, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(500.0), Some(2000.0)));
    assert_eq!(m.channels[1].color, Some([255, 0, 0, 255]));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("image.lof"), 11);
    common::corrupt_sweep(&sample("experiment/Metadata/image.xlif"), 17);
}

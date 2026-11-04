//! # ics
//!
//! ICS reader tests against the reference readers and synthetic files.
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
    let failures = common::check_against_oracles("ics");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/ics").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Ics, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let meta = |name: &str| scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone();
    // The data file opens through its header.
    let m = meta("be_u16.ids");
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (4, 3, 1, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(250.0), Some(1000.0)));
    assert_eq!(m.channels[1].excitation_wavelength_nm, Some(561.0));
    assert_eq!(m.objective.and_then(|o| o.numerical_aperture), Some(1.4));
    let m = meta("ch_first.ics");
    assert_eq!((m.size_c, m.size_t, m.physical_size.x_nm, m.time_increment_s), (3, 2, Some(2.0), Some(1.5)));
    assert_eq!((meta("gzip.ics").size_c, meta("gzip.ics").pixel_type), (2, PixelType::I8));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("ch_first.ics"), 7);
    common::corrupt_sweep(&sample("gzip.ics"), 5);
}

//! # oir
//!
//! Olympus OIR reader tests against the reference readers.
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
    let failures = common::check_against_oracles("oir");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::PixelType;

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/oir").join(name)
}

#[test]
fn synthetic_sample_matches_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        let got = decode_level(r.as_ref(), 0, 0).unwrap().pixels;
        assert_eq!(crc(&got), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("synthetic.oir")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Oir);
    let m = r.series_meta(0).unwrap().clone();
    // Time point 2 lives in the continuation file synthetic_00001.
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type, m.significant_bits), (2, 3, 2, PixelType::U16, Some(12)));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.time_increment_s), (Some(250.0), Some(1500.0), Some(2.5)));
    // Channels by @order; one enabled laser of the two listed.
    let names: Vec<_> = m.channels.iter().map(|c| c.name.clone().unwrap_or_default()).collect();
    assert_eq!(names, ["CH1", "CH2"]);
    assert_eq!((m.channels[0].fluor.as_deref(), m.channels[0].excitation_wavelength_nm), (Some("DAPI"), Some(488.0)));
    let o = m.objective.unwrap();
    assert_eq!((o.model.as_deref(), o.magnification, o.numerical_aperture), (Some("UPLXAPO20X"), Some(20.0), Some(0.8)));
    assert_eq!(m.acquisition_date.as_deref(), Some("2024-01-02T03:04:05.600+01:00"));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("synthetic.oir"), 7);
}

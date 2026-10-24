//! # oif
//!
//! Olympus FluoView OIB/OIF reader tests against the reference readers and
//! the synthetic samples.
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
    let failures = common::check_against_oracles("oif");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/oif").join(name)
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
    let oib = scopeio::open(&sample("synthetic.oib")).unwrap();
    let oif = scopeio::open(&sample("synthetic.oif")).unwrap();
    assert_eq!((oib.format().id, oif.format().id), (FormatId::Oib, FormatId::Oif));
    let m = oib.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type, m.significant_bits), (2, 3, 2, PixelType::U16, Some(12)));
    // The first plane's .pty (0,125 um) wins over the main file (0,25 um); z from the Z interval (750 nm).
    assert_eq!((m.physical_size.x_nm, m.physical_size.y_nm, m.physical_size.z_nm), (Some(125.0), Some(125.0), Some(750.0)));
    assert_eq!(m.channels[0].name.as_deref(), Some("CH1"));
    assert_eq!((m.channels[0].emission_wavelength_nm, m.channels[0].excitation_wavelength_nm), (Some(461.0), Some(405.0)));
    // DyeName "None" and emission 0 are not reported.
    assert_eq!((m.channels[1].fluor.as_deref(), m.channels[1].emission_wavelength_nm), (None, None));
    let o = m.objective.as_ref().unwrap();
    assert_eq!((o.magnification, o.numerical_aperture, o.model.as_deref()), (Some(60.0), Some(1.35), Some("UPLSAPO 60X O NA:1.35")));
    assert_eq!(m.acquisition_date.as_deref(), Some("2021-03-04T05:06:07"));
}

#[test]
fn missing_plane_reads_as_zeros() {
    let dir = std::env::temp_dir().join(format!("scopeio-oif-{}", std::process::id()));
    let files = dir.join("synthetic.oif.files");
    std::fs::create_dir_all(&files).unwrap();
    std::fs::copy(sample("synthetic.oif"), dir.join("synthetic.oif")).unwrap();
    for e in std::fs::read_dir(sample("synthetic.oif.files")).unwrap() {
        let e = e.unwrap();
        if e.file_name() != "s_C002Z003T002.tif" {
            std::fs::copy(e.path(), files.join(e.file_name())).unwrap();
        }
    }
    let r = scopeio::open(&dir.join("synthetic.oif")).unwrap();
    let full = common::read(r.as_ref(), 0, 0, scopeio::Plane::new(1, 2, 1), scopeio::Region::new(0, 0, 20, 13)).unwrap();
    assert!(full.iter().all(|&b| b == 0));
    let other = common::read(r.as_ref(), 0, 0, scopeio::Plane::new(0, 2, 1), scopeio::Region::new(0, 0, 20, 13)).unwrap();
    assert!(other.iter().any(|&b| b != 0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("synthetic.oib"), 11);
}

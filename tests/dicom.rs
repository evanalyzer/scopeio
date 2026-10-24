//! # dicom
//!
//! DICOM reader tests against the reference readers and synthetic files.
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
    let failures = common::check_against_oracles("dicom");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{AssociatedKind, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/dicom").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap().pixels,
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap().pixels,
            _ => r.read_associated(&AssociatedKind::Label, &Default::default()).unwrap().as_bytes().to_vec(),
        };
        assert_eq!(crc(&got), t["crc32"], "{} {}", t["file"], t["image"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("wsi/level1.dcm")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Dicom);
    let m = r.series_meta(0).unwrap().clone();
    // Opening any file of the set gives the whole pyramid.
    assert_eq!((m.levels.len(), m.levels[1].size_x, m.levels[1].downsample, m.samples_per_pixel), (2, 20, 2.0, 3));
    assert_eq!((m.physical_size.x_nm, m.acquisition_date.as_deref()), (Some(500.0), Some("2024-01-02T03:04:05")));
    assert_eq!(r.associated_images(), [AssociatedKind::Label]);

    let f = scopeio::open(&sample("fluo/level0.dcm")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((f.size_c, f.size_z, f.pixel_type, f.physical_size.z_nm), (2, 2, PixelType::U16, Some(2000.0)));
    assert_eq!(f.channels[1].name.as_deref(), Some("FITC"));
    assert_eq!((f.channels[0].excitation_wavelength_nm, f.channels[0].emission_wavelength_nm), (Some(405.0), Some(461.0)));

    let p = scopeio::open(&sample("plain.dcm")).unwrap().series_meta(0).unwrap().clone();
    // Implicit VR; frames as z; pixel spacing row \ column.
    assert_eq!((p.size_z, p.pixel_type, p.physical_size.x_nm, p.physical_size.y_nm), (3, PixelType::I16, Some(250_000.0), Some(500_000.0)));
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in ["wsi/level1.dcm", "fluo/level0.dcm", "plain.dcm"] {
        common::corrupt_sweep(&sample(name), 13);
    }
}

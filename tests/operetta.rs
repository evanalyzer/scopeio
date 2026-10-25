//! # operetta
//!
//! Harmony export (Operetta / Opera Phenix) reader tests against the
//! reference readers and a synthetic export.
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
use scopeio::{FormatId, PixelType, Plane};

fn index() -> std::path::PathBuf {
    root().join("tests/data/operetta/Plate1__2024-01-01T00_00_00-Measurement 1/Images/Index.idx.xml")
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("operetta");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_export_matches_spec_truth() {
    let r = scopeio::open(&index()).unwrap();
    assert_eq!(r.format().id, FormatId::Operetta);
    for t in read_tsv(&root().join("tests/data/operetta/truth.tsv")) {
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(r.series_meta(s).unwrap().name.as_deref(), Some(t["name"].as_str()));
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "series {s}");
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&index()).unwrap();
    assert_eq!(r.series_count(), 4);
    let m = r.series_meta(1).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 3, 2, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.time_increment_s), (Some(650.0), Some(2000.0), Some(30.0)));
    assert_eq!(m.channels[1].name.as_deref(), Some("Alexa 488"));
    assert_eq!((m.channels[0].excitation_wavelength_nm, m.channels[0].emission_wavelength_nm, m.channels[0].exposure_s), (Some(375.0), Some(456.0), Some(0.05)));
    assert_eq!(m.stage_position.unwrap().x_nm, Some(200_000.0));
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 1)).unwrap().delta_t_s, Some(30.0));
}

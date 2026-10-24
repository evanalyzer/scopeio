//! # zvi
//!
//! Zeiss AxioVision ZVI reader tests against the reference readers and a
//! synthetic file.
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
    let failures = common::check_against_oracles("zvi");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/zvi").join(name)
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
    let r = scopeio::open(&sample("gray.zvi")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Zvi);
    let m = r.series_meta(0).unwrap().clone();
    // Stored indices z 13..15 and channels 2, 4 become 0-based positions.
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 3, 1, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(250.0), Some(1500.0)));
    assert_eq!((m.channels[1].name.as_deref(), m.channels[1].color, m.channels[1].exposure_s), (Some("FITC"), Some([0, 255, 0, 255]), Some(0.0455)));
    let o = m.objective.unwrap();
    assert_eq!((o.magnification, o.numerical_aperture), (Some(63.0), Some(1.4)));
    assert_eq!(m.acquisition_date.as_deref(), Some("2023-04-05T12:00:00"));
    // Relative time of the item at c = 1, z = 2 (item 5).
    assert_eq!(r.plane_meta(0, Plane::new(1, 2, 0)).unwrap().delta_t_s, Some(7.5));

    let c = scopeio::open(&sample("colour.zvi")).unwrap().series_meta(0).unwrap().clone();
    // Uncalibrated: no pixel size; the exposure from the image tags.
    assert_eq!((c.samples_per_pixel, c.physical_size.x_nm, c.channels[0].exposure_s), (3, None, Some(0.02)));
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in ["gray.zvi", "colour.zvi"] {
        common::corrupt_sweep(&sample(name), 13);
    }
}

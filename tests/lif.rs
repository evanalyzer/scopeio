//! # lif
//!
//! Leica LIF reader tests against the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::{crc, decode, read_tsv, root};
use scopeio::Plane;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("lif");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/lif").join(name)
}

#[test]
fn synthetic_files_match_spec_truth() {
    let mut failures = Vec::new();
    for t in read_tsv(&sample("truth.tsv")) {
        let series: usize = t["series"].parse().unwrap();
        match scopeio::open(&sample(&t["file"])).and_then(|r| decode(r.as_ref())) {
            Ok(d) if crc(&d[series].pixels) == t["crc32"] => {}
            Ok(d) => failures.push(format!("{} series {series}: {} != {}", t["file"], crc(&d[series].pixels), t["crc32"])),
            Err(e) => failures.push(format!("{}: {e}", t["file"])),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("planar.lif")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Lif);
    let m = r.series_meta(0).unwrap();
    assert_eq!(m.name.as_deref(), Some("stack"));
    assert_eq!((m.size_c, m.size_z, m.size_t, m.significant_bits), (2, 3, 2, Some(12)));
    assert!((m.physical_size.x_nm.unwrap() - 200.0).abs() < 1e-6);
    assert!((m.physical_size.z_nm.unwrap() - 1000.0).abs() < 1e-6);
    assert_eq!(m.time_increment_s, Some(30.0));
    assert_eq!(m.channels[1].color, Some([255, 0, 0, 255]));
    let o = m.objective.as_ref().unwrap();
    assert_eq!((o.model.as_deref(), o.magnification, o.immersion.as_deref()), (Some("HC PL APO 63x/1.40 OIL"), Some(63.0), Some("Oil")));
    assert!(m.acquisition_date.is_some());
    let rgb = r.series_meta(1).unwrap();
    assert_eq!((rgb.size_c, rgb.samples_per_pixel), (1, 3));

    let t = scopeio::open(&sample("tiles_lambda.lif")).unwrap();
    assert_eq!(t.series_count(), 4);
    let tile = t.series_meta(2).unwrap();
    assert_eq!(tile.name.as_deref(), Some("tiles [tile 3]"));
    assert!((tile.stage_position.unwrap().x_nm.unwrap() - 2e5).abs() < 1e-6);
    let l = t.series_meta(3).unwrap();
    assert_eq!((l.size_c, l.pixel_type), (3, scopeio::PixelType::F32));
    assert_eq!(l.channels[2].emission_wavelength_nm.map(f64::round), Some(540.0));
    assert_eq!(t.plane_meta(3, Plane::new(0, 0, 0)).unwrap().delta_t_s, None);
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in "planar.lif tiles_lambda.lif ".split_whitespace() {
        common::corrupt_sweep(&common::root().join("tests/data/lif").join(name), 7);
    }
}

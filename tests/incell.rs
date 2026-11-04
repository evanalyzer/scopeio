//! # incell
//!
//! IN Cell Analyzer reader tests against the reference readers and
//! synthetic acquisitions.
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
    let failures = common::check_against_oracles("incell");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/incell").join(name)
}

#[test]
fn synthetic_acquisitions_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::InCell, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("listed/plate.xdce")).unwrap();
    assert_eq!(r.series_count(), 4);
    let m = r.series_meta(3).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.name.as_deref()), (2, 2, 1, Some("well C04 field 2")));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(325.0), Some(1500.0)));
    assert_eq!(
        (m.channels[1].name.as_deref(), m.channels[1].emission_wavelength_nm, m.channels[1].exposure_s),
        (Some("FITC_511"), Some(511.0), Some(0.2))
    );
    let plate = &r.plates()[0];
    assert_eq!((plate.rows, plate.cols, plate.wells.len()), (8, 12, 4));
    assert_eq!((plate.wells[2].row, plate.wells[2].col, plate.wells[2].field), (2, 3, 0));
    let p = r.plane_meta(0, Plane::new(1, 1, 0)).unwrap();
    assert_eq!((p.delta_t_s, p.position.and_then(|p| p.z_nm)), (Some(11.0), Some(101_500.0)));
    let r = scopeio::open(&sample("named/plate.xdce")).unwrap();
    assert_eq!((r.series_count(), r.series_meta(0).unwrap().size_c), (1, 2));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("listed/plate.xdce"), 97);
}

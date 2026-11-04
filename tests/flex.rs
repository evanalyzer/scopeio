//! # flex
//!
//! Opera Flex reader tests against the reference readers and a synthetic
//! file.
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
    let failures = common::check_against_oracles("flex");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/flex").join(name)
}

#[test]
fn synthetic_well_matches_truth() {
    let r = scopeio::open(&sample("well.flex")).unwrap();
    assert_eq!((r.format().id, r.series_count()), (FormatId::Flex, 2));
    for t in read_tsv(&sample("truth.tsv")) {
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "series {s}");
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("well.flex")).unwrap();
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.name.as_deref(), m.physical_size.x_nm), (2, 2, Some("well C04 field 2"), Some(250.0)));
    assert_eq!(m.channels[1].exposure_s, Some(0.2));
    assert_eq!(m.objective.as_ref().and_then(|o| o.numerical_aperture), Some(0.7));
    let p = r.plane_meta(1, Plane::new(1, 1, 0)).unwrap();
    assert_eq!((p.delta_t_s, p.position.and_then(|p| p.z_nm)), (Some(17.0), Some(2000.0)));
    assert_eq!(r.plates()[0].wells[0].row, 2);
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("well.flex"), 41);
}

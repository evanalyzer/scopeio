//! # nd2
//!
//! Nikon ND2 reader tests against the reference readers.
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
    let failures = common::check_against_oracles("nd2");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode, read_tsv, root};
use scopeio::Plane;

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/nd2").join(name)
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
    let r = scopeio::open(&sample("zstack_2ch.nd2")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t), (2, 3, 1));
    assert_eq!(m.physical_size.x_nm, Some(500.0));
    assert!((m.physical_size.z_nm.unwrap() - 300.0).abs() < 1e-9);
    assert_eq!(m.channels[0].name.as_deref(), Some("DAPI"));
    // uiColor 0xFF0000 is blue: red is the lowest byte.
    assert_eq!(m.channels[0].color, Some([0, 0, 255, 255]));
    assert_eq!(m.acquisition_date.as_deref(), Some("2017-06-06T09:15:06Z"));
    assert_eq!(m.objective.as_ref().and_then(|o| o.magnification), Some(20.0));
    assert_eq!(r.plane_meta(0, Plane::new(0, 2, 0)).unwrap().delta_t_s, None);

    let p = scopeio::open(&sample("positions_time.nd2")).unwrap();
    assert_eq!(p.series_count(), 2);
    let m1 = p.series_meta(1).unwrap();
    assert_eq!(m1.time_increment_s, Some(60.0));
    // The second valid point is the third stored one.
    assert_eq!(m1.stage_position.and_then(|s| s.x_nm), Some(200_000.0));

    let v2 = scopeio::open(&sample("v2_xml_z.nd2")).unwrap();
    assert_eq!(v2.series_meta(0).unwrap().size_z, 2);
    assert_eq!(v2.format().id, scopeio::FormatId::Nd2);
}

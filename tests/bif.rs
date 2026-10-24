//! # bif
//!
//! Ventana BIF reader tests against the reference readers.
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
    let failures = common::check_against_oracles("bif");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::AssociatedKind;

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/bif");
    let r = scopeio::open(&dir.join("synthetic.bif")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Bif);
    let m = r.series_meta(0).unwrap();
    assert_eq!(m.physical_size.x_nm, Some(250.0));
    assert_eq!((m.size_z, m.physical_size.z_nm), (1, None));
    assert_eq!(m.objective.as_ref().and_then(|o| o.magnification), Some(40.0));
    assert_eq!(r.associated_images(), [AssociatedKind::Macro, AssociatedKind::Other("probability".into())]);
    // Focal planes near to far, 1.5 um apart.
    let v = scopeio::open(&dir.join("volumetric.bif")).unwrap();
    let m = v.series_meta(0).unwrap();
    assert_eq!((m.size_z, m.physical_size.z_nm), (3, Some(1500.0)));
    for t in read_tsv(&dir.join("truth.tsv")) {
        let r = scopeio::open(&dir.join(&t["file"])).unwrap();
        let what = format!("{} {}", t["file"], t["image"]);
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap(),
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap(),
            _ => {
                let img = r.read_associated(&AssociatedKind::Macro, &Default::default()).unwrap();
                assert_eq!(crc(img.as_bytes()), t["crc32"], "{what}");
                continue;
            }
        };
        assert_eq!(got.width.to_string(), t["width"], "{what}");
        assert_eq!(crc(&got.pixels), t["crc32"], "{what}");
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in ["synthetic.bif", "volumetric.bif", "iscan.bif"] {
        common::corrupt_sweep(&root().join("tests/data/bif").join(name), 13);
    }
}

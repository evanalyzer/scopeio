//! # qptiff
//!
//! Akoya QPTIFF reader tests against the reference readers.
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
    let failures = common::check_against_oracles("qptiff");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::AssociatedKind;

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/qptiff");
    let r = scopeio::open(&dir.join("synthetic.qptiff")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Qptiff);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.levels.len(), m.levels[1].size_x), (2, 2, 20));
    assert_eq!(m.physical_size.x_nm, Some(500.0));
    assert_eq!(m.channels[1].name.as_deref(), Some("FITC"));
    assert_eq!(m.channels[1].color, Some([0, 255, 0, 255]));
    assert_eq!(m.channels[0].exposure_s, Some(0.0025));
    assert_eq!(m.objective.as_ref().and_then(|o| o.model.as_deref()), Some("20x"));
    assert_eq!(r.associated_images(), [AssociatedKind::Thumbnail, AssociatedKind::Macro, AssociatedKind::Label]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap().pixels,
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap().pixels,
            kind => {
                let kind = match kind {
                    "thumbnail" => AssociatedKind::Thumbnail,
                    "macro" => AssociatedKind::Macro,
                    _ => AssociatedKind::Label,
                };
                r.read_associated(&kind, &Default::default()).unwrap().as_bytes().to_vec()
            }
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&root().join("tests/data/qptiff/synthetic.qptiff"), 7);
}

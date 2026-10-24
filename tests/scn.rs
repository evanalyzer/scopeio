//! # scn
//!
//! Leica SCN reader tests against the reference readers.
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
    let failures = common::check_against_oracles("scn");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::AssociatedKind;

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/scn");
    let r = scopeio::open(&dir.join("synthetic.scn")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Scn);
    assert_eq!(r.series_count(), 2);
    let b = r.series_meta(0).unwrap();
    assert_eq!((b.name.as_deref(), b.levels.len(), b.samples_per_pixel), (Some("bright"), 2, 3));
    assert_eq!(b.physical_size.x_nm, Some(500.0));
    assert_eq!(b.acquisition_date.as_deref(), Some("2021-04-05T06:07:08Z"));
    assert_eq!(b.objective.as_ref().and_then(|o| o.magnification), Some(20.0));
    assert_eq!(b.stage_position.and_then(|p| p.x_nm), Some(300_000.0));
    let f = r.series_meta(1).unwrap();
    assert_eq!((f.size_c, f.channels[1].name.as_deref(), f.channels[1].color), (2, Some("FITC"), Some([0, 255, 0, 255])));
    assert_eq!(f.channels[0].exposure_s, Some(0.02));
    assert_eq!(r.associated_images(), [AssociatedKind::Macro]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        let got = match t["image"].split_once('/') {
            Some((s, l)) => decode_level(r.as_ref(), s.parse().unwrap(), l.parse().unwrap()).unwrap().pixels,
            None => r.read_associated(&AssociatedKind::Macro, &Default::default()).unwrap().as_bytes().to_vec(),
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&root().join("tests/data/scn/synthetic.scn"), 11);
}

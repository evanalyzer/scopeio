//! # svs
//!
//! Aperio SVS reader tests against the reference readers.
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
    let failures = common::check_against_oracles("svs");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::AssociatedKind;

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/svs");
    let r = scopeio::open(&dir.join("synthetic.svs")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Svs);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.levels.len(), m.levels[1].size_x, m.levels[1].downsample), (2, 20, 2.0));
    assert_eq!(m.physical_size.x_nm, Some(250.0));
    assert_eq!(m.objective.as_ref().and_then(|o| o.magnification), Some(40.0));
    assert_eq!(m.acquisition_date.as_deref(), Some("2021-03-14T08:30:00"));
    assert_eq!(m.name.as_deref(), Some("synthetic"));
    assert_eq!(r.associated_images(), [AssociatedKind::Thumbnail, AssociatedKind::Label, AssociatedKind::Macro]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap().pixels,
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap().pixels,
            kind => {
                let kind = match kind {
                    "thumbnail" => AssociatedKind::Thumbnail,
                    "label" => AssociatedKind::Label,
                    _ => AssociatedKind::Macro,
                };
                r.read_associated(&kind, &Default::default()).unwrap().as_bytes().to_vec()
            }
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
}

//! # mrxs
//!
//! 3DHistech MIRAX reader tests against the reference readers.
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
    let failures = common::check_against_oracles("mrxs");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::AssociatedKind;

#[test]
fn synthetic_slide_matches_spec_truth() {
    let dir = root().join("tests/data/mrxs");
    let r = scopeio::open(&dir.join("synthetic.mrxs")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Mrxs);
    let m = r.series_meta(0).unwrap();
    // Two photos of 32 x 16 overlapping by 4 pixels, the second 4 lower.
    assert_eq!((m.levels.len(), m.levels[0].size_x, m.levels[0].size_y), (2, 60, 20));
    assert_eq!(m.physical_size.x_nm, Some(250.0));
    assert_eq!(m.acquisition_date.as_deref(), Some("2021-02-01T03:04:05"));
    assert_eq!(r.associated_images(), [AssociatedKind::Label]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap().pixels,
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap().pixels,
            _ => r.read_associated(&AssociatedKind::Label, &Default::default()).unwrap().as_bytes().to_vec(),
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
}

#[test]
fn corrupt_slides_do_not_panic() {
    // Damage each slide file in a copy of the slide folder.
    let src = root().join("tests/data/mrxs");
    let tmp = std::env::temp_dir().join(format!("scopeio-mrxs-{}", std::process::id()));
    for name in ["Index.dat", "Slidedat.ini", "Data0000.dat"] {
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("synthetic")).unwrap();
        std::fs::copy(src.join("synthetic.mrxs"), tmp.join("synthetic.mrxs")).unwrap();
        for f in ["Index.dat", "Slidedat.ini", "Data0000.dat"] {
            std::fs::copy(src.join("synthetic").join(f), tmp.join("synthetic").join(f)).unwrap();
        }
        let original = std::fs::read(src.join("synthetic").join(name)).unwrap();
        for at in (0..original.len()).step_by(5) {
            let mut d = original.clone();
            if at % 2 == 0 {
                d.truncate(at);
            } else {
                d[at] ^= 0x5A;
            }
            std::fs::write(tmp.join("synthetic").join(name), &d).unwrap();
            if let Ok(r) = scopeio::open(&tmp.join("synthetic.mrxs")) {
                common::decode_small(r.as_ref());
                let _ = r.read_associated(&AssociatedKind::Label, &Default::default());
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

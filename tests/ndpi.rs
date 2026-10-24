//! # ndpi
//!
//! Hamamatsu NDPI reader tests against the reference readers.
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
    let failures = common::check_against_oracles("ndpi");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::{AssociatedKind, ChannelSelect, Region, RegionRequest};

#[test]
fn synthetic_file_matches_spec_truth() {
    let dir = root().join("tests/data/ndpi");
    let r = scopeio::open(&dir.join("synthetic.ndpi")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Ndpi);
    let m = r.series_meta(0).unwrap();
    // Level 0 is read by restart interval: 2 MCUs of 8 x 8.
    assert_eq!((m.levels.len(), m.levels[0].tile, m.levels[1].tile), (2, Some((16, 8)), None));
    assert_eq!(m.physical_size.x_nm, Some(250.0));
    assert_eq!(m.objective.as_ref().and_then(|o| o.magnification), Some(40.0));
    assert_eq!(m.acquisition_date.as_deref(), Some("2021-05-06T07:08:09"));
    assert_eq!(r.associated_images(), [AssociatedKind::Macro]);
    for t in read_tsv(&dir.join("truth.tsv")) {
        let got = match t["image"].as_str() {
            "level0" => decode_level(r.as_ref(), 0, 0).unwrap().pixels,
            "level1" => decode_level(r.as_ref(), 0, 1).unwrap().pixels,
            _ => r.read_associated(&AssociatedKind::Macro, &Default::default()).unwrap().as_bytes().to_vec(),
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
    // A region across interval and row boundaries equals the same pixels of the full level.
    let full = decode_level(r.as_ref(), 0, 0).unwrap().pixels;
    let mut req = RegionRequest::new(0, 0, Region::new(13, 5, 30, 9));
    req.channels = ChannelSelect::Channel(0);
    let part = r.read_region(&req).unwrap();
    let expect: Vec<u8> = (5..14).flat_map(|y| full[(y * 64 + 13) * 3..(y * 64 + 43) * 3].to_vec()).collect();
    assert_eq!(part.as_bytes(), &expect[..]);
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in "synthetic.ndpi ".split_whitespace() {
        common::corrupt_sweep(&common::root().join("tests/data/ndpi").join(name), 7);
    }
}

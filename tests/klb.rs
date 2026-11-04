//! # klb
//!
//! Keller Lab Block reader tests against the reference readers and
//! synthetic files.
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
    let failures = common::check_against_oracles("klb");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{ChannelSelect, FormatId, PixelType, Region, RegionRequest};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/klb").join(name)
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Klb, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata_and_regions() {
    let r = scopeio::open(&sample("stack.klb")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type, m.levels[0].tile), (2, 3, 2, PixelType::U16, Some((4, 3))));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.time_increment_s), (Some(500.0), Some(2000.0), Some(2.5)));
    // A region across four blocks: x 3..6, y 2..4 of z 2, channel 1, t 1.
    let mut req = RegionRequest::new(0, 0, Region::new(3, 2, 3, 2));
    (req.z, req.t, req.channels) = (2, 1, ChannelSelect::Channel(1));
    let px = r.read_region(&req).unwrap();
    let v = |x: u64, y: u64| ((x * 1000 + y * 77 + 2 * 3000 + 20000 + 11) % 65536) as u16;
    let want: Vec<u8> = (2..4).flat_map(|y| (3..6).flat_map(move |x| v(x, y).to_le_bytes())).collect();
    assert_eq!(px.as_bytes(), want.as_slice());
    let r = scopeio::open(&sample("int8.klb")).unwrap();
    assert_eq!((r.series_meta(0).unwrap().size_c, r.series_meta(0).unwrap().pixel_type), (3, PixelType::I8));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("stack.klb"), 3);
    common::corrupt_sweep(&sample("plain.klb"), 5);
}

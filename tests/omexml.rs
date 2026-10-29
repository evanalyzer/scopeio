//! # omexml
//!
//! OME-XML (inline pixels) reader tests against the reference readers and
//! synthetic files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("omexml");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/omexml").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::OmeXml, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("zlib_be.ome.xml")).unwrap();
    assert_eq!(r.series_count(), 2);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.pixel_type, m.physical_size.x_nm), (2, 3, PixelType::I16, Some(500.0)));
    assert_eq!(m.channels[1].name.as_deref(), Some("GFP"));
    assert!(r.original_metadata(0).unwrap().documents[0].1.contains("<OME"));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("zlib_be.ome.xml"), 5);
    common::corrupt_sweep(&sample("bit.ome.xml"), 3);
}

//! # ome_zarr
//!
//! OME-Zarr reader tests against the spec-derived truth of the synthetic
//! stores and against the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::{crc, decode, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/zarr").join(name)
}

#[test]
fn synthetic_stores_match_spec_truth() {
    let mut failures = Vec::new();
    for t in read_tsv(&sample("truth.tsv")) {
        let series: usize = t["series"].parse().unwrap();
        let decoded = match scopeio::open(&sample(&t["file"])).and_then(|r| decode(r.as_ref())) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{}: {e}", t["file"]));
                continue;
            }
        };
        let s = &decoded[series];
        let got = (s.channels.to_string(), s.size_z.to_string(), s.size_t.to_string(), crc(&s.pixels));
        let want = (t["channels"].clone(), t["size_z"].clone(), t["size_t"].clone(), t["crc32"].clone());
        if got != want {
            failures.push(format!("{} series {series}: got {got:?}, want {want:?}", t["file"]));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("ome-zarr");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn metadata_and_plate_layout() {
    let r = scopeio::open(&sample("v2_zlib_tczyx.zarr")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t), (2, 3, 2));
    assert_eq!(m.physical_size.x_nm, Some(500.0));
    assert_eq!(m.time_increment_s, Some(1.0));
    assert_eq!(m.channels[1].name.as_deref(), Some("ch1"));
    assert_eq!(m.channels[1].color, Some([0, 255, 0, 255]));
    assert_eq!(m.levels.len(), 2);
    assert_eq!((m.levels[1].size_x, m.levels[1].downsample), (4, 1.75));

    let plate = scopeio::open(&sample("plate.zarr")).unwrap();
    assert_eq!(plate.series_count(), 4);
    let p = &plate.plates()[0];
    assert_eq!((p.name.as_deref(), p.rows, p.cols, p.wells.len()), (Some("test plate"), 3, 3, 4));
    let w = p.wells[3];
    assert_eq!((w.row, w.col, w.field, w.series), (2, 0, 1, 3));

    let b2r = scopeio::open(&sample("bioformats2raw.zarr")).unwrap();
    assert_eq!(b2r.series_count(), 2);
    let original = b2r.original_metadata(0).unwrap();
    assert!(original.documents.iter().any(|(name, _)| name == "OME-XML"));
}

#[test]
fn probe_accepts_store_and_metadata_file() {
    assert_eq!(scopeio::probe(&sample("v2_yx.zarr")).unwrap(), Some(scopeio::FormatId::OmeZarr));
    assert_eq!(scopeio::probe(&sample("v3_gzip_crc.zarr/zarr.json")).unwrap(), Some(scopeio::FormatId::OmeZarr));
    assert_eq!(scopeio::probe(&sample("v2_yx.zarr/0")).unwrap(), None);
}

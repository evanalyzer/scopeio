//! # omp2info
//!
//! Olympus OMP2INFO reader tests: synthetic acquisition and the public
//! multi-area acquisition (vendor stitch first, then the tiles).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::{crc, decode_level, read_tsv, root};

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("omp2info");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_acquisition() {
    let r = scopeio::open(&root().join("tests/data/omp2info/acq/acq.omp2info")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Omp2info);
    // Vendor stitch, then two tiles; all are copies of the synthetic OIR.
    assert_eq!(r.series_count(), 3);
    let oir = scopeio::open(&root().join("tests/data/oir/synthetic.oir")).unwrap();
    let expected = crc(&decode_level(oir.as_ref(), 0, 0).unwrap().pixels);
    for s in 0..3 {
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), expected, "series {s}");
    }
    assert!(r.series_meta(0).unwrap().name.as_deref().unwrap().starts_with("stitched"));
    let tile = r.series_meta(2).unwrap();
    assert!(tile.name.as_deref().unwrap().starts_with("tile x1 y0"));
    // Nominal position: origin + 1 x 100000 nm x (1 - 10 %).
    let p = tile.stage_position.unwrap();
    assert_eq!((p.x_nm.map(f64::round), p.y_nm.map(f64::round)), (Some(-910_000.0), Some(2_000_000.0)));
}

#[test]
fn public_acquisition() {
    let dir = root().join("tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725");
    if !dir.join("Stitch_A01_G001.oir").exists() || !dir.join("tiling-sample-brain-section_A01_G001_0025.oir").exists() {
        return;
    }
    let r = scopeio::open(&dir.join("matl.omp2info")).unwrap();
    assert_eq!(r.series_count(), 26);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_c, m.size_z), (3554, 2540, 2, 53));
    // The first tile equals Bio-Formats' reading of that OIR file.
    let reference = read_tsv(&root().join("tests/expected/oir.tsv"))
        .into_iter()
        .find(|t| t["file"].ends_with("tiling-sample-brain-section_A01_G001_0001.oir") && t["tool"] == "bioformats")
        .unwrap();
    assert_eq!(crc(&decode_level(r.as_ref(), 1, 0).unwrap().pixels), reference["crc32"]);
}

#[test]
fn corrupt_index_does_not_panic() {
    let src = std::fs::read(root().join("tests/data/omp2info/acq/acq.omp2info")).unwrap();
    let dir = std::env::temp_dir().join(format!("scopeio-omp2info-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for cut in (0..src.len()).step_by(41) {
        std::fs::write(dir.join("x.omp2info"), &src[..cut]).unwrap();
        let _ = scopeio::open(&dir.join("x.omp2info"));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

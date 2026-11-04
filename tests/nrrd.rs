//! # nrrd
//!
//! NRRD reader tests against the
//! reference readers and synthetic files.
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
    let failures = common::check_against_oracles("nrrd");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}



use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/nrrd").join(name)
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Nrrd, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let m = scopeio::open(&sample("scalar.nrrd")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.levels[0].size_x, m.pixel_type), (1, 3, 7, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(500.0), Some(2000.0)));
    let m = scopeio::open(&sample("rgb.nhdr")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.levels[0].size_x, m.levels[0].size_y, m.physical_size.x_nm), (3, 7, 5, Some(250.0)));
    let m = scopeio::open(&sample("text.nrrd")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.time_increment_s, m.pixel_type), (2, 1, 4, Some(2.0), PixelType::I16));
    let m = scopeio::open(&sample("tail.nhdr")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_c, m.pixel_type, m.physical_size.x_nm), (1, PixelType::U32, None));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("scalar.nrrd"), 5);
    common::corrupt_sweep(&sample("text.nrrd"), 11);
    // Detached headers corrupted next to their data files.
    for name in ["rgb.nhdr", "hex.nhdr", "tail.nhdr"] {
        let src = sample(name);
        let data = std::fs::read(&src).unwrap();
        let target = sample(&format!("sweep-{}-{name}", std::process::id()));
        for at in (0..data.len()).step_by(3) {
            let mut d = data.clone();
            if at % 2 == 0 {
                d.truncate(at);
            } else {
                d[at] ^= 0xA5;
            }
            std::fs::write(&target, &d).unwrap();
            if let Ok(r) = scopeio::open(&target) {
                let _ = decode_level(r.as_ref(), 0, 0);
            }
        }
        std::fs::remove_file(&target).unwrap();
    }
}

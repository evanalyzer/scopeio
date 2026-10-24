//! # bmp
//!
//! BMP reader tests against the spec-derived truth of the synthetic samples
//! and against the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use scopeio::{Plane, Region};

use common::{crc, decode, read_tsv, root};

#[test]
fn synthetic_samples_match_spec_truth() {
    let dir = root().join("tests/data/bmp");
    let truth = read_tsv(&dir.join("truth.tsv"));
    assert!(!truth.is_empty());
    for t in truth {
        let reader = scopeio::open(&dir.join(&t["file"])).unwrap_or_else(|e| panic!("{}: {e}", t["file"]));
        let series = decode(reader.as_ref()).unwrap();
        assert_eq!(series[0].channels.to_string(), t["samples"], "{}", t["file"]);
        assert_eq!(crc(&series[0].pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("bmp");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn regions_equal_crops_of_full_image() {
    for entry in std::fs::read_dir(root().join("tests/data/bmp")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "bmp") {
            continue;
        }
        let reader = scopeio::open(&path).unwrap();
        let meta = reader.series_meta(0).unwrap();
        let (w, h) = (meta.levels[0].size_x, meta.levels[0].size_y);
        let px = meta.samples_per_pixel as usize;
        let full = common::read(reader.as_ref(), 0, 0, Plane::default(), Region::new(0, 0, w, h)).unwrap();
        for region in [
            Region::new(1, 1, w - 1, h - 1),
            Region::new(0, 0, 1, 1),
            Region::new(w - 1, h - 1, 1, 1),
            Region::new(w / 2, 0, w - w / 2, h),
            Region::new(0, h / 2, w, 1),
        ] {
            let part = common::read(reader.as_ref(), 0, 0, Plane::default(), region).unwrap();
            let expected: Vec<u8> = (region.y..region.y + region.height)
                .flat_map(|y| {
                    let start = (y as usize * w as usize + region.x as usize) * px;
                    full[start..start + region.width as usize * px].iter().copied()
                })
                .collect();
            assert_eq!(part, expected, "{} {region:?}", path.display());
        }
    }
}

#[test]
fn metadata_of_rgba_and_gray_samples() {
    let dir = root().join("tests/data/bmp");
    let rgba = scopeio::open(&dir.join("rgba32_v5.bmp")).unwrap();
    let meta = rgba.series_meta(0).unwrap();
    assert_eq!(meta.samples_per_pixel, 4);
    assert_eq!((meta.levels[0].size_x, meta.levels[0].size_y), (4, 3));
    let nm = meta.physical_size.x_nm.unwrap();
    assert!((nm - 1e9 / 2835.0).abs() < 1e-6);

    let gray = scopeio::open(&dir.join("pal8_gray_topdown.bmp")).unwrap();
    assert_eq!(gray.series_meta(0).unwrap().samples_per_pixel, 1);
    let px: Vec<u8> = common::read(gray.as_ref(), 0, 0, Plane::default(), Region::new(0, 0, 6, 1)).unwrap();
    // Index (x * 3) % 256 in the top row of the identity palette.
    assert_eq!(px, vec![0, 3, 6, 9, 12, 15]);
}

#[test]
fn rejects_invalid_requests_and_files() {
    let dir = root().join("tests/data/bmp");
    let reader = scopeio::open(&dir.join("rgb24_odd_width.bmp")).unwrap();
    assert!(common::read(reader.as_ref(), 0, 0, Plane::new(1, 0, 0), Region::new(0, 0, 1, 1)).is_err());
    assert!(common::read(reader.as_ref(), 0, 0, Plane::default(), Region::new(2, 0, 2, 1)).is_err());
    assert!(common::read(reader.as_ref(), 1, 0, Plane::default(), Region::new(0, 0, 1, 1)).is_err());

    let truncated = std::env::temp_dir().join(format!("scopeio-truncated-{}.bmp", std::process::id()));
    let bytes = std::fs::read(dir.join("rgb24_odd_width.bmp")).unwrap();
    std::fs::write(&truncated, &bytes[..bytes.len() - 4]).unwrap();
    assert!(matches!(scopeio::open(&truncated), Err(scopeio::Error::Corrupt(_))));
    std::fs::remove_file(truncated).unwrap();
}

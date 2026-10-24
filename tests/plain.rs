//! # plain
//!
//! PNG, JPEG and JPEG 2000 reader tests against the spec-derived truth of
//! the synthetic samples and the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane, Region};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/plain").join(name)
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("plain");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_samples_match_spec_truth() {
    let truth = read_tsv(&sample("truth.tsv"));
    assert!(truth.len() >= 10);
    for t in truth {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        let got = decode_level(r.as_ref(), 0, 0).unwrap().pixels;
        assert_eq!(crc(&got), t["crc32"], "{}", t["file"]);
    }
}

fn meta(name: &str) -> scopeio::SeriesMeta {
    scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone()
}

#[test]
fn metadata() {
    let id = |name: &str| scopeio::open(&sample(name)).unwrap().format().id;
    assert_eq!((id("rgb8.png"), id("rgb8.jpg"), id("rgb8.jp2"), id("gray16.j2k")), (FormatId::Png, FormatId::Jpeg, FormatId::Jpeg2000, FormatId::Jpeg2000));
    let m = meta("rgb8.png");
    // pHYs of 40000 pixels per centimetre.
    assert_eq!((m.samples_per_pixel, m.size_c, m.physical_size.x_nm), (3, 1, Some(250.0)));
    let m = meta("graya8.png");
    assert_eq!((m.samples_per_pixel, m.size_c), (1, 2));
    let m = meta("gray16.png");
    assert_eq!(m.pixel_type, PixelType::U16);
    // JFIF density of 254 dots per inch.
    let m = meta("rgb8.jpg");
    let nm = m.physical_size.x_nm.unwrap();
    assert!((nm - 100_000.0).abs() < 1e-6, "{nm}");
    let m = meta("gray16.j2k");
    assert_eq!((m.pixel_type, m.levels[0].size_x, m.levels[0].size_y), (PixelType::U16, 37, 23));
}

#[test]
fn regions_equal_crops_of_full_image() {
    for name in ["rgb8.png", "rgb16.png", "rgb8_interlaced.png", "gray1.png", "gray2_interlaced.png", "gray8.jpg", "rgb8.jp2", "gray16.j2k"] {
        let r = scopeio::open(&sample(name)).unwrap();
        let m = r.series_meta(0).unwrap();
        let (w, h) = (m.levels[0].size_x, m.levels[0].size_y);
        let px = m.samples_per_pixel as usize * m.pixel_type.bytes();
        let full = common::read(r.as_ref(), 0, 0, Plane::default(), Region::new(0, 0, w, h)).unwrap();
        for region in [Region::new(1, 1, w - 1, h - 1), Region::new(w - 1, h - 1, 1, 1), Region::new(5, 7, 9, 3), Region::new(0, h / 2, w, 1)] {
            let part = common::read(r.as_ref(), 0, 0, Plane::default(), region).unwrap();
            let expected: Vec<u8> = (region.y..region.y + region.height)
                .flat_map(|y| {
                    let start = (y as usize * w as usize + region.x as usize) * px;
                    full[start..start + region.width as usize * px].to_vec()
                })
                .collect();
            assert_eq!(part, expected, "{name} {region:?}");
        }
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    for name in ["rgb8.png", "rgb8_interlaced.png", "palette.png", "rgb8.jpg", "rgb8.jp2", "gray16.j2k"] {
        common::corrupt_sweep(&sample(name), 7);
    }
}

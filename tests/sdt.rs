//! # sdt
//!
//! Becker & Hickl SDT reader tests against the reference readers and
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
    let failures = common::check_against_oracles("sdt");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane, Region};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/sdt").join(name)
}

#[test]
fn synthetic_blocks_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Sdt, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("image.sdt")).unwrap();
    assert_eq!(r.series_count(), 2);
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_t, m.pixel_type), (7, 5, 8, PixelType::U16));
    // Bin width tac_r / (tac_g * adc_re).
    assert!((m.time_increment_s.unwrap() - 5e-8 / 32.0).abs() < 1e-15);
    assert_eq!(m.acquisition_date.as_deref(), Some("2026-11-04T10:11:12"));
    let m = scopeio::open(&sample("decay.sdt")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.size_t), (1, 1, 16));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("image.sdt"), 11);
    common::corrupt_sweep(&sample("decay.sdt"), 5);
}

#[test]
fn zip_regions_in_any_order_equal_crops() {
    // Block 1 is ZIP: regions are decoded per request and cached; crops
    // inside a cached region and new regions must give the same pixels.
    let r = scopeio::open(&sample("image.sdt")).unwrap();
    let full: Vec<Vec<u8>> = (0..8).map(|t| common::read(r.as_ref(), 1, 0, Plane::new(0, 0, t), Region::new(0, 0, 7, 5)).unwrap()).collect();
    let crop = |t: u32, x: u64, y: u64, w: u64, h: u64| -> Vec<u8> {
        (y..y + h).flat_map(|row| full[t as usize][(row * 7 + x) as usize * 2..(row * 7 + x + w) as usize * 2].to_vec()).collect()
    };
    // Inside the cached full block, then a new small region, then outside it.
    for (t, x, y, w, h) in [(3, 1, 1, 4, 3), (7, 2, 0, 3, 5), (0, 5, 2, 2, 2), (5, 0, 4, 7, 1), (6, 1, 1, 6, 4)] {
        let got = common::read(r.as_ref(), 1, 0, Plane::new(0, 0, t), Region::new(x, y, w, h)).unwrap();
        assert_eq!(got, crop(t, x, y, w, h), "plane {t} region {x},{y} {w}x{h}");
    }
}

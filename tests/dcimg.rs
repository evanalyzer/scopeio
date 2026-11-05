//! # dcimg
//!
//! Hamamatsu DCIMG reader tests against the reference readers and
//! synthetic files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("dcimg");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane};

/// `name` (`<stem>.dcimg`) in its own folder, or `truth.tsv`.
fn sample(name: &str) -> std::path::PathBuf {
    let dir = root().join("tests/data/dcimg");
    match name.strip_suffix(".dcimg") {
        Some(stem) if !stem.contains('/') => dir.join(stem).join(name),
        _ => dir.join(name),
    }
}

fn original(r: &dyn scopeio::ImageReader, key: &str) -> Option<String> {
    r.original_metadata(0).unwrap().pairs.into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Dcimg, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    // Packed: footer time stamps 0.25 s apart, stored pixels in row 2.
    let r = scopeio::open(&sample("packed_stored.dcimg")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_t, m.size_z, m.size_c, m.levels[0].size_x, m.levels[0].size_y, m.pixel_type), (3, 1, 1, 16, 6, PixelType::U16));
    assert_eq!((m.time_increment_s, m.acquisition_date.as_deref()), (Some(0.25), Some("2024-09-05T16:53:17.000000Z")));
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 2)).unwrap().delta_t_s, Some(0.5));
    assert_eq!(original(r.as_ref(), "StoredPixels").as_deref(), Some("row 2 column 1 count 4"));
    assert_eq!(original(r.as_ref(), "FrameLayout").as_deref(), Some("packed"));
    // Framed: trailer stamps, camera block.
    let r = scopeio::open(&sample("framed_stored.dcimg")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_t, m.time_increment_s), (4, Some(1.1)));
    assert_eq!(original(r.as_ref(), "CameraModel").as_deref(), Some("C11440-22C"));
    assert_eq!(original(r.as_ref(), "CameraSerial").as_deref(), Some("001634"));
    assert_eq!(original(r.as_ref(), "SubArray").as_deref(), Some("x 0 width 24 y 940 height 5"));
    assert_eq!(original(r.as_ref(), "Binning").as_deref(), Some("2"));
    // A dropped frame shows in the counters.
    let r = scopeio::open(&sample("framed_plain.dcimg")).unwrap();
    assert_eq!(original(r.as_ref(), "FrameCounters").as_deref(), Some("0 - 2"));
    assert_eq!(original(r.as_ref(), "StoredPixels"), None);
    // A truncated file keeps its complete frames.
    assert_eq!(scopeio::open(&sample("truncated.dcimg")).unwrap().series_meta(0).unwrap().size_t, 2);
}

#[test]
fn regions_equal_crops_of_the_plane() {
    // Crops through the stored-pixel row, bands and the last row.
    let r = scopeio::open(&sample("framed_stored.dcimg")).unwrap();
    let full = common::read(r.as_ref(), 0, 0, Plane::new(0, 0, 3), scopeio::Region::new(0, 0, 12, 5)).unwrap();
    for (x, y, w, h) in [(0, 0, 3, 2), (1, 1, 5, 3), (2, 4, 10, 1), (0, 0, 12, 5)] {
        let crop = common::read(r.as_ref(), 0, 0, Plane::new(0, 0, 3), scopeio::Region::new(x, y, w, h)).unwrap();
        let want: Vec<u8> = (y..y + h).flat_map(|row| full[(row * 12 + x) as usize * 2..(row * 12 + x + w) as usize * 2].to_vec()).collect();
        assert_eq!(crop, want, "region {x},{y} {w}x{h}");
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    for (name, step) in [("packed_stored.dcimg", 3), ("framed_stored.dcimg", 3), ("framed_plain.dcimg", 5), ("packed_u8.dcimg", 2)] {
        common::corrupt_sweep(&sample(name), step);
    }
}

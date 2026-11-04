//! # avi
//!
//! AVI reader tests against the reference readers and synthetic files.
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
    let failures = common::check_against_oracles("avi");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{ChannelSelect, Error, FormatId, PixelType, Region, RegionRequest};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/avi").join(name)
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let name = &t["file"];
        let r = scopeio::open(&sample(name)).unwrap();
        assert_eq!(r.format().id, FormatId::Avi, "{name}");
        let m = r.series_meta(0).unwrap();
        assert_eq!((m.size_t.to_string(), m.samples_per_pixel.to_string()), (t["frames"].clone(), t["samples"].clone()), "{name}");
        assert_eq!((m.size_c, m.size_z, m.pixel_type), (1, 1, PixelType::U8), "{name}");
        let px = decode_level(r.as_ref(), 0, 0).unwrap().pixels;
        if t["crc32"] == "-" {
            // Motion JPEG is lossy: the mean of the source samples is kept.
            let mean = px.iter().map(|&v| v as f64).sum::<f64>() / px.len() as f64;
            let want: f64 = t["mean"].parse().unwrap();
            assert!((mean - want).abs() < 2.0, "{name}: mean {mean}, source {want}");
        } else {
            assert_eq!(crc(&px), t["crc32"], "{name}");
        }
    }
}

#[test]
fn motion_jpeg_without_huffman_tables_uses_the_standard_tables() {
    let decode = |name: &str| decode_level(scopeio::open(&sample(name)).unwrap().as_ref(), 0, 0).unwrap().pixels;
    assert_eq!(decode("mjpeg.avi"), decode("mjpeg_nodht.avi"));
}

#[test]
fn metadata() {
    let r = scopeio::open(&sample("bgr24.avi")).unwrap();
    let m = r.series_meta(0).unwrap();
    // T from the index, not avih.dwTotalFrames (99).
    assert_eq!((m.size_t, m.time_increment_s, m.physical_size.x_nm, m.name.as_deref()), (3, Some(0.1), None, Some("video")));
    let o = r.original_metadata(0).unwrap();
    let get = |k: &str| o.pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!((get("avih TotalFrames"), get("stream 0 fccHandler"), get("INFO ISFT")), (Some("99"), Some("DIB "), Some("make_avi_samples")));
    let m = scopeio::open(&sample("opendml.avi")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!((m.size_t, m.time_increment_s), (5, Some(0.04)));
    // MJPEG with fccHandler 'DIB ': the codec comes from biCompression.
    let r = scopeio::open(&sample("mjpeg.avi")).unwrap();
    assert_eq!(
        r.original_metadata(0).unwrap().pairs.iter().find(|(k, _)| k == "stream 0 strf Compression fourcc").map(|p| p.1.as_str()),
        Some("MJPG")
    );
    // The audio stream is ignored.
    assert_eq!(scopeio::open(&sample("absidx.avi")).unwrap().series_count(), 1);
}

#[test]
fn regions_match_full_frames() {
    for name in ["bgr24.avi", "topdown24.avi", "gray8.avi", "y800.avi", "rle8.avi", "mjpeg.avi", "opendml.avi", "dropped.avi"] {
        let r = scopeio::open(&sample(name)).unwrap();
        let m = r.series_meta(0).unwrap().clone();
        let (w, h, s) = (m.levels[0].size_x, m.levels[0].size_y, m.samples_per_pixel as u64);
        for t in 0..m.size_t {
            let mut full = RegionRequest::new(0, 0, Region::new(0, 0, w, h));
            (full.t, full.channels) = (t, ChannelSelect::Channel(0));
            let all = r.read_region(&full).unwrap().as_bytes().to_vec();
            let (rx, ry, rw, rh) = (1, 1, w - 2, h - 1);
            let mut part = RegionRequest::new(0, 0, Region::new(rx, ry, rw, rh));
            (part.t, part.channels) = (t, ChannelSelect::Channel(0));
            let got = r.read_region(&part).unwrap().as_bytes().to_vec();
            let want: Vec<u8> = (ry..ry + rh).flat_map(|y| all[((y * w + rx) * s) as usize..((y * w + rx + rw) * s) as usize].to_vec()).collect();
            assert_eq!(got, want, "{name} t {t}");
        }
    }
}

#[test]
fn errors_are_clean() {
    match scopeio::open(&sample("h264.avi")) {
        Err(Error::UnsupportedVariant(m)) => assert!(m.contains("H264") && m.contains("OME-TIFF"), "{m}"),
        Err(e) => panic!("expected an unsupported codec, got {e}"),
        Ok(_) => panic!("H.264 must not open"),
    }
    // A file cut in its second frame keeps its complete first frame.
    let r = scopeio::open(&sample("truncated.avi")).unwrap();
    assert_eq!(r.series_meta(0).unwrap().size_t, 1);
    assert!(decode_level(r.as_ref(), 0, 0).is_ok());
}

#[test]
fn corrupt_files_do_not_panic() {
    for (name, step) in [("bgr24.avi", 3), ("mjpeg_nodht.avi", 7), ("rle4.avi", 3), ("opendml.avi", 3), ("noidx.avi", 5)] {
        common::corrupt_sweep(&sample(name), step);
    }
}

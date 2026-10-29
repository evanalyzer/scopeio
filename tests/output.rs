//! # output
//!
//! Output options: scaled f32 types, sample selection from one decode and
//! the format list.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use common::root;
use scopeio::{OutputType, Region, RegionRequest};

fn read<T: scopeio::Pixel>(file: &str, dtype: OutputType) -> Vec<T> {
    let r = scopeio::open(&root().join("tests/data").join(file)).unwrap();
    let l = &r.series_meta(0).unwrap().levels[0];
    let mut req = RegionRequest::new(0, 0, Region::new(0, 0, l.size_x, l.size_y));
    req.output.dtype = dtype;
    r.read_region(&req).unwrap().into_vec().unwrap()
}

#[test]
fn significant_bits_scale_12_bit_data() {
    let meta = scopeio::open(&root().join("tests/data/tiff/u12_le.tif")).unwrap().series_meta(0).unwrap().clone();
    assert_eq!(meta.significant_bits, Some(12));
    let native: Vec<u16> = read("tiff/u12_le.tif", OutputType::Native);
    let f: Vec<f32> = read("tiff/u12_le.tif", OutputType::F32SignificantBits);
    assert!(native.iter().any(|&v| v > 2048));
    for (n, v) in native.iter().zip(&f) {
        assert!((v - *n as f32 / 4095.0).abs() < 1e-6, "{n} -> {v}");
    }
    // Without a significant-bits tag the type's bits are used.
    let full: Vec<f32> = read("tiff/u16_be.tif", OutputType::F32SignificantBits);
    let norm: Vec<f32> = read("tiff/u16_be.tif", OutputType::F32Normalized);
    assert_eq!(full, norm);
}

#[test]
fn scaled_output_is_not_clamped() {
    let native: Vec<u16> = read("tiff/u12_le.tif", OutputType::Native);
    let f: Vec<f32> = read("tiff/u12_le.tif", OutputType::F32Scaled { scale: 0.5, offset: -100.0 });
    for (n, v) in native.iter().zip(&f) {
        assert_eq!(*v, *n as f32 * 0.5 - 100.0);
    }
    assert!(f.iter().any(|&v| v < 0.0) && f.iter().any(|&v| v > 1.0));
}

#[test]
fn shifted_normalization_maps_signed_range_to_unit() {
    let native: Vec<i8> = read("tiff/i8_be.tif", OutputType::Native);
    let f: Vec<f32> = read("tiff/i8_be.tif", OutputType::F32NormalizedShifted);
    assert!(native.iter().any(|&v| v < 0));
    for (n, v) in native.iter().zip(&f) {
        assert!((v - (*n as f32 + 128.0) / 255.0).abs() < 1e-6);
        assert!((0.0..=1.0).contains(v));
    }
}

use scopeio::{ChannelSelect, Layout, OutputSpec};

/// Reads all of `file` with `channels` in `layout`; also returns the
/// number of pixels.
fn rgb_read(file: &str, channels: ChannelSelect, layout: Layout) -> (scopeio::Result<Vec<u8>>, usize) {
    let r = scopeio::open(&root().join("tests/data").join(file)).unwrap();
    let l = &r.series_meta(0).unwrap().levels[0];
    let n = (l.size_x * l.size_y) as usize;
    let mut req = RegionRequest::new(0, 0, Region::new(0, 0, l.size_x, l.size_y));
    req.channels = channels;
    req.output = OutputSpec { layout, ..Default::default() };
    (r.read_region(&req).and_then(|b| b.into_vec()), n)
}

#[test]
fn samples_select_planes_from_one_read() {
    let (hwc, n) = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Channel(0), Layout::Hwc);
    let hwc = hwc.unwrap();
    // Blue then red, planar.
    let chw = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Samples { channel: 0, samples: vec![2, 0] }, Layout::Chw).0.unwrap();
    assert_eq!(chw.len(), 2 * n);
    for i in 0..n {
        assert_eq!((chw[i], chw[n + i]), (hwc[3 * i + 2], hwc[3 * i]), "pixel {i}");
    }
    // Interleaved, and the same as single Sample reads.
    let two = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Samples { channel: 0, samples: vec![1, 2] }, Layout::Hwc).0.unwrap();
    let green = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Sample { channel: 0, sample: 1 }, Layout::Hwc).0.unwrap();
    for i in 0..n {
        assert_eq!((two[2 * i], two[2 * i + 1]), (green[i], hwc[3 * i + 2]));
    }
}

#[test]
fn samples_of_gray_and_invalid_selections() {
    let gray = rgb_read("tiff/u4_be.tif", ChannelSelect::Samples { channel: 0, samples: vec![0] }, Layout::Chw).0.unwrap();
    let plain = rgb_read("tiff/u4_be.tif", ChannelSelect::Channel(0), Layout::Chw).0.unwrap();
    assert_eq!(gray, plain);
    for samples in [vec![], vec![0, 1, 2, 0, 1], vec![3]] {
        let bad = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Samples { channel: 0, samples }, Layout::Chw).0;
        assert!(matches!(bad, Err(scopeio::Error::OutOfRange(_))));
    }
    let bad = rgb_read("tiff/rgb8_tiles.tif", ChannelSelect::Samples { channel: 1, samples: vec![0] }, Layout::Chw).0;
    assert!(matches!(bad, Err(scopeio::Error::OutOfRange(_))));
}

#[test]
fn every_format_lists_names() {
    for f in scopeio::supported_formats() {
        assert!(!f.extensions.is_empty() || !f.file_names.is_empty(), "{} lists no names", f.name);
        assert!(f.extensions.iter().all(|e| !e.starts_with('.') && e.to_ascii_lowercase() == *e), "{}", f.name);
    }
    let by_id = |id| scopeio::supported_formats().iter().find(|f| f.id == id).unwrap();
    let ome = by_id(scopeio::FormatId::OmeTiff);
    assert!(ome.matches_name("cells.OME.TIF") && !ome.matches_name("cells.tif") && !ome.matches_name("ome.tif"));
    assert!(by_id(scopeio::FormatId::Nifti).matches_name("brain.nii.gz"));
    assert!(!by_id(scopeio::FormatId::Tiff).matches_name("image.xtif"));
    let operetta = by_id(scopeio::FormatId::Operetta);
    assert!(operetta.matches_name("index.idx.xml") && !operetta.opens_folder);
    assert!(by_id(scopeio::FormatId::OmeZarr).opens_folder);
}

#[test]
fn sample_files_match_their_format_names() {
    // Every committed sample that opens is named as its format declares.
    let mut checked = 0;
    let mut stack = vec![root().join("tests/data")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            // The opened reader names the flavour (probe says TIFF for STK).
            let Some(id) = scopeio::open(&path).ok().map(|r| r.format().id) else {
                if path.is_dir() {
                    stack.push(path);
                }
                continue;
            };
            let f = scopeio::supported_formats().iter().find(|f| f.id == id).unwrap();
            // NDTiff dataset folders have no naming convention.
            let free_name = path.is_dir() && id == scopeio::FormatId::MicroManager;
            assert!(free_name || f.matches_name(&name), "{} detected as {} but not named so", path.display(), f.name);
            checked += 1;
            if path.is_dir() && !f.opens_folder {
                stack.push(path);
            }
        }
    }
    assert!(checked > 100, "only {checked} samples");
}

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

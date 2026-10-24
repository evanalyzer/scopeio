//! # tiff
//!
//! TIFF reader tests against the spec-derived truth of the synthetic samples
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

use scopeio::{PixelType, Plane, Region};

use common::{crc, decode, read_tsv, root};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/tiff").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    let truth = read_tsv(&sample("truth.tsv"));
    assert!(!truth.is_empty());
    let mut failures = Vec::new();
    for t in truth {
        let name = &t["file"];
        let series: usize = t["series"].parse().unwrap();
        let result = scopeio::open(&sample(name)).and_then(|r| {
            let levels = r.series_meta(series)?.levels.len();
            decode(r.as_ref()).map(|s| (s, levels))
        });
        let (decoded, levels) = match result {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        let s = &decoded[series];
        let got = (
            s.channels.to_string(),
            s.size_z.to_string(),
            s.size_t.to_string(),
            s.pixel_type.clone(),
            levels.to_string(),
            crc(&s.pixels),
        );
        let want = (
            t["channels"].clone(),
            t["size_z"].clone(),
            t["size_t"].clone(),
            t["pixel_type"].clone(),
            t["levels"].clone(),
            t["crc32"].clone(),
        );
        if got != want {
            failures.push(format!("{name} series {series}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("tiff");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn regions_equal_crops_of_full_image() {
    for entry in std::fs::read_dir(root().join("tests/data/tiff")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "tif") {
            continue;
        }
        let reader = scopeio::open(&path).unwrap();
        for series in 0..reader.series_count() {
            let meta = reader.series_meta(series).unwrap();
            let (w, h) = (meta.levels[0].size_x, meta.levels[0].size_y);
            let px = meta.samples_per_pixel as usize * meta.pixel_type.bytes();
            let plane = Plane::new(meta.size_c - 1, 0, meta.size_t - 1);
            let full = common::read(reader.as_ref(), series, 0, plane, Region::new(0, 0, w, h)).unwrap();
            for region in [
                Region::new(1, 1, w - 1, h - 1),
                Region::new(w as u64 - 1, h as u64 - 1, 1, 1),
                Region::new(w as u64 / 3, h as u64 / 2, w / 2, h / 2),
                Region::new(0, h as u64 - 2, w, 2),
            ] {
                let part = common::read(reader.as_ref(), series, 0, plane, region).unwrap();
                let expected: Vec<u8> = (region.y..region.y + region.height as u64)
                    .flat_map(|y| {
                        let start = (y as usize * w as usize + region.x as usize) * px;
                        full[start..start + region.width as usize * px].iter().copied()
                    })
                    .collect();
                assert_eq!(part, expected, "{} series {series} {region:?}", path.display());
            }
        }
    }
}

#[test]
fn imagej_dimensions_and_calibration() {
    let r = scopeio::open(&sample("imagej_hyperstack.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t), (2, 3, 2));
    assert_eq!(m.physical_size.x_nm, Some(200.0));
    assert_eq!(m.physical_size.z_nm, Some(250.0));
    assert_eq!(m.time_increment_s, Some(2.5));
}

#[test]
fn pyramid_levels() {
    for name in ["pyramid_subifd.tif", "pyramid_reduced.tif"] {
        let r = scopeio::open(&sample(name)).unwrap();
        let m = r.series_meta(0).unwrap();
        let sizes: Vec<_> = m.levels.iter().map(|l| (l.size_x, l.size_y, l.downsample)).collect();
        assert_eq!(sizes, vec![(32, 24, 1.0), (16, 12, 2.0), (8, 6, 4.0)], "{name}");
        let px: Vec<u8> = common::read(r.as_ref(), 0, 2, Plane::default(), Region::new(0, 0, 8, 6)).unwrap();
        assert_eq!(px.len(), 48);
    }
    // Level 2 of the SubIFD sample holds (x * 4 + y) % 256.
    let r = scopeio::open(&sample("pyramid_subifd.tif")).unwrap();
    let px: Vec<u8> = common::read(r.as_ref(), 0, 2, Plane::default(), Region::new(1, 2, 2, 1)).unwrap();
    assert_eq!(px, vec![6, 10]);
}

#[test]
fn palette_with_colour_map_expands_to_rgb() {
    let r = scopeio::open(&sample("palette8.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.samples_per_pixel, m.pixel_type), (1, 3, PixelType::U8));
    assert_eq!(m.channels[0].lut, None);
    let r = scopeio::open(&sample("palette8_rgb16.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.samples_per_pixel, m.pixel_type, m.significant_bits), (3, PixelType::U16, None));
}

#[test]
fn palette_ramps_keep_indices_as_intensities() {
    let r = scopeio::open(&sample("palette4_gray.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.samples_per_pixel, m.pixel_type), (1, PixelType::U8));
    assert_eq!(m.channels[0].color, None);
    assert_eq!(m.channels[0].lut.as_ref().unwrap()[15], [0xFFFF; 3]);

    let r = scopeio::open(&sample("palette8_green.tif")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.samples_per_pixel, m.pixel_type), (1, PixelType::U8));
    assert_eq!(m.channels[0].color, Some([0, 255, 0, 255]));
    assert_eq!(m.channels[0].lut.as_ref().unwrap().len(), 256);
}

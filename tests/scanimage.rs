//! # scanimage
//!
//! ScanImage TIFF reader tests: the CatalystNeuro test files (downloaded,
//! see tests/README.md), whose synthetic pixel values tell where each page
//! belongs, the reference readers and synthetic files.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use std::path::{Path, PathBuf};

use common::{read_tsv, root};
use scopeio::{FormatId, Plane, Region};

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("scanimage");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn gin(rel: &str) -> PathBuf {
    root().join("tests/reference/gin-ophys/imaging_datasets/ScanImage").join(rel)
}

/// Every pixel of a plane as i16.
fn plane(r: &dyn scopeio::ImageReader, series: usize, p: Plane) -> Vec<i16> {
    let m = r.series_meta(series).unwrap();
    let region = Region::new(0, 0, m.levels[0].size_x, m.levels[0].size_y);
    common::read(r, series, 0, p, region).unwrap().chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
}

/// Checks that every plane holds `1000 * unit + 10 * slice + channel
/// (+ 10000 * roi)`; `unit` is the volume (time point) counted from
/// `first_unit`, `channels` the ScanImage channel numbers.
fn check_values(path: &Path, channels: &[i16], first_unit: i16) {
    let r = scopeio::open(path).unwrap();
    assert_eq!(r.format().id, FormatId::ScanImage, "{}", path.display());
    let multi = r.series_count() > 1;
    for s in 0..r.series_count() {
        let m = r.series_meta(s).unwrap();
        assert_eq!(m.size_c as usize, channels.len(), "{}", path.display());
        for t in 0..m.size_t {
            for z in 0..m.size_z {
                for (c, &number) in channels.iter().enumerate() {
                    let want = 1000 * (first_unit + t as i16) + 10 * z as i16 + number + if multi { 10000 * s as i16 } else { 0 };
                    let got = plane(r.as_ref(), s, Plane::new(c as u32, z, t));
                    assert!(got.iter().all(|&v| v == want), "{} series {s} c{c} z{z} t{t}: want {want}, got {:?}", path.display(), &got[..4]);
                }
            }
        }
    }
}

/// File, ScanImage channel numbers, first unit, (c, z, t).
type Case = (&'static str, &'static [i16], i16, (u32, u32, u32));

#[test]
fn synthetic_values_land_on_their_planes() {
    let cases: &[Case] = &[
        // Fast z with 2 flyback frames per volume: 12 slices kept.
        ("tiff_format_version_3/volumetric/flyback_frames_two_channels_resonant_version_2016b/volume_00001.tif", &[1, 2], 0, (2, 12, 2)),
        ("tiff_format_version_3/volumetric/single_channel_resonant_version_2016b/volume_00001_00001.tif", &[1], 0, (1, 6, 2)),
        // Two ROIs stacked without fly-to lines (linear scanner).
        ("tiff_format_version_3/multi_roi/volumetric_two_rois_two_channels_linear_version_2016b/volume_00001_00001.tif", &[1, 2], 0, (2, 11, 1)),
        // The second file of a split recording: frames 2 and 3.
        ("tiff_format_version_3/planar/split_over_files_unidirectional_resonant_version_2017b/planar_00001_00002.tif", &[1], 2, (1, 1, 2)),
        ("tiff_format_version_4/planar/split_over_files_version_2019b/planar_00001_00001.tif", &[1], 0, (1, 1, 2)),
        // Three ROIs with 4 fly-to lines between them.
        (
            "tiff_format_version_4/multi_roi/planar_three_rois_split_over_files_no_frame_count_version_2020_1/planar_00001_00003.tif",
            &[2],
            4,
            (1, 1, 1),
        ),
        ("tiff_format_version_4/volumetric/arbitrary_stack_three_depths_two_channels_version_2019b/volume_00001.tif", &[1, 2], 0, (2, 3, 2)),
        ("tiff_format_version_4/photostimulation/on_demand_four_stimulus_groups_version_2020_0/photostim_00002.tif", &[1], 0, (1, 1, 10)),
    ];
    let mut checked = 0;
    for (rel, channels, first, (c, z, t)) in cases {
        let path = gin(rel);
        if !path.exists() {
            continue;
        }
        let r = scopeio::open(&path).unwrap();
        let m = r.series_meta(0).unwrap();
        assert_eq!((m.size_c, m.size_z, m.size_t), (*c, *z, *t), "{rel}");
        check_values(&path, channels, *first);
        checked += 1;
    }
    if checked == 0 {
        eprintln!("scanimage: CatalystNeuro test files not present, skipped");
    }
}

#[test]
fn layouts_of_the_test_files() {
    // (file, series, c, z, t, width, height) from the dataset README.
    let cases: &[(&str, usize, u32, u32, u32, u64, u64)] = &[
        ("legacy/multiple_frames_per_slice/frames_averaged_two_channels_version_3_6/stack_001.tif", 1, 2, 50, 1, 64, 64),
        ("legacy/planar/single_channel_version_4/planar.tif", 1, 1, 1, 20, 64, 64),
        ("tiff_format_version_1/planar/single_channel_resonant_version_5_1/planar.tif", 1, 1, 1, 10, 64, 64),
        ("tiff_format_version_2/multi_roi/planar_json_metadata_version_2015/planar_00001_00001.tif", 9, 1, 1, 2, 64, 464),
        ("tiff_format_version_3/planar/channels_one_and_three_as_column_vector_linear_version_5_3/planar_00001.tif", 1, 2, 1, 7, 64, 64),
        ("tiff_format_version_3/multiple_frames_per_slice/frames_saved_resonant_version_2017b/stack_00001.tif", 1, 1, 17, 6, 64, 64),
        ("tiff_format_version_3/multi_roi/planar_three_rois_of_different_heights_version_2018b/planar_00001.tif", 3, 1, 1, 3, 120, 502),
        ("tiff_format_version_3/multi_roi/volumetric_three_rois_first_file_of_split_version_2018a/stack_00002_00001.tif", 3, 1, 500, 1, 16, 16),
        ("tiff_format_version_4/volumetric/flyback_frames_two_channels_version_2021_0/volume_00001_00001.tif", 1, 2, 9, 3, 20, 20),
        (
            "tiff_format_version_4/multiple_frames_per_slice/frames_saved_galvo_galvo_bounded_stack_version_2021_0/stack_00003.tif",
            1,
            1,
            12,
            10,
            64,
            64,
        ),
        ("tiff_format_version_4/light_beads_microscopy/six_rois_twenty_six_channels_version_2023_1/planar_00001_00001.tif", 6, 26, 1, 2, 32, 32),
        ("files_with_errors/tiff_format_version_3/stack_stopped_early_version_5_6/stack_00004.tif", 1, 2, 7, 1, 64, 64),
        ("files_with_errors/tiff_format_version_4/bounded_stack_stopped_early_version_2020_1/stack_00002.tif", 1, 1, 3, 10, 64, 64),
    ];
    for (rel, series, c, z, t, w, h) in cases {
        let path = gin(rel);
        if !path.exists() {
            continue;
        }
        let r = scopeio::open(&path).unwrap();
        let m = r.series_meta(0).unwrap();
        assert_eq!((r.series_count(), m.size_c, m.size_z, m.size_t, m.levels[0].size_x, m.levels[0].size_y), (*series, *c, *z, *t, *w, *h), "{rel}");
    }
    // Acquisitions stopped before their first frame are rejected.
    let empty = gin("files_with_errors/tiff_format_version_3/no_frames_first_page_offset_at_end_of_file_version_5_6/file_00001.tif");
    if empty.exists() {
        assert!(scopeio::open(&empty).is_err());
    }
}

#[test]
fn metadata_of_the_test_files() {
    let path = gin("tiff_format_version_4/volumetric/arbitrary_stack_three_depths_two_channels_version_2019b/volume_00001.tif");
    if path.exists() {
        // Depths from the list, not from the step size fields.
        let r = scopeio::open(&path).unwrap();
        let zs: Vec<_> = (0..3).map(|z| r.plane_meta(0, Plane::new(0, z, 0)).unwrap().position.and_then(|p| p.z_nm)).collect();
        assert_eq!(zs, [Some(0.0), Some(30_000.0), Some(55_000.0)]);
        assert_eq!(r.series_meta(0).unwrap().physical_size.z_nm, None);
    }
    let path = gin("tiff_format_version_3/planar/channels_one_and_three_as_row_vector_linear_version_5_3/planar_00001.tif");
    if path.exists() {
        let r = scopeio::open(&path).unwrap();
        let m = r.series_meta(0).unwrap();
        let names: Vec<_> = m.channels.iter().map(|c| c.name.clone().unwrap()).collect();
        assert_eq!(names, ["Channel 1", "Channel 3"]);
        // Frame times from the page descriptions.
        let t1 = r.plane_meta(0, Plane::new(0, 0, 1)).unwrap().delta_t_s.unwrap();
        assert!((t1 - m.time_increment_s.unwrap()).abs() < 0.01, "{t1}");
    }
}

#[test]
fn synthetic_files_match_truth() {
    let dir = root().join("tests/data/scanimage");
    for t in read_tsv(&dir.join("truth.tsv")) {
        let path = dir.join(&t["file"]);
        let r = scopeio::open(&path).unwrap();
        assert_eq!(r.format().id, FormatId::ScanImage, "{}", t["file"]);
        let m = r.series_meta(0).unwrap();
        let dims = format!("{}x{}x{}x{}", r.series_count(), m.size_c, m.size_z, m.size_t);
        assert_eq!(dims, t["series_c_z_t"], "{}", t["file"]);
        let channels: Vec<i16> = t["channels"].split(',').map(|c| c.parse().unwrap()).collect();
        check_values(&path, &channels, 0);
        if t["px_nm"] != "-" {
            assert_eq!(m.physical_size.x_nm.map(|v| format!("{v:.1}")).as_deref(), Some(t["px_nm"].as_str()), "{}", t["file"]);
        }
    }
}

#[test]
fn corrupt_files_do_not_panic() {
    let dir = root().join("tests/data/scanimage");
    for t in read_tsv(&dir.join("truth.tsv")) {
        common::corrupt_sweep(&dir.join(&t["file"]), 7);
    }
}

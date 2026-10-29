//! # micromanager
//!
//! Micro-Manager reader tests against the reference readers and synthetic
//! datasets.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("micromanager");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/micromanager").join(name)
}

#[test]
fn synthetic_samples_match_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::MicroManager, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn every_file_of_a_dataset_opens_it() {
    for f in ["stack/syn_MMStack_Pos0_1.ome.tif", "stack/syn_MMStack_Pos1.ome.tif", "nd/NDTiff.index", "nd/nd_NDTiffStack.tif", "sep/Pos1/img_000000002_FITC_000.tif"] {
        let r = scopeio::open(&sample(f)).unwrap();
        assert_eq!((r.format().id, r.series_count()), (FormatId::MicroManager, 2), "{f}");
    }
}

#[test]
fn stack_metadata() {
    let r = scopeio::open(&sample("stack/syn_MMStack_Pos0.ome.tif")).unwrap();
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type, m.significant_bits), (2, 2, 2, PixelType::U16, Some(12)));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm, m.time_increment_s), (Some(250.0), Some(1500.0), Some(0.5)));
    assert_eq!(m.name.as_deref(), Some("Right"));
    assert_eq!(m.acquisition_date.as_deref(), Some("2026-10-29T10:11:12.345+01:00"));
    assert_eq!(m.channels[1].name.as_deref(), Some("GFP"));
    assert_eq!(m.channels[0].color, Some([0, 0, 255, 255]));
    assert_eq!(m.stage_position.and_then(|p| p.x_nm), Some(101_000.0));
    // Image metadata of the big-endian file and of the continuation file.
    let p = r.plane_meta(1, Plane::new(1, 1, 0)).unwrap();
    assert_eq!((p.delta_t_s, p.exposure_s), (Some(0.01), Some(0.021)));
    assert_eq!(p.position.and_then(|p| p.z_nm), Some(4500.0));
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 1)).unwrap().delta_t_s, Some(0.5));
    // The missing image has no metadata.
    assert_eq!(r.plane_meta(1, Plane::new(1, 1, 1)).unwrap(), scopeio::PlaneMeta::default());
}

#[test]
fn separate_files_metadata() {
    let r = scopeio::open(&sample("sep/Pos0/metadata.txt")).unwrap();
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_t, m.pixel_type, m.name.as_deref()), (2, 3, PixelType::U8, Some("Pos1")));
    assert_eq!((m.physical_size.x_nm, m.time_increment_s), (Some(500.0), Some(2.0)));
    assert_eq!(r.plane_meta(1, Plane::new(1, 0, 2)).unwrap().delta_t_s, Some(4.001));
    let r = scopeio::open(&sample("sep2/Pos0/metadata.txt")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_z, m.physical_size.x_nm, m.channels[0].name.as_deref()), (2, Some(100.0), Some("Cy5")));
    assert_eq!(r.plane_meta(0, Plane::new(0, 1, 0)).unwrap().exposure_s, Some(0.0075));
}

#[test]
fn ndtiff_metadata() {
    let r = scopeio::open(&sample("nd")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.significant_bits), (2, 2, 1, Some(12)));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(650.0), Some(2000.0)));
    assert_eq!(m.channels.iter().map(|c| c.name.as_deref()).collect::<Vec<_>>(), [Some("DAPI"), Some("GFP")]);
    assert_eq!(r.series_meta(1).unwrap().name.as_deref(), Some("position 1"));
    assert_eq!(r.plane_meta(1, Plane::new(0, 0, 0)).unwrap().delta_t_s, Some(0.1));
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&sample("stack/syn_MMStack_Pos1.ome.tif"), 13);
    common::corrupt_sweep(&sample("sep2/Pos0/metadata.txt"), 7);
    common::corrupt_sweep(&sample("nd/NDTiff.index"), 3);
}

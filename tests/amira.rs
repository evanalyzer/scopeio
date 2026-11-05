//! # amira
//!
//! AmiraMesh reader tests against the reference readers and synthetic
//! files.
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
    let failures = common::check_against_oracles("amira");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/amira").join(name)
}

fn meta(name: &str) -> scopeio::SeriesMeta {
    scopeio::open(&sample(name)).unwrap().series_meta(0).unwrap().clone()
}

#[test]
fn synthetic_files_match_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::AmiraMesh, "{}", t["file"]);
        assert_eq!(crc(&decode_level(r.as_ref(), 0, 0).unwrap().pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    let m = meta("le_ushort.am");
    assert_eq!((m.size_c, m.size_z, m.levels[0].size_x, m.levels[0].size_y, m.pixel_type), (1, 3, 7, 5, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.y_nm, m.physical_size.z_nm), (Some(500.0), Some(500.0), Some(2000.0)));
    // 2D lattice in nm, big endian.
    let m = meta("be_short.am");
    assert_eq!((m.size_z, m.pixel_type, m.physical_size.x_nm, m.physical_size.z_nm), (1, PixelType::I16, Some(2.0), None));
    // Vector components and label fields are channels.
    let m = meta("vector3.am");
    assert_eq!((m.size_c, m.pixel_type), (3, PixelType::F32));
    assert_eq!(m.channels[2].name.as_deref(), Some("Vectors[2]"));
    let m = meta("labels.am");
    assert_eq!((m.size_c, m.channels[0].name.as_deref(), m.channels[1].name.as_deref()), (2, Some("Labels"), Some("Probability")));
    // Stacked: x / y from BoundingBoxXY, no z spacing.
    let m = meta("stacked_double.am");
    assert_eq!((m.pixel_type, m.physical_size.x_nm, m.physical_size.z_nm), (PixelType::F64, Some(250.0), None));
    // Only the top-level BoundingBox counts.
    assert_eq!(meta("zip_nosize.am").physical_size.x_nm, Some(500.0));
    assert_eq!(meta("zip_int.am").physical_size.x_nm, None);
}

#[test]
fn units_and_parameters_in_original_metadata() {
    let r = scopeio::open(&sample("be_short.am")).unwrap();
    let pairs = r.original_metadata(0).unwrap().pairs;
    let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    assert_eq!(get("Parameters.Units.Coordinates"), Some("nm"));
    assert_eq!(get("define.Lattice"), Some("7 5"));
    assert_eq!(get("Lattice.Data"), Some("short @1"));
}

#[test]
fn corrupt_files_do_not_panic() {
    for (name, step) in [
        ("le_ushort.am", 3),
        ("be_short.am", 3),
        ("ascii_float.am", 5),
        ("zip_int.am", 2),
        ("rle_byte.am", 1),
        ("zip_nosize.am", 3),
        ("labels.am", 2),
        ("stacked_double.am", 7),
    ] {
        common::corrupt_sweep(&sample(name), step);
    }
}

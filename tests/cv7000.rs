//! # cv7000
//!
//! Yokogawa CellVoyager CV7000 / CV8000 reader tests against the
//! reference readers and synthetic measurements.
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
    let failures = common::check_against_oracles("cv7000");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/cv7000").join(name)
}

#[test]
fn synthetic_measurement_matches_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::Cv7000, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    // D01 has no image file: four series, the last one (an AF error) blank.
    for entry in ["plate/synth.wpi", "plate/MeasurementData.mlf", "plate/MeasurementDetail.mrf"] {
        let r = scopeio::open(&sample(entry)).unwrap();
        assert_eq!(r.format().id, FormatId::Cv7000, "{entry}");
        assert_eq!(r.series_count(), 4, "{entry}");
    }
    let r = scopeio::open(&sample("plate/synth.wpi")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.name.as_deref(), m.size_c, m.size_z, m.size_t, m.pixel_type), (Some("well B03 field 1"), 3, 3, 2, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(650.0), Some(2000.0)));
    assert_eq!(m.acquisition_date.as_deref(), Some("2026-11-04T10:11:12"));
    let names: Vec<_> = m.channels.iter().map(|c| c.name.as_deref().unwrap_or_default()).collect();
    assert_eq!(names, ["DNA", "ER", "BF"]);
    let c = &m.channels[0];
    assert_eq!(
        (c.excitation_wavelength_nm, c.emission_wavelength_nm, c.exposure_s, c.binning, c.color),
        (Some(405.0), Some(445.0), Some(0.1), Some((2, 2)), Some([0, 0x2f, 0xff, 0xff]))
    );
    assert_eq!(m.channels[2].excitation_wavelength_nm, None);
    let p = r.plane_meta(0, Plane::new(2, 1, 1)).unwrap();
    assert_eq!(p.position.and_then(|p| p.z_nm), Some(-1000.0));
    assert!((p.delta_t_s.unwrap() - 62.25).abs() < 1e-6);
    let plate = &r.plates()[0];
    assert_eq!((plate.name.as_deref(), plate.rows, plate.cols, plate.wells.len()), (Some("synth"), 8, 12, 4));
    assert_eq!((plate.wells[3].row, plate.wells[3].col, plate.wells[3].field), (2, 4, 1));
}

#[test]
fn corrupt_files_do_not_panic() {
    // Each index file corrupted next to the intact rest of the measurement.
    let src = sample("plate");
    let dir = std::env::temp_dir().join(format!("scopeio-cv7000-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(&src).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
    }
    for (name, step) in [("MeasurementData.mlf", 13), ("MeasurementDetail.mrf", 7), ("synth.mes", 7), ("synth.wpi", 5)] {
        let data = std::fs::read(src.join(name)).unwrap();
        for at in (0..data.len()).step_by(step) {
            let mut d = data.clone();
            if at % 2 == 0 {
                d.truncate(at);
            } else {
                d[at] ^= 0xA5;
            }
            std::fs::write(dir.join(name), &d).unwrap();
            if let Ok(r) = scopeio::open(&dir.join("synth.wpi")) {
                for s in 0..r.series_count() {
                    let _ = decode_level(r.as_ref(), s, 0);
                }
            }
        }
        std::fs::write(dir.join(name), &data).unwrap();
    }
    std::fs::remove_dir_all(&dir).unwrap();
    common::corrupt_sweep(&sample("plate/MeasurementData.mlf"), 13);
}

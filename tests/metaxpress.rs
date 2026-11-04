//! # metaxpress
//!
//! MetaXpress reader tests against the
//! reference readers and synthetic plates.
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
    let failures = common::check_against_oracles("metaxpress");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}


use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/metaxpress").join(name)
}

#[test]
fn synthetic_plate_matches_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        assert_eq!(r.format().id, FormatId::MetaXpress, "{}", t["file"]);
        let s: usize = t["series"].parse().unwrap();
        assert_eq!(crc(&decode_level(r.as_ref(), s, 0).unwrap().pixels), t["crc32"], "{} series {s}", t["file"]);
    }
}

#[test]
fn synthetic_metadata() {
    // The copy of the HTD in a time point folder opens the same plate.
    for entry in ["plate/synth.HTD", "plate/TimePoint_2/synth.HTD"] {
        let r = scopeio::open(&sample(entry)).unwrap();
        // B02 and B03 with three sites each; C04 is not selected.
        assert_eq!(r.series_count(), 6, "{entry}");
        let m = r.series_meta(4).unwrap();
        assert_eq!((m.name.as_deref(), m.size_c, m.size_z, m.size_t, m.pixel_type), (Some("well B03 field 2"), 2, 2, 2, PixelType::U16), "{entry}");
        assert_eq!((m.physical_size.x_nm, m.physical_size.y_nm), (Some(500.0), Some(500.0)), "{entry}");
        let names: Vec<_> = m.channels.iter().map(|c| c.name.as_deref().unwrap_or_default()).collect();
        assert_eq!(names, ["DAPI", "FITC"]);
        let plate = &r.plates()[0];
        assert_eq!((plate.name.as_deref(), plate.rows, plate.cols, plate.wells.len()), (Some("synthetic plate"), 8, 12, 6));
        assert_eq!((plate.wells[4].row, plate.wells[4].col, plate.wells[4].field), (1, 2, 1));
    }
    let r = scopeio::open(&sample("plate/synth.HTD")).unwrap();
    assert!(r.plane_meta(0, Plane::new(1, 1, 1)).is_ok());
}

#[test]
fn corrupt_files_do_not_panic() {
    // The HTD corrupted next to the intact image folders.
    let src = sample("plate/synth.HTD");
    let data = std::fs::read(&src).unwrap();
    let dir = sample("plate");
    let target = dir.join(format!("sweep-{}.HTD", std::process::id()));
    for at in (0..data.len()).step_by(7) {
        let mut d = data.clone();
        if at % 2 == 0 {
            d.truncate(at);
        } else {
            d[at] ^= 0xA5;
        }
        std::fs::write(&target, &d).unwrap();
        if let Ok(r) = scopeio::open(&target) {
            for s in 0..r.series_count().min(8) {
                let _ = decode_level(r.as_ref(), s, 0);
            }
        }
    }
    std::fs::remove_file(&target).unwrap();
    common::corrupt_sweep(&sample("plate/TimePoint_1/ZStep_1/synth_B02_s1_w1.TIF"), 7);
}

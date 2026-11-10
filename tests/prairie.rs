//! # prairie
//!
//! Bruker Prairie View reader tests against the reference readers and
//! synthetic recordings.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("prairie");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{read_tsv, root};
use scopeio::{FormatId, Plane, Region};

/// Every pixel of a plane as u16.
fn plane(r: &dyn scopeio::ImageReader, p: Plane) -> Vec<u16> {
    let m = r.series_meta(0).unwrap();
    let region = Region::new(0, 0, m.levels[0].size_x, m.levels[0].size_y);
    common::read(r, 0, 0, p, region).unwrap().chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect()
}

#[test]
fn synthetic_recordings_match_truth() {
    let dir = root().join("tests/data/prairie");
    for t in read_tsv(&dir.join("truth.tsv")) {
        let name = &t["folder"];
        let r = scopeio::open(&dir.join(name).join(format!("{name}.xml"))).unwrap();
        assert_eq!(r.format().id, FormatId::Prairie, "{name}");
        let m = r.series_meta(0).unwrap();
        assert_eq!(format!("{}x{}x{}", m.size_c, m.size_z, m.size_t), t["c_z_t"], "{name}");
        let channels: Vec<u16> = t["channels"].split(',').map(|c| c.parse().unwrap()).collect();
        let missing: Option<u32> = t["missing_t"].parse().ok();
        for tt in 0..m.size_t {
            for z in 0..m.size_z {
                for (c, &number) in channels.iter().enumerate() {
                    let want = if Some(tt) == missing { 0 } else { 1000 * tt as u16 + 10 * z as u16 + number };
                    let got = plane(r.as_ref(), Plane::new(c as u32, z, tt));
                    assert!(got.iter().all(|&v| v == want), "{name} c{c} z{z} t{tt}: want {want}, got {}", got[0]);
                }
            }
        }
        assert_eq!((m.physical_size.x_nm, m.physical_size.y_nm), (Some(500.0), Some(500.0)), "{name}");
        let objective = m.objective.as_ref().unwrap();
        assert_eq!((objective.magnification, objective.numerical_aperture), (Some(16.0), Some(0.8)));
        assert_eq!(m.acquisition_date.as_deref(), Some("2026-02-02T15:04:05"));
    }
}

#[test]
fn synthetic_plane_metadata() {
    let dir = root().join("tests/data/prairie");
    // Bidirectional z series: positions in depth order in both cycles.
    let r = scopeio::open(&dir.join("zseries/zseries.xml")).unwrap();
    for t in 0..2 {
        let zs: Vec<_> = (0..3).map(|z| r.plane_meta(0, Plane::new(0, z, t)).unwrap().position.and_then(|p| p.z_nm)).collect();
        assert_eq!(zs, [Some(10_000.0), Some(12_000.0), Some(14_000.0)], "cycle {t}");
    }
    assert_eq!(r.series_meta(0).unwrap().physical_size.z_nm, Some(2000.0));
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.channels[0].name.as_deref(), m.channels[0].excitation_wavelength_nm), (Some("Ch1"), Some(920.0)));
    // Timed: times since the first frame, frame period as increment.
    let r = scopeio::open(&dir.join("multiseq/multiseq.xml")).unwrap();
    let times: Vec<_> = (0..4).map(|t| r.plane_meta(0, Plane::new(0, 0, t)).unwrap().delta_t_s).collect();
    assert_eq!(times, [Some(0.0), Some(0.25), Some(10.0), Some(10.25)]);
    assert_eq!(r.series_meta(0).unwrap().time_increment_s, Some(0.25));
}

#[test]
fn corrupt_files_do_not_panic() {
    let dir = root().join("tests/data/prairie");
    common::corrupt_tree_sweep(&dir.join("timed"), "timed.xml", "timed.xml", 37);
    common::corrupt_tree_sweep(&dir.join("timed"), "timed.xml", "timed_Cycle00001_Ch2_000001.ome.tif", 5);
    common::corrupt_tree_sweep(&dir.join("zseries"), "zseries.xml", "zseries.xml", 53);
}

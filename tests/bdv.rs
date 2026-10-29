//! # bdv
//!
//! BigDataViewer XML/HDF5 reader tests against the reference readers and a
//! synthetic dataset.
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
    let failures = common::check_against_oracles("bdv");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{FormatId, PixelType};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/bdv").join(name)
}

#[test]
fn synthetic_dataset_matches_spec_truth() {
    let r = scopeio::open(&sample("synthetic.xml")).unwrap();
    assert_eq!(r.format().id, FormatId::Bdv);
    for t in read_tsv(&sample("truth.tsv")) {
        let (s, l): (usize, usize) = (t["series"].parse().unwrap(), t["level"].parse().unwrap());
        assert_eq!(crc(&decode_level(r.as_ref(), s, l).unwrap().pixels), t["crc32"], "series {s} level {l}");
    }
}

#[test]
fn synthetic_metadata() {
    // The HDF5 file opens its XML.
    let r = scopeio::open(&sample("synthetic.h5")).unwrap();
    assert_eq!((r.format().id, r.series_count()), (FormatId::Bdv, 2));
    let m = r.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.size_t, m.pixel_type), (2, 4, 2, PixelType::U16));
    assert_eq!((m.physical_size.x_nm, m.physical_size.z_nm), (Some(500.0), Some(2000.0)));
    assert_eq!(m.name.as_deref(), Some("angle 90, illumination 0"));
    assert_eq!(m.channels.iter().map(|c| c.name.as_deref()).collect::<Vec<_>>(), [Some("DAPI"), Some("GFP")]);
    assert_eq!(m.levels.len(), 2);
    assert_eq!((m.levels[1].size_x, m.levels[1].size_y, m.levels[1].downsample), (5, 4, 2.0));
    assert_eq!(m.levels[0].tile, Some((4, 4)));
}

#[test]
fn corrupt_files_do_not_panic() {
    // Corrupt HDF5 files next to the intact XML.
    let dir = std::env::temp_dir().join(format!("scopeio-bdv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(sample("synthetic.xml"), dir.join("synthetic.xml")).unwrap();
    let data = std::fs::read(sample("synthetic.h5")).unwrap();
    for at in (0..data.len()).step_by(37) {
        let mut d = data.clone();
        if at % 2 == 0 {
            d.truncate(at.max(8));
        } else {
            d[at] ^= 0xA5;
        }
        std::fs::write(dir.join("synthetic.h5"), &d).unwrap();
        if let Ok(r) = scopeio::open(&dir.join("synthetic.xml")) {
            // Corrupt sizes can claim huge volumes: decode only small ones.
            let small = (0..r.series_count()).all(|s| {
                r.series_meta(s).is_ok_and(|m| (m.levels[0].size_x as u128) * (m.levels[0].size_y as u128) * (m.size_c * m.size_z * m.size_t) as u128 * 8 < 64 << 20)
            });
            if small {
                let _ = common::decode(r.as_ref());
            }
        }
    }
    std::fs::remove_dir_all(&dir).ok();
    // Corrupt XML: the HDF5 file is missing next to the copy, so only parsing runs.
    common::corrupt_sweep(&sample("synthetic.xml"), 7);
}

//! # vms
//!
//! Hamamatsu VMS reader tests against the reference readers and a
//! synthetic slide.
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
    let failures = common::check_against_oracles("vms");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{AssociatedKind, FormatId, OutputSpec};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/vms").join(name)
}

#[test]
fn synthetic_slide_matches_truth() {
    let r = scopeio::open(&sample("slide.vms")).unwrap();
    assert_eq!(r.format().id, FormatId::Vms);
    let truth: std::collections::HashMap<String, String> = read_tsv(&sample("truth.tsv")).into_iter().map(|t| (t["image"].clone(), t["crc32"].clone())).collect();
    for level in 0..2 {
        assert_eq!(crc(&decode_level(r.as_ref(), 0, level).unwrap().pixels), truth[&format!("level{level}")], "level {level}");
    }
    let m = r.read_associated(&AssociatedKind::Macro, &OutputSpec::default()).unwrap();
    assert_eq!(crc(m.data.as_bytes()), truth["macro"]);
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("slide.vms")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_z, m.samples_per_pixel, m.physical_size.x_nm), (2, 3, Some(500.0)));
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.levels[0].tile), (48, 24, Some((16, 8))));
    assert_eq!((m.levels[1].size_x, m.levels[1].downsample), (24, 2.0));
    assert_eq!(m.objective.as_ref().and_then(|o| o.magnification), Some(20.0));
    assert_eq!(r.associated_images(), [AssociatedKind::Macro]);
}

#[test]
fn corrupt_files_do_not_panic() {
    // JPEG files next to the intact index.
    let dir = std::env::temp_dir().join(format!("scopeio-vms-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(sample("")).unwrap().flatten() {
        std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
    }
    let data = std::fs::read(sample("slide(0,0,0).jpg")).unwrap();
    for at in (0..data.len()).step_by(7) {
        let mut d = data.clone();
        if at % 2 == 0 {
            d.truncate(at.max(2));
        } else {
            d[at] ^= 0xA5;
        }
        std::fs::write(dir.join("slide(0,0,0).jpg"), &d).unwrap();
        if let Ok(r) = scopeio::open(&dir.join("slide.vms")) {
            let _ = common::decode(r.as_ref());
        }
    }
    std::fs::remove_dir_all(&dir).ok();
    common::corrupt_sweep(&sample("slide.vms"), 3);
}

//! # philips
//!
//! Philips TIFF reader tests against the reference readers and a
//! synthetic file.
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
    let failures = common::check_against_oracles("philips");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode_level, read_tsv, root};
use scopeio::{AssociatedKind, FormatId, Plane, Region};

fn synthetic() -> std::path::PathBuf {
    root().join("tests/data/philips/synthetic.tiff")
}

#[test]
fn synthetic_file_matches_truth() {
    let r = scopeio::open(&synthetic()).unwrap();
    assert_eq!(r.format().id, FormatId::PhilipsTiff);
    let m = r.series_meta(0).unwrap();
    // Padded TIFF sizes 32 x 32 and 16 x 16 become 32 x 24 and 16 x 12.
    let sizes: Vec<_> = m.levels.iter().map(|l| (l.size_x, l.size_y, l.downsample)).collect();
    assert_eq!(sizes, [(64, 48, 1.0), (32, 24, 2.0), (16, 12, 4.0)]);
    // Column spacing 0.00025 mm, row spacing 0.0005 mm.
    assert_eq!((m.physical_size.x_nm, m.physical_size.y_nm), (Some(250.0), Some(500.0)));
    assert_eq!((m.name.as_deref(), m.acquisition_date.as_deref()), (Some("SYN-42"), Some("2024-02-29T13:14:15")));
    assert_eq!(r.associated_images(), [AssociatedKind::Label, AssociatedKind::Macro]);
    for t in read_tsv(&root().join("tests/data/philips/truth.tsv")) {
        let got = match t["image"].split_once('/') {
            Some((s, l)) => decode_level(r.as_ref(), s.parse().unwrap(), l.parse().unwrap()).unwrap().pixels,
            None => r.read_associated(&AssociatedKind::Macro, &Default::default()).unwrap().as_bytes().to_vec(),
        };
        assert_eq!(crc(&got), t["crc32"], "{}", t["image"]);
    }
    // The XML label (a solid JPEG) wins over the TIFF "Label" directory.
    let label = r.read_associated(&AssociatedKind::Label, &Default::default()).unwrap();
    assert_eq!((label.width, label.height, label.channels), (24, 12, 3));
    for px in label.as_bytes().chunks(3) {
        assert!(px[0].abs_diff(200) <= 2 && px[1].abs_diff(100) <= 2 && px[2].abs_diff(50) <= 2, "{px:?}");
    }
    let original = r.original_metadata(0).unwrap();
    assert!(original.pairs.contains(&("philips.barcode".into(), "SYN-42".into())));
    assert!(original.pairs.contains(&("philips.DICOM_MANUFACTURER".into(), "Synthetic".into())));
}

#[test]
fn regions_equal_crops_of_the_level() {
    // Crops across tile borders, the missing tile and the padded edge.
    let r = scopeio::open(&synthetic()).unwrap();
    for (level, w, h) in [(0usize, 64u64, 48u64), (1, 32, 24)] {
        let full = common::read(r.as_ref(), 0, level, Plane::default(), Region::new(0, 0, w, h)).unwrap();
        for (x, y, cw, ch) in [(0, 0, 5, 3), (10, 12, 20, 9), (w - 7, h - 5, 7, 5), (15, 15, 2, 2)] {
            let crop = common::read(r.as_ref(), 0, level, Plane::default(), Region::new(x, y, cw, ch)).unwrap();
            let want: Vec<u8> = (y..y + ch).flat_map(|row| full[((row * w + x) * 3) as usize..((row * w + x + cw) * 3) as usize].to_vec()).collect();
            assert_eq!(crop, want, "level {level} region {x},{y} {cw}x{ch}");
        }
    }
    // Reads may not reach into the padding.
    assert!(common::read(r.as_ref(), 0, 1, Plane::default(), Region::new(0, 20, 32, 5)).is_err());
}

#[test]
fn corrupt_files_do_not_panic() {
    common::corrupt_sweep(&synthetic(), 13);
}

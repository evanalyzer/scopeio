//! # czi
//!
//! Zeiss CZI reader tests against the reference readers.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

#[test]
fn matches_reference_readers() {
    let failures = common::check_against_oracles("czi");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode, decode_level, read_tsv, root};
use scopeio::{AssociatedKind, Plane};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/czi").join(name)
}

#[test]
fn synthetic_files_match_spec_truth() {
    let mut failures = Vec::new();
    for t in read_tsv(&sample("truth.tsv")) {
        let series: usize = t["series"].parse().unwrap();
        let level: usize = t["level"].parse().unwrap();
        let got = scopeio::open(&sample(&t["file"])).and_then(|r| {
            if level == 0 { decode(r.as_ref()).map(|d| d[series].pixels.clone()) } else { decode_level(r.as_ref(), series, level).map(|s| s.pixels) }
        });
        match got {
            Ok(p) if crc(&p) == t["crc32"] => {}
            Ok(p) => failures.push(format!("{} series {series} level {level}: {} != {}", t["file"], crc(&p), t["crc32"])),
            Err(e) => failures.push(format!("{} series {series} level {level}: {e}", t["file"])),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn synthetic_metadata() {
    let r = scopeio::open(&sample("mosaic_bgr24.czi")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Czi);
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.samples_per_pixel, m.levels.len()), (3, 2));
    assert_eq!((m.levels[0].size_x, m.levels[0].size_y, m.levels[1].size_x, m.levels[1].downsample), (22, 10, 11, 2.0));
    assert_eq!(m.physical_size.x_nm, Some(500.0));
    assert!(r.associated_images().is_empty());

    let s = scopeio::open(&sample("scenes_zstd.czi")).unwrap();
    assert_eq!(s.series_count(), 3);
    let names: Vec<_> = (0..3).map(|i| s.series_meta(i).unwrap().name.clone().unwrap_or_default()).collect();
    assert_eq!(names, ["A1 H=0", "B2 H=0", "B2 H=1"]);
    let m = s.series_meta(1).unwrap();
    assert_eq!((m.size_c, m.size_z, m.significant_bits), (2, 2, Some(12)));
    assert_eq!(m.channels[1].name.as_deref(), Some("FITC"));
    assert_eq!(m.channels[1].color, Some([0, 255, 0, 255]));

    let c = scopeio::open(&sample("chunked_time.czi")).unwrap();
    let m = c.series_meta(0).unwrap();
    assert_eq!((m.size_t, m.time_increment_s), (3, Some(2.5)));
    assert_eq!(c.plane_meta(0, Plane::new(0, 0, 2)).unwrap().delta_t_s, Some(5.0));
    assert!(c.read_associated(&AssociatedKind::Label, &Default::default()).is_err());
}

#[test]
fn multi_file_document() {
    let r = scopeio::open(&sample("multipart.czi")).unwrap();
    let m = r.series_meta(0).unwrap();
    assert_eq!((m.size_t, m.pixel_type), (3, scopeio::PixelType::U8));
    // TimeStamps read from part 1.
    assert_eq!(r.plane_meta(0, Plane::new(0, 0, 2)).unwrap().delta_t_s, Some(3.0));
    // An attachment naming an absent part is read from the master.
    let original = r.original_metadata(0).unwrap();
    assert!(original.pairs.iter().any(|(k, v)| k == "Attachment Profile" && v == "CZPRF"));
    assert!(matches!(scopeio::open(&sample("multipart (1).czi")), Err(scopeio::Error::UnsupportedVariant(_))));

    // Without part 2 the other planes stay readable.
    let dir = std::env::temp_dir().join(format!("scopeio-czi-parts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["multipart.czi", "multipart (1).czi"] {
        std::fs::copy(sample(name), dir.join(name)).unwrap();
    }
    let r = scopeio::open(&dir.join("multipart.czi")).unwrap();
    let mut req = scopeio::RegionRequest::new(0, 0, scopeio::Region::new(0, 0, 6, 5));
    req.t = 1;
    assert!(r.read_region(&req).is_ok());
    req.t = 2;
    assert!(matches!(r.read_region(&req), Err(scopeio::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn corrupt_files_do_not_panic() {
    let data = std::fs::read(sample("scenes_zstd.czi")).unwrap();
    let dir = std::env::temp_dir().join(format!("scopeio-czi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cut.czi");
    for cut in (0..data.len()).step_by(97) {
        let mut d = data.clone();
        // Truncate or flip bytes in place.
        if cut % 2 == 0 {
            d.truncate(cut.max(16));
        } else {
            d[cut] ^= 0xA5;
        }
        std::fs::write(&path, &d).unwrap();
        if let Ok(r) = scopeio::open(&path) {
            common::decode_small(r.as_ref());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

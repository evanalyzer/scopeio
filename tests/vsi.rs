//! # vsi
//!
//! Olympus VSI reader tests against the reference readers.
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
    let failures = common::check_against_oracles("vsi");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

use common::{crc, decode, read_tsv, root};
use scopeio::{AssociatedKind, OutputSpec, Region, RegionRequest};

fn sample(name: &str) -> std::path::PathBuf {
    root().join("tests/data/vsi").join(name)
}

#[test]
fn synthetic_dataset_matches_spec_truth() {
    for t in read_tsv(&sample("truth.tsv")) {
        let r = scopeio::open(&sample(&t["file"])).unwrap();
        let s = &decode(r.as_ref()).unwrap()[t["series"].parse::<usize>().unwrap()];
        assert_eq!(s.channels.to_string(), t["channels"]);
        assert_eq!(crc(&s.pixels), t["crc32"], "{}", t["file"]);
    }
}

#[test]
fn synthetic_metadata_levels_and_thumbnail() {
    let r = scopeio::open(&sample("synthetic.vsi")).unwrap();
    assert_eq!(r.format().id, scopeio::FormatId::Vsi);
    let m = r.series_meta(0).unwrap();
    assert_eq!(m.name.as_deref(), Some("Synthetic 2C"));
    assert_eq!(m.physical_size.x_nm, Some(250.0));
    assert_eq!(m.acquisition_date.as_deref(), Some("2023-11-14T22:13:20Z"));
    let names: Vec<_> = m.channels.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("DAPI"), Some("FITC")]);
    assert_eq!((m.levels[1].size_x, m.levels[1].size_y, m.levels[1].downsample), (75, 50, 2.0));
    // Level 1 holds (x + y + c) % 7; its grid origin is level 0's halved.
    let req = RegionRequest::new(0, 1, Region::new(0, 0, 4, 1));
    let px: Vec<u16> = r.read_region(&req).unwrap().into_vec().unwrap();
    assert_eq!(px, [0, 1, 2, 3]);
    assert_eq!(r.associated_images(), [AssociatedKind::Thumbnail]);
    let thumb = r.read_associated(&AssociatedKind::Thumbnail, &OutputSpec::default()).unwrap();
    assert_eq!((thumb.width, thumb.height, thumb.channels), (2, 2, 3));
    assert_eq!(&thumb.as_bytes()[..3], &[255, 0, 0]);
}

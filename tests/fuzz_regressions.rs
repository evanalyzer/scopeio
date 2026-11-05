//! # fuzz_regressions
//!
//! Inputs found by the fuzz targets (`fuzz/`) that panicked, aborted or
//! allocated without bound before their fixes. Each is opened under the
//! file name its target uses and read like the fuzz harness does; errors
//! are expected, panics and aborts fail.
//!
//! The inputs are mutations of the committed samples in `tests/data`.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use std::path::{Component, Path};

use scopeio::{ChannelSelect, Plane, Region, RegionRequest};

/// File name each fuzz target writes its input to (see `fuzz/fuzz_targets`).
const TARGETS: &[(&str, &str)] = &[
    ("tiff", "f.tif"),
    ("ome_tiff", "f.ome.tif"),
    ("ndpi", "f.ndpi"),
    ("nd2", "f.nd2"),
    ("czi", "f.czi"),
    ("ims", "f.ims"),
    ("dicom", "f.dcm"),
    ("avi", "f.avi"),
];

/// Reads metadata and 256 x 256 corners of every level and plane corner,
/// as `scopeio_fuzz::exercise` does.
fn exercise(path: &Path) {
    let Ok(r) = scopeio::open(path) else { return };
    for s in 0..r.series_count().min(8) {
        let Ok(m) = r.series_meta(s) else { continue };
        let m = m.clone();
        let _ = r.original_metadata(s);
        let corners = |n: u32| if n > 1 { vec![0, n - 1] } else { vec![0] };
        for c in corners(m.size_c) {
            for z in corners(m.size_z) {
                for t in corners(m.size_t) {
                    let _ = r.plane_meta(s, Plane::new(c, z, t));
                    for (level, lv) in m.levels.iter().enumerate().take(8) {
                        for (x, y) in [(0, 0), (lv.size_x.saturating_sub(256), lv.size_y.saturating_sub(256))] {
                            let mut req = RegionRequest::new(s, level, Region::new(x, y, lv.size_x.min(256), lv.size_y.min(256)));
                            (req.z, req.t, req.channels) = (z, t, ChannelSelect::Channel(c));
                            let _ = r.read_region(&req);
                        }
                    }
                }
            }
        }
    }
    for kind in r.associated_images().iter().take(8) {
        let _ = r.read_associated(kind, &Default::default());
    }
}

fn inputs(target: &str) -> Vec<std::path::PathBuf> {
    let dir = common::root().join("tests/data/fuzz").join(target);
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    files
}

#[test]
fn single_file_inputs_do_not_panic() {
    let dir = std::env::temp_dir().join(format!("scopeio-fuzz-regressions-{}", std::process::id()));
    for (target, name) in TARGETS {
        for input in inputs(target) {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(&input, dir.join(name)).unwrap();
            exercise(&dir.join(name));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Unpacks a `zarr` target input (`path, 0x00, u32 LE length, bytes`
/// entries, see `scopeio_fuzz::tree`) below `dir`.
fn unpack(mut data: &[u8], dir: &Path) {
    while let Some(nul) = data.iter().position(|&b| b == 0) {
        let (name, tail) = (&data[..nul], &data[nul + 1..]);
        let Some(len) = tail.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) else { break };
        let body = &tail[4..][..len.min(tail.len() - 4)];
        data = &tail[4 + body.len()..];
        let Ok(name) = std::str::from_utf8(name) else { continue };
        let rel = Path::new(name);
        if name.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            continue;
        }
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
}

#[test]
fn zarr_inputs_do_not_panic() {
    let dir = std::env::temp_dir().join(format!("scopeio-fuzz-regressions-zarr-{}", std::process::id()));
    for input in inputs("zarr") {
        let _ = std::fs::remove_dir_all(&dir);
        let store = dir.join("f.zarr");
        std::fs::create_dir_all(&store).unwrap();
        unpack(&std::fs::read(&input).unwrap(), &store);
        exercise(&store);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

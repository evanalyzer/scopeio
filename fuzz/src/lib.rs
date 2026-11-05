//! # scopeio-fuzz
//!
//! Shared harness of the fuzz targets: the input is written as a file (or a
//! folder tree), opened with `scopeio::open` and exercised through the
//! public API: series and plane metadata, original metadata, small regions
//! of every level and plane corner, associated images. Errors are expected;
//! panics, aborts, hangs and runaway allocations are findings.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::{Component, Path, PathBuf};

use scopeio::{ChannelSelect, ImageReader, Region, RegionRequest};

/// Largest region edge decoded per read.
const EDGE: u64 = 256;
/// Series, planes and associated images visited per input.
const LIMIT: usize = 8;

/// One empty working folder per fuzzing process.
fn workdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scopeio-fuzz-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Fuzzes a single-file format: `data` becomes the file `name`.
pub fn file(data: &[u8], name: &str) {
    let path = workdir().join(name);
    if std::fs::write(&path, data).is_ok() {
        exercise(&path);
    }
}

/// Fuzzes a folder format: `data` is a sequence of entries
/// `relative path, 0x00, u32 LE length, bytes` unpacked below the folder
/// `root`, which is then opened. Paths leaving the folder are dropped.
pub fn tree(data: &[u8], root: &str) {
    let dir = workdir().join(root);
    let mut rest = data;
    while let Some(nul) = rest.iter().position(|&b| b == 0) {
        let (name, tail) = (&rest[..nul], &rest[nul + 1..]);
        let Some(len) = tail.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) else { break };
        let body = &tail[4..][..len.min(tail.len() - 4)];
        rest = &tail[4 + body.len()..];
        let Ok(name) = std::str::from_utf8(name) else { continue };
        let rel = Path::new(name);
        if name.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            continue;
        }
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, body);
    }
    exercise(&dir);
}

/// Opens `path` and walks the public API with small reads.
fn exercise(path: &Path) {
    let Ok(r) = scopeio::open(path) else { return };
    let _ = r.format();
    let _ = r.plates();
    for s in 0..r.series_count().min(LIMIT) {
        let Ok(m) = r.series_meta(s) else { continue };
        let m = m.clone();
        let _ = r.original_metadata(s);
        let corners = |n: u32| if n > 1 { vec![0, n - 1] } else { vec![0] };
        for c in corners(m.size_c) {
            for z in corners(m.size_z) {
                for t in corners(m.size_t) {
                    let _ = r.plane_meta(s, scopeio::Plane::new(c, z, t));
                    for level in 0..m.levels.len().min(LIMIT) {
                        let lv = &m.levels[level];
                        for (x, y) in [(0, 0), (lv.size_x.saturating_sub(EDGE), lv.size_y.saturating_sub(EDGE))] {
                            let region = Region::new(x, y, lv.size_x.min(EDGE), lv.size_y.min(EDGE));
                            let mut req = RegionRequest::new(s, level, region);
                            (req.z, req.t, req.channels) = (z, t, ChannelSelect::Channel(c));
                            let _ = r.read_region(&req);
                        }
                    }
                }
            }
        }
    }
    associated(r.as_ref());
}

/// Decodes the associated images (labels, macros, thumbnails).
fn associated(r: &dyn ImageReader) {
    for kind in r.associated_images().iter().take(LIMIT) {
        let _ = r.read_associated(kind, &Default::default());
    }
}

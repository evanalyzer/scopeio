//! # avi
//!
//! Fuzz target for the AVI reader: arbitrary bytes as an `.avi` file are
//! opened and every small frame is decoded; nothing may panic.
//!
//! Run: `cargo +nightly fuzz run avi fuzz/corpus/avi` (seed the corpus with
//! `tests/data/avi/*.avi`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#![no_main]

use libfuzzer_sys::fuzz_target;
use scopeio::{ChannelSelect, Region, RegionRequest};

fuzz_target!(|data: &[u8]| {
    // The readers open paths: one temporary file per process.
    let path = std::env::temp_dir().join(format!("scopeio-fuzz-{}.avi", std::process::id()));
    if std::fs::write(&path, data).is_err() {
        return;
    }
    if let Ok(r) = scopeio::open(&path) {
        for s in 0..r.series_count() {
            let Ok(m) = r.series_meta(s) else { continue };
            let (w, h) = (m.levels[0].size_x, m.levels[0].size_y);
            if w * h > 1 << 20 {
                continue;
            }
            for t in 0..m.size_t.min(8) {
                let mut req = RegionRequest::new(s, 0, Region::new(0, 0, w, h));
                (req.t, req.channels) = (t, ChannelSelect::Channel(0));
                let _ = r.read_region(&req);
            }
        }
    }
});

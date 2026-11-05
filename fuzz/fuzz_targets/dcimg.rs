//! # dcimg
//!
//! Fuzz target for the DCIMG reader: arbitrary bytes as a `.dcimg` file are
//! opened and read through the public API; nothing may panic.
//!
//! Run: `cargo +nightly fuzz run dcimg fuzz/corpus/dcimg` (seed the corpus
//! with `python3 -I fuzz/seed.py`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| scopeio_fuzz::file(data, "f.dcimg"));

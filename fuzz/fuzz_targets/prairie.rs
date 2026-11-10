//! # prairie
//!
//! Fuzz target for the Prairie View reader: a folder tree (see `scopeio_fuzz::tree`)
//! whose `rec.xml` is
//! opened and read through the public API; nothing may panic.
//!
//! Run: `cargo +nightly fuzz run prairie fuzz/corpus/prairie` (seed the corpus
//! with `python3 -I fuzz/seed.py`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-10
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| scopeio_fuzz::tree_file(data, "rec", "rec.xml"));

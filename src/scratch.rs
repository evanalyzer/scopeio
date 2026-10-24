//! # scratch
//!
//! Thread-local scratch buffers for compressed and intermediate data, reused
//! across reads to avoid allocations. Buffers larger than [`RETAIN_MAX`]
//! are released after use.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::cell::RefCell;

/// Largest buffer kept for reuse per thread and nesting depth.
const RETAIN_MAX: usize = 32 << 20;

thread_local! {
    /// Free buffers; nested calls take one each.
    static POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` with a scratch buffer of exactly `len` bytes.
///
/// The content is unspecified (zero on first use, stale data afterwards);
/// callers must overwrite what they read.
pub(crate) fn with<R>(len: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
    let mut buf = POOL.with(|p| p.borrow_mut().pop()).unwrap_or_default();
    if buf.len() < len {
        buf.resize(len, 0);
    }
    let result = f(&mut buf[..len]);
    if buf.capacity() <= RETAIN_MAX {
        POOL.with(|p| p.borrow_mut().push(buf));
    }
    result
}

/// Like [`with`], but the buffer is zero-filled.
pub(crate) fn with_zeroed<R>(len: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
    with(len, |buf| {
        buf.fill(0);
        f(buf)
    })
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_buffers_are_distinct_and_reused() {
        let outer = with(16, |a| {
            a.fill(1);
            with(8, |b| {
                b.fill(2);
                b.as_ptr() as usize
            });
            a.iter().all(|&v| v == 1)
        });
        assert!(outer);
        with_zeroed(16, |a| assert!(a.iter().all(|&v| v == 0)));
    }
}

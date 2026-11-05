//! # scratch
//!
//! Thread-local scratch buffers for compressed and intermediate data, reused
//! across reads to avoid allocations. Buffers larger than [`RETAIN_MAX`]
//! are released after use; requests above [`LEN_MAX`] fail, so sizes read
//! from corrupt headers cannot exhaust memory.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::cell::RefCell;

use crate::{Error, Result};

/// Largest buffer kept for reuse per thread and nesting depth.
const RETAIN_MAX: usize = 32 << 20;
/// Largest buffer handed out: 2 GiB, above the largest chunk, frame or
/// strip a valid file needs at once.
pub(crate) const LEN_MAX: usize = 2 << 30;

thread_local! {
    /// Free buffers; nested calls take one each.
    static POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` with a scratch buffer of exactly `len` bytes.
///
/// The content is unspecified (zero on first use, stale data afterwards);
/// callers must overwrite what they read.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] if `len` exceeds [`LEN_MAX`] or cannot be
/// allocated, else the result of `f`.
pub(crate) fn with<R>(len: usize, f: impl FnOnce(&mut [u8]) -> Result<R>) -> Result<R> {
    if len > LEN_MAX {
        return Err(Error::Corrupt(format!("buffer of {len} bytes requested")));
    }
    let mut buf = POOL.with(|p| p.borrow_mut().pop()).unwrap_or_default();
    if buf.len() < len {
        if buf.try_reserve_exact(len - buf.len()).is_err() {
            return Err(Error::Corrupt(format!("buffer of {len} bytes cannot be allocated")));
        }
        buf.resize(len, 0);
    }
    let result = f(&mut buf[..len]);
    if buf.capacity() <= RETAIN_MAX {
        POOL.with(|p| p.borrow_mut().push(buf));
    }
    result
}

/// Like [`with`], but the buffer is zero-filled.
pub(crate) fn with_zeroed<R>(len: usize, f: impl FnOnce(&mut [u8]) -> Result<R>) -> Result<R> {
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
                Ok(b.as_ptr() as usize)
            })?;
            Ok(a.iter().all(|&v| v == 1))
        });
        assert!(outer.unwrap());
        assert!(with_zeroed(16, |a| Ok(a.iter().all(|&v| v == 0))).unwrap());
    }

    #[test]
    fn oversized_requests_fail() {
        assert!(matches!(with(LEN_MAX + 1, |_| Ok(())), Err(Error::Corrupt(_))));
    }
}

//! # alloc
//!
//! Counts heap allocations of reads. Kept in its own test binary because
//! it installs a global allocator.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use scopeio::{ChannelSelect, Layout, OutputSpec, OutputType, PixelBuf, PixelViewMut, Region, RegionRequest};

/// System allocator that counts allocations of the current thread.
struct Counting;

thread_local! {
    static COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
static TOTAL: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards to the system allocator unchanged; counting has no
// effect on the returned memory. Test code only.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        TOTAL.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// Allocations made by `f` on this thread.
fn allocations(f: impl FnOnce()) -> usize {
    let before = COUNT.with(|c| c.get());
    f();
    COUNT.with(|c| c.get()) - before
}

#[test]
fn uncompressed_reads_allocate_nothing() {
    for file in ["tests/data/tiff/same_rgb16_le_chunky.tif", "tests/data/tiff/u8_le.tif", "tests/external/tif_01.tif"] {
        let path = common::root().join(file);
        if !path.exists() {
            continue;
        }
        let r = scopeio::open(&path).unwrap();
        let l = &r.series_meta(0).unwrap().levels[0];
        let region = Region::new(1, 1, l.size_x - 2, l.size_y - 2);
        let req = RegionRequest::new(0, 0, region);
        let shape = r.output_shape(&req).unwrap();
        let mut buf = PixelBuf::zeros(shape.pixel_type, shape.len());
        let mut read = || {
            let view = PixelViewMut::contiguous(buf.as_slice_mut(), shape.width, shape.height, shape.channels, shape.layout);
            r.read_region_into(&req, view).unwrap();
        };
        // The first read loads the chunk table.
        read();
        assert_eq!(allocations(read), 0, "{file}");
    }
}

#[test]
fn converted_reads_allocate_only_bounded_scratch() {
    let path = common::root().join("tests/data/tiff/gray3_planar.tif");
    let r = scopeio::open(&path).unwrap();
    let l = &r.series_meta(0).unwrap().levels[0];
    let mut req = RegionRequest::new(0, 0, Region::new(0, 0, l.size_x, l.size_y));
    req.channels = ChannelSelect::Channels(vec![0, 1, 2]);
    req.output = OutputSpec { layout: Layout::Chw, dtype: OutputType::F32Normalized, ..Default::default() };
    let shape = r.output_shape(&req).unwrap();
    let mut buf = PixelBuf::zeros(shape.pixel_type, shape.len());
    let mut read = || {
        let view = PixelViewMut::contiguous(buf.as_slice_mut(), shape.width, shape.height, shape.channels, shape.layout);
        r.read_region_into(&req, view).unwrap();
    };
    read();
    // Scratch buffers are reused per thread after the first read.
    assert_eq!(allocations(read), 0);
}

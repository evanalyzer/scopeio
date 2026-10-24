//! # scopeio
//!
//! Fast, pure-Rust, clean-room reader for microscopy and whole-slide images.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.
//!
//! ## Data model
//! A file contains one or more **series** (scenes, positions, wells). Each
//! series has resolution **levels** (level 0 is full resolution) and
//! **planes** addressed by channel, z-slice and time point. A colour image
//! is one channel with 3 (or 4) samples per pixel. Label, macro and
//! thumbnail images are associated images, not series.
//!
//! ## Ready-to-use output
//! Reads return native-endian, unpacked, correctly typed samples in the
//! requested [`Layout`] (Hwc or Chw), [`ColorOrder`] and [`OutputType`],
//! whatever the file stores. Buffers can be moved as they are into
//! kornia-rs, OpenCV, ndarray or tch:
//!
//! ```no_run
//! use scopeio::{ChannelSelect, Layout, OutputSpec, Region, RegionRequest};
//!
//! let reader = scopeio::open("image.ome.tif".as_ref())?;
//! let mut req = RegionRequest::new(0, 0, Region::new(0, 0, 512, 512));
//! req.channels = ChannelSelect::Channels(vec![0, 1, 2]);
//! req.output = OutputSpec { layout: Layout::Chw, ..Default::default() };
//! let img = reader.read_region(&req)?; // [3, 512, 512]
//! let data: Vec<u16> = img.into_vec()?;
//! # Ok::<(), scopeio::Error>(())
//! ```
//!
//! ## Units
//! All metadata uses fixed units: lengths and wavelengths in nanometres,
//! times in seconds. Field names carry the unit (`_nm`, `_s`).
//!
//! ## Threads
//! Handles are `Send + Sync` and stateless: share one `Arc<dyn ImageReader>`
//! between threads and read tiles in parallel. scopeio keeps no pixel cache
//! and starts no threads.

mod codecs;
mod errors;
mod formats;
mod io;
mod pixel;
mod reader;
mod scratch;
mod types;

pub use errors::{Error, Result};
pub use formats::{open, open_with, probe, supported_formats};
pub use pixel::Pixel;
pub use reader::ImageReader;
pub use types::*;

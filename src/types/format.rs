//! # format
//!
//! Format identification and open options.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// File formats scopeio can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FormatId {
    Bmp,
    Tiff,
    OmeTiff,
    OmeZarr,
    Vsi,
    Nd2,
    Czi,
    Lif,
    Svs,
    Ndpi,
    Mrxs,
    Lsm,
}

/// Identification of the reader that produced the data, for
/// reproducibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormatInfo {
    pub id: FormatId,
    /// Human-readable format name.
    pub name: &'static str,
    /// Version of the format reader; changes when its output changes.
    pub reader_version: &'static str,
    /// Version of the scopeio crate.
    pub crate_version: &'static str,
}

impl FormatInfo {
    pub(crate) const fn new(id: FormatId, name: &'static str, reader_version: &'static str) -> Self {
        Self { id, name, reader_version, crate_version: env!("CARGO_PKG_VERSION") }
    }
}

/// Where multi-channel and multi-tile reads may run in parallel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Parallelism {
    /// Sequential; the caller parallelises, e.g. one task per tile.
    #[default]
    Caller,
    /// Region reads may use the global rayon pool. Not implemented yet;
    /// reads are sequential.
    Rayon,
}

/// Options for [`open_with`](crate::open_with).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenOptions {
    /// Memory-map uncompressed data. Not implemented yet; opening with
    /// `true` fails.
    pub mmap: bool,
    pub parallelism: Parallelism,
    /// Upper bound of file handles kept open by multi-file formats. Files
    /// beyond it are opened for each read and closed again.
    pub max_open_files: usize,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self { mmap: false, parallelism: Parallelism::Caller, max_open_files: 64 }
    }
}

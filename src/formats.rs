//! # formats
//!
//! Format readers, format detection and the entry points.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#[cfg(feature = "bmp")]
mod bmp;
#[cfg(feature = "czi")]
mod czi;
#[cfg(feature = "lif")]
mod lif;
#[cfg(feature = "tiff")]
mod slide;
#[cfg(feature = "svs")]
mod svs;
#[cfg(feature = "nd2")]
mod nd2;
#[cfg(feature = "tiff")]
mod tiff;
#[cfg(feature = "vsi")]
mod vsi;
#[cfg(feature = "zarr")]
mod zarr;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::reader::Reader;
use crate::{Error, FormatId, FormatInfo, ImageReader, OpenOptions, Result};

/// Number of leading bytes inspected to detect the format.
const MAGIC_LEN: usize = 512;

/// Formats compiled into this build.
static SUPPORTED: &[FormatInfo] = &[
    #[cfg(feature = "bmp")]
    bmp::FORMAT,
    #[cfg(feature = "tiff")]
    tiff::FORMAT,
    #[cfg(feature = "tiff")]
    tiff::OME_FORMAT,
    #[cfg(feature = "zarr")]
    zarr::FORMAT,
    #[cfg(feature = "vsi")]
    vsi::FORMAT,
    #[cfg(feature = "nd2")]
    nd2::FORMAT,
    #[cfg(feature = "czi")]
    czi::FORMAT,
    #[cfg(feature = "lif")]
    lif::FORMAT,
    #[cfg(feature = "svs")]
    svs::FORMAT,
];

/// Formats this build can read.
pub fn supported_formats() -> &'static [FormatInfo] {
    SUPPORTED
}

/// Detects the format of a file from its content.
///
/// # Returns
///
/// The format, or `None` if no reader of this build recognises the file.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read.
pub fn probe(path: &Path) -> Result<Option<FormatId>> {
    // Directory-based formats first: a directory cannot be read as a file.
    #[cfg(feature = "zarr")]
    if let Some(id) = zarr::probe(path) {
        return Ok(Some(id));
    }
    if path.is_dir() {
        return Ok(None);
    }
    let mut head = Vec::with_capacity(MAGIC_LEN);
    File::open(path)?.take(MAGIC_LEN as u64).read_to_end(&mut head)?;
    #[cfg(feature = "bmp")]
    if bmp::matches(&head) {
        return Ok(Some(FormatId::Bmp));
    }
    #[cfg(feature = "lif")]
    if lif::matches(&head) {
        return Ok(Some(FormatId::Lif));
    }
    #[cfg(feature = "czi")]
    if czi::matches(&head) {
        return Ok(Some(FormatId::Czi));
    }
    #[cfg(feature = "nd2")]
    if nd2::matches(&head) {
        return Ok(Some(FormatId::Nd2));
    }
    // VSI files start with a TIFF header; check them before plain TIFF.
    #[cfg(feature = "vsi")]
    if vsi::matches(&head) {
        return Ok(Some(FormatId::Vsi));
    }
    #[cfg(feature = "tiff")]
    if let Some(id) = tiff::probe(path, &head)? {
        return Ok(Some(id));
    }
    let _ = &head;
    Ok(None)
}

/// Opens an image file with default options.
///
/// # Errors
///
/// Returns [`Error::UnsupportedFormat`] if no reader recognises the file, or
/// the error of the matching reader.
pub fn open(path: &Path) -> Result<Arc<dyn ImageReader>> {
    open_with(path, &OpenOptions::default())
}

/// Opens an image file.
///
/// # Errors
///
/// Same as [`open`]; [`Error::UnsupportedVariant`] if an option is not
/// implemented.
pub fn open_with(path: &Path, opts: &OpenOptions) -> Result<Arc<dyn ImageReader>> {
    if opts.mmap {
        return Err(Error::UnsupportedVariant("memory mapping is not implemented yet".into()));
    }
    let _ = opts;
    match probe(path)? {
        #[cfg(feature = "bmp")]
        Some(FormatId::Bmp) => Ok(Arc::new(Reader(bmp::Bmp::open(path)?))),
        #[cfg(feature = "tiff")]
        Some(FormatId::Tiff | FormatId::OmeTiff) => Ok(Arc::new(Reader(tiff::Tiff::open(path, opts)?))),
        #[cfg(feature = "svs")]
        Some(FormatId::Svs) => Ok(Arc::new(Reader(svs::open(path, opts)?))),
        #[cfg(feature = "lif")]
        Some(FormatId::Lif) => Ok(Arc::new(Reader(lif::Lif::open(path, opts)?))),
        #[cfg(feature = "czi")]
        Some(FormatId::Czi) => Ok(Arc::new(Reader(czi::Czi::open(path, opts)?))),
        #[cfg(feature = "nd2")]
        Some(FormatId::Nd2) => Ok(Arc::new(Reader(nd2::Nd2::open(path, opts)?))),
        #[cfg(feature = "vsi")]
        Some(FormatId::Vsi) => Ok(Arc::new(Reader(vsi::Vsi::open(path, opts)?))),
        #[cfg(feature = "zarr")]
        Some(FormatId::OmeZarr) => Ok(Arc::new(Reader(zarr::Zarr::open(path, opts)?))),
        _ => Err(Error::UnsupportedFormat(path.display().to_string())),
    }
}

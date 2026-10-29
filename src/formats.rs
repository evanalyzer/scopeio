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

#[cfg(feature = "bdv")]
mod bdv;
#[cfg(feature = "bif")]
mod bif;
#[cfg(feature = "bmp")]
mod bmp;
#[cfg(any(feature = "oif", feature = "zvi", feature = "xrm"))]
mod cfb;
#[cfg(feature = "oif")]
mod oif;
#[cfg(feature = "stk")]
mod nd;
#[cfg(feature = "oir")]
mod oir;
#[cfg(feature = "omp2info")]
mod omp2info;
#[cfg(feature = "operetta")]
mod operetta;
#[cfg(feature = "zvi")]
mod zvi;
#[cfg(feature = "czi")]
mod czi;
#[cfg(feature = "dicom")]
mod dicom;
#[cfg(feature = "dv")]
mod dv;
#[cfg(any(feature = "ims", feature = "bdv"))]
mod hdf5;
#[cfg(feature = "ims")]
mod ims;
#[cfg(feature = "lif")]
mod lif;
#[cfg(feature = "tiff")]
mod slide;
#[cfg(feature = "lsm")]
mod lsm;
#[cfg(feature = "micromanager")]
mod micromanager;
#[cfg(feature = "mrc")]
mod mrc;
#[cfg(feature = "mrxs")]
mod mrxs;
#[cfg(feature = "nifti")]
mod nifti;
#[cfg(any(feature = "png", feature = "jpg", feature = "jp2"))]
mod plain;
#[cfg(feature = "qptiff")]
mod qptiff;
#[cfg(feature = "scanr")]
mod scanr;
#[cfg(feature = "scn")]
mod scn;
#[cfg(feature = "ndpi")]
mod ndpi;
#[cfg(feature = "ndpi")]
mod ndpis;
#[cfg(feature = "svs")]
mod svs;
#[cfg(feature = "nd2")]
mod nd2;
#[cfg(feature = "tiff")]
mod tiff;
#[cfg(feature = "vsi")]
mod vsi;
#[cfg(feature = "vms")]
mod vms;
#[cfg(feature = "xrm")]
mod xrm;
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
    #[cfg(feature = "omexml")]
    tiff::omexml::FORMAT,
    #[cfg(feature = "stk")]
    tiff::STK_FORMAT,
    #[cfg(feature = "sis")]
    tiff::SIS_FORMAT,
    #[cfg(feature = "fluoview")]
    tiff::FLUOVIEW_FORMAT,
    #[cfg(feature = "nikon-tiff")]
    tiff::NIKON_FORMAT,
    #[cfg(feature = "nef")]
    tiff::NEF_FORMAT,
    #[cfg(feature = "stk")]
    nd::FORMAT,
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
    #[cfg(feature = "lif")]
    lif::LOF_FORMAT,
    #[cfg(feature = "lif")]
    lif::XLEF_FORMAT,
    #[cfg(feature = "svs")]
    svs::FORMAT,
    #[cfg(feature = "ndpi")]
    ndpi::FORMAT,
    #[cfg(feature = "ndpi")]
    ndpis::FORMAT,
    #[cfg(feature = "vms")]
    vms::FORMAT,
    #[cfg(feature = "mrxs")]
    mrxs::FORMAT,
    #[cfg(feature = "lsm")]
    lsm::FORMAT,
    #[cfg(feature = "qptiff")]
    qptiff::FORMAT,
    #[cfg(feature = "scn")]
    scn::FORMAT,
    #[cfg(feature = "bif")]
    bif::FORMAT,
    #[cfg(feature = "ims")]
    ims::FORMAT,
    #[cfg(feature = "bdv")]
    bdv::FORMAT,
    #[cfg(feature = "oif")]
    oif::OIB_FORMAT,
    #[cfg(feature = "oif")]
    oif::OIF_FORMAT,
    #[cfg(feature = "oir")]
    oir::FORMAT,
    #[cfg(feature = "omp2info")]
    omp2info::FORMAT,
    #[cfg(feature = "dicom")]
    dicom::FORMAT,
    #[cfg(feature = "zvi")]
    zvi::FORMAT,
    #[cfg(feature = "xrm")]
    xrm::FORMAT,
    #[cfg(feature = "operetta")]
    operetta::FORMAT,
    #[cfg(feature = "dv")]
    dv::FORMAT,
    #[cfg(feature = "mrc")]
    mrc::FORMAT,
    #[cfg(feature = "nifti")]
    nifti::FORMAT,
    #[cfg(feature = "nifti")]
    nifti::ANALYZE_FORMAT,
    #[cfg(feature = "scanr")]
    scanr::FORMAT,
    #[cfg(feature = "micromanager")]
    micromanager::FORMAT,
    #[cfg(feature = "png")]
    plain::PNG_FORMAT,
    #[cfg(feature = "jpg")]
    plain::JPEG_FORMAT,
    #[cfg(feature = "jp2")]
    plain::JP2_FORMAT,
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
    #[cfg(feature = "micromanager")]
    if micromanager::matches_dir(path) {
        return Ok(Some(FormatId::MicroManager));
    }
    if path.is_dir() {
        return Ok(None);
    }
    // The .mrxs file is a preview JPEG; its slide folder identifies MIRAX.
    #[cfg(feature = "mrxs")]
    if mrxs::matches(path) {
        return Ok(Some(FormatId::Mrxs));
    }
    let mut head = Vec::with_capacity(MAGIC_LEN);
    File::open(path)?.take(MAGIC_LEN as u64).read_to_end(&mut head)?;
    #[cfg(feature = "bmp")]
    if bmp::matches(&head) {
        return Ok(Some(FormatId::Bmp));
    }
    #[cfg(feature = "dicom")]
    if dicom::matches(&head) {
        return Ok(Some(FormatId::Dicom));
    }
    #[cfg(feature = "omp2info")]
    if omp2info::matches(path, &head) {
        return Ok(Some(FormatId::Omp2info));
    }
    #[cfg(feature = "oir")]
    if oir::matches(&head) {
        return Ok(Some(FormatId::Oir));
    }
    #[cfg(feature = "stk")]
    if nd::matches(path, &head) {
        return Ok(Some(FormatId::Nd));
    }
    // Micro-Manager stacks are OME-TIFF files; check them before TIFF.
    #[cfg(feature = "micromanager")]
    if micromanager::matches(path, &head) {
        return Ok(Some(FormatId::MicroManager));
    }
    #[cfg(feature = "scanr")]
    if scanr::matches(path, &head) {
        return Ok(Some(FormatId::ScanR));
    }
    #[cfg(feature = "operetta")]
    if operetta::matches(path, &head) {
        return Ok(Some(FormatId::Operetta));
    }
    #[cfg(feature = "xrm")]
    if xrm::matches(path, &head) {
        return Ok(Some(FormatId::Xrm));
    }
    #[cfg(feature = "zvi")]
    if zvi::matches(path, &head) {
        return Ok(Some(FormatId::Zvi));
    }
    #[cfg(feature = "oif")]
    if oif::matches_oib(path, &head) {
        return Ok(Some(FormatId::Oib));
    }
    #[cfg(feature = "oif")]
    if oif::matches_oif(path, &head) {
        return Ok(Some(FormatId::Oif));
    }
    #[cfg(feature = "bdv")]
    if bdv::matches(path, &head) {
        return Ok(Some(FormatId::Bdv));
    }
    #[cfg(feature = "ims")]
    if ims::matches(path, &head) {
        return Ok(Some(FormatId::Ims));
    }
    #[cfg(feature = "vms")]
    if vms::matches(path, &head) {
        return Ok(Some(FormatId::Vms));
    }
    #[cfg(feature = "ndpi")]
    if ndpis::matches(&head) {
        return Ok(Some(FormatId::Ndpis));
    }
    #[cfg(feature = "lif")]
    if lif::matches(&head) {
        return Ok(Some(FormatId::Lif));
    }
    #[cfg(feature = "lif")]
    if lif::matches_lof(&head) {
        return Ok(Some(FormatId::Lof));
    }
    #[cfg(feature = "lif")]
    if lif::matches_xml(path, &head) {
        return Ok(Some(FormatId::Xlef));
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
    // Inline-pixel OME-XML before OME-TIFF companion files.
    #[cfg(feature = "omexml")]
    if tiff::omexml::matches(path, &head) {
        return Ok(Some(FormatId::OmeXml));
    }
    #[cfg(feature = "tiff")]
    if let Some(id) = tiff::probe(path, &head)? {
        return Ok(Some(id));
    }
    #[cfg(feature = "dv")]
    if dv::matches(&head) {
        return Ok(Some(FormatId::DeltaVision));
    }
    #[cfg(feature = "nifti")]
    if let Some(id) = nifti::probe(path, &head) {
        return Ok(Some(id));
    }
    #[cfg(feature = "mrc")]
    if mrc::matches(path, &head) {
        return Ok(Some(FormatId::Mrc));
    }
    #[cfg(any(feature = "png", feature = "jpg", feature = "jp2"))]
    if let Some(id) = plain::probe(&head) {
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
        Some(FormatId::Tiff | FormatId::OmeTiff | FormatId::Stk | FormatId::Sis | FormatId::FluoView | FormatId::NikonTiff | FormatId::Nef) => Ok(Arc::new(Reader(tiff::Tiff::open(path, opts)?))),
        #[cfg(feature = "omexml")]
        Some(FormatId::OmeXml) => Ok(Arc::new(Reader(tiff::omexml::OmeXml::open(path)?))),
        #[cfg(any(feature = "png", feature = "jpg", feature = "jp2"))]
        Some(id @ (FormatId::Png | FormatId::Jpeg | FormatId::Jpeg2000)) => Ok(Arc::new(Reader(plain::Plain::open(path, id)?))),
        #[cfg(feature = "stk")]
        Some(FormatId::Nd) => Ok(Arc::new(Reader(nd::Nd::open(path, opts)?))),
        #[cfg(feature = "dv")]
        Some(FormatId::DeltaVision) => Ok(Arc::new(Reader(dv::Dv::open(path)?))),
        #[cfg(feature = "nifti")]
        Some(FormatId::Nifti | FormatId::Analyze) => Ok(Arc::new(Reader(nifti::Nifti::open(path)?))),
        #[cfg(feature = "mrc")]
        Some(FormatId::Mrc) => Ok(Arc::new(Reader(mrc::Mrc::open(path)?))),
        #[cfg(feature = "micromanager")]
        Some(FormatId::MicroManager) => Ok(Arc::new(Reader(micromanager::MicroManager::open(path, opts)?))),
        #[cfg(feature = "scanr")]
        Some(FormatId::ScanR) => Ok(Arc::new(Reader(scanr::ScanR::open(path)?))),
        #[cfg(feature = "operetta")]
        Some(FormatId::Operetta) => Ok(Arc::new(Reader(operetta::Operetta::open(path)?))),
        #[cfg(feature = "xrm")]
        Some(FormatId::Xrm) => Ok(Arc::new(Reader(xrm::Xrm::open(path)?))),
        #[cfg(feature = "zvi")]
        Some(FormatId::Zvi) => Ok(Arc::new(Reader(zvi::Zvi::open(path)?))),
        #[cfg(feature = "dicom")]
        Some(FormatId::Dicom) => Ok(Arc::new(Reader(dicom::Dicom::open(path)?))),
        #[cfg(feature = "omp2info")]
        Some(FormatId::Omp2info) => Ok(Arc::new(Reader(omp2info::Omp2info::open(path)?))),
        #[cfg(feature = "oir")]
        Some(FormatId::Oir) => Ok(Arc::new(Reader(oir::Oir::open(path)?))),
        #[cfg(feature = "oif")]
        Some(id @ (FormatId::Oib | FormatId::Oif)) => Ok(Arc::new(Reader(oif::Oif::open(path, id)?))),
        #[cfg(feature = "bdv")]
        Some(FormatId::Bdv) => Ok(Arc::new(Reader(bdv::Bdv::open(path)?))),
        #[cfg(feature = "ims")]
        Some(FormatId::Ims) => Ok(Arc::new(Reader(ims::Ims::open(path, opts)?))),
        #[cfg(feature = "bif")]
        Some(FormatId::Bif) => Ok(Arc::new(Reader(bif::Bif::open(path, opts)?))),
        #[cfg(feature = "scn")]
        Some(FormatId::Scn) => Ok(Arc::new(Reader(scn::open(path, opts)?))),
        #[cfg(feature = "qptiff")]
        Some(FormatId::Qptiff) => Ok(Arc::new(Reader(qptiff::open(path, opts)?))),
        #[cfg(feature = "lsm")]
        Some(FormatId::Lsm) => Ok(Arc::new(Reader(lsm::open(path, opts)?))),
        #[cfg(feature = "mrxs")]
        Some(FormatId::Mrxs) => Ok(Arc::new(Reader(mrxs::Mrxs::open(path, opts)?))),
        #[cfg(feature = "ndpi")]
        Some(FormatId::Ndpi) => Ok(Arc::new(Reader(ndpi::open(path, opts)?))),
        #[cfg(feature = "vms")]
        Some(FormatId::Vms) => Ok(Arc::new(Reader(vms::Vms::open(path)?))),
        #[cfg(feature = "ndpi")]
        Some(FormatId::Ndpis) => Ok(Arc::new(Reader(ndpis::Ndpis::open(path, opts)?))),
        #[cfg(feature = "svs")]
        Some(FormatId::Svs) => Ok(Arc::new(Reader(svs::open(path, opts)?))),
        #[cfg(feature = "lif")]
        Some(FormatId::Lif | FormatId::Lof | FormatId::Xlef) => Ok(Arc::new(Reader(lif::Lif::open(path, opts)?))),
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

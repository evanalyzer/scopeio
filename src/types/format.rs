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
    /// Hamamatsu NDPIS fluorescence set (one NDPI per channel).
    Ndpis,
    Mrxs,
    Lsm,
    Qptiff,
    Scn,
    Bif,
    Ims,
    Png,
    Jpeg,
    /// JPEG 2000 file (JP2) or raw codestream (J2K).
    Jpeg2000,
    /// Olympus FluoView compound file.
    Oib,
    /// Olympus FluoView settings file with its `.oif.files` folder.
    Oif,
    Oir,
    Dicom,
    Zvi,
    /// Leica object file (one image).
    Lof,
    /// Leica XML experiment, collection or image file.
    Xlef,
    /// PerkinElmer/Revvity Harmony export (Operetta, Opera Phenix).
    Operetta,
    DeltaVision,
    /// MetaMorph stack (TIFF with UIC tags).
    Stk,
    /// MetaMorph multi-dimensional acquisition (`.nd` and its TIFF/STK files).
    Nd,
    /// MRC / CCP4 electron microscopy map or image stack.
    Mrc,
    /// Olympus Soft Imaging Solutions TIFF (analySIS, cellSens, iTEM).
    Sis,
    /// Olympus FluoView TIFF (FV300, FV500).
    FluoView,
    /// NIfTI-1 / NIfTI-2 (`.nii`, `.nii.gz`, `.hdr` + `.img`).
    Nifti,
    /// Olympus scan^R high-content screening experiment.
    ScanR,
    /// Nikon NIS-Elements TIFF (NIS private tags 65325-65333).
    NikonTiff,
    /// Nikon NEF camera raw (uncompressed raw data).
    Nef,
    /// Olympus/Evident multi-area acquisition (`.omp2info` and its OIR files).
    Omp2info,
    /// Micro-Manager image stacks, separate image files and NDTiff datasets.
    MicroManager,
    /// BigDataViewer XML/HDF5 (`.xml` + `.h5`).
    Bdv,
    /// Mayo Analyze 7.5 (`.hdr` + `.img`).
    Analyze,
    /// Hamamatsu VMS (`.vms` index and JPEG files).
    Vms,
    /// OME-XML with the pixels inline (`BinData`).
    OmeXml,
    /// Zeiss Xradia (`.txm`, `.txrm`, `.xrm`).
    Xrm,
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
    /// File extensions, lower case without the leading dot; multi-part
    /// extensions such as `ome.tif` or `nii.gz` included. Extensions shared
    /// with other formats (`tif`) are listed too: the content decides.
    pub extensions: &'static [&'static str],
    /// Fixed file names that identify the format (e.g. `Index.idx.xml`).
    pub file_names: &'static [&'static str],
    /// The format is opened as a folder (OME-Zarr), not a file.
    pub opens_folder: bool,
}

impl FormatInfo {
    pub(crate) const fn new(id: FormatId, name: &'static str, reader_version: &'static str) -> Self {
        Self {
            id,
            name,
            reader_version,
            crate_version: env!("CARGO_PKG_VERSION"),
            extensions: &[],
            file_names: &[],
            opens_folder: false,
        }
    }

    pub(crate) const fn extensions(mut self, extensions: &'static [&'static str]) -> Self {
        self.extensions = extensions;
        self
    }

    pub(crate) const fn file_names(mut self, names: &'static [&'static str]) -> Self {
        self.file_names = names;
        self
    }

    pub(crate) const fn folder(mut self) -> Self {
        self.opens_folder = true;
        self
    }

    /// Returns `true` if a file or folder named `name` may be of this
    /// format by its name: a listed extension (case-insensitive) or file
    /// name. Use it for file-dialog filters; [`probe`](crate::probe) decides
    /// by content.
    pub fn matches_name(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        self.file_names.iter().any(|n| n.eq_ignore_ascii_case(name))
            || self.extensions.iter().any(|e| lower.len() > e.len() && lower.ends_with(e) && lower.as_bytes()[lower.len() - e.len() - 1] == b'.')
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

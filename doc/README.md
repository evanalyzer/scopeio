# Format documentation

Reference material for implementing the readers in scopeio. Collected on
2026-10-24: 287 files, about 180 MB, one folder per format (`doc/<format>/`).

## Rules for using this folder

- **Specifications and documentation only.** No reader source code was
  downloaded. Bio-Formats pages and source were not opened (see
  `CLAUDE.local.md`).
- **Most documents are copyrighted by their publishers.** They are here
  for reference. Check the license column in the manifests before
  redistributing anything.
- **Community-written documents need a clean-room check** before a reader
  relies on them: who wrote them, and what they were derived from. The
  manifests have a section for these.

## Manifests

Each manifest lists every file with source URL, publisher, license and
notes, plus the formats for which nothing public was found.

| Manifest | Formats |
|---|---|
| [_manifest_tiff_wsi_codecs.md](_manifest_tiff_wsi_codecs.md) | TIFF family, OME-TIFF/XML/NGFF/Zarr, whole-slide formats, DNG/camera raw, compression codecs (`codecs/`) |
| [_manifest_vendor.md](_manifest_vendor.md) | Microscope and camera vendor formats (ND2, LIF, VSI, CZI, IMS, DV, SPE, SDT, PTU, ...) |
| [_manifest_scientific.md](_manifest_scientific.md) | MRC, NIfTI, Analyze, FITS, ICS, NRRD, MINC, HDF5, DICOM, EM formats, EPS/PostScript |
| [_manifest_raster_hcs_spm.md](_manifest_raster_hcs_spm.md) | PNG, GIF, BMP, TGA, PSD, AVI, QuickTime, ..., plate screening (HCS), scanning probe (AFM/SPM) |

## Coverage overview

**Official or vendor specification available**
- TIFF: TIFF 6.0 with tech notes, BigTIFF, DNG, OME-TIFF, OME-XML schema,
  OME-NGFF 0.4/0.5, Zarr v2/v3.
- Whole-slide: Aperio SVS (2008 whitepaper), Ventana BIF, QPTIFF.
- Codecs: JPEG, JFIF, Exif, JPEG 2000 (incl. JP2/JPX, HTJ2K), JPEG-XR,
  JPEG-LS, Deflate/zlib/gzip, Zstandard, WebP, LZ4, Blosc, PackBits. LZW
  is in TIFF 6.0 section 13.
- Scientific: MRC2014, IMOD, NIfTI-1/2, Analyze 7.5, FITS 4.0, ICS,
  NRRD, MINC, HDF5, DICOM (PS3.5, PS3.6, PS3.10 and the pixel and
  whole-slide sections of PS3.3).
- Other EM and medical formats: SPIDER, IMAGIC, Varian FDF, ECAT7, KLB,
  AmiraMesh, BDV, CellH5.
- Raster and video: PNG, APNG, MNG, GIF, BMP, PCX, TGA, Netpbm, PICT,
  PSD, AVI/RIFF, QuickTime, EPS.
- Vendor: Imaris IMS, Imspector OBF/MSR, PicoQuant PTU, Princeton SPE
  2.x/3.0, DeltaVision, SBIG, PCO B16, Becker & Hickl SDT (handbook),
  Micro-Manager/NDTiff.

**Community-written documentation only (review before use)**
- Nikon ND2, Leica LIF/LOF/XLEF, Olympus VSI/ETS, OIF/OIB, OIR.
- Zeiss CZI (libCZI docs pages, not the full specification), Zeiss ZVI,
  Andor SIF, Hamamatsu DCIMG, Gatan DM3/DM4.
- Canon CR2, Nikon NEF, ImageJ TIFF, Nanoscope, JPK.

**No public specification found**
- Many vendor TIFF variants: LSM (full specification), FluoView, NDPI,
  Leica SCN, MetaMorph STK, Prairie, Andor, Improvision, Nikon Elements.
- Most plate-screening formats (only user manuals for InCell, MetaXpress
  and Yokogawa).
- Most scanning-probe formats.
- Many small legacy formats.

The detailed lists, with URLs, are in the "Not found" section of each
manifest.

## Open points

- **`olympus-vsi/`:** the bioio-vsi document says its layout partly comes
  from slideio's (BSD-3) driver and was compared against Bio-Formats
  output as a black box. The OpenReadout notes (`_openreadout/` holds
  their license and clean-room policy) were derived from permissively
  licensed readers. Review provenance before relying on them.
- **`nikon-nd2/`:** nd2-rs `DATASTRUCTURE.md` is a port of the `nd2`
  Python package (BSD-3).
- **`becker-hickl-sdt/bh-TCSPC-Handbook-8th-edition.pdf`:** from a public
  university mirror. The vendor now puts newer editions behind a free
  registration.
- **`zeiss-lsm/Lsmfile.doc`:** an old Zeiss document rehosted by a third
  party. Its distribution terms are unknown.
- **Restrictive licenses:**
  - `canon-crw-cr2/` contains a CC BY-NC-SA page (non-commercial).
  - The OpenSlide format pages are CC BY-SA.
- **`amira/amira_31_user_manual.pdf`:** a vendor manual found under a
  university "internal" path.
- **Scanning-probe readers:** during collection, two scanning-probe reader
  programs (Topometrix v3, Park HDF) were briefly opened and then deleted
  without being saved. Whoever implements those two formats should know.
- **Worth saving manually from a browser** (JavaScript-only pages):
  - The Molecular Devices HTD article, which has an annotated example.
  - The MetaMorph STK article.
  - The current Apple QTFF page.

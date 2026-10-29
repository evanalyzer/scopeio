# Provenance

Sources used for each format reader. No GPL, LGPL or AGPL code was opened,
copied or used as a template. Bio-Formats and bioformats-rs were run only
as black-box command-line tools to produce reference output
(`tests/expected/`); their source code was not read.

## Windows BMP (`src/formats/bmp.rs`)

- Microsoft Learn: "Bitmap Storage", "Bitmap Compression", `BITMAPFILEHEADER`,
  `BITMAPCOREHEADER`, `BITMAPINFOHEADER`, `BITMAPV4HEADER`, `BITMAPV5HEADER`,
  `RGBQUAD` (`doc/bmp/`).
- Library of Congress format description fdd000189 (`doc/bmp/`).
- Own analysis of sample files; synthetic samples written from the
  documentation by `tests/scripts/make_bmp_samples.py`.

## TIFF, BigTIFF, ImageJ TIFF (`src/formats/tiff*`)

- TIFF 6.0 specification (Adobe), sections 2-9, 13-15, 18, 21
  (`doc/tiff/TIFF6.0_specification.pdf`).
- TIFF Technical Note 2 (JPEG in TIFF) and the Adobe floating-point
  predictor note (`doc/tiff/`).
- BigTIFF specification (`doc/bigtiff/`).
- ImageJ hyperstack description keys: own analysis of sample files.
- Own analysis of sample files (libtiff test images, condensation set);
  synthetic samples written from the specifications by
  `tests/scripts/make_tiff_samples.py`, independent samples written with
  ImageMagick/libtiff by `tests/scripts/make_tiff_im_samples.sh`.

## OME-TIFF (`src/formats/tiff/ome.rs`)

- OME-TIFF specification and file structure pages, OME 2016-06 schema
  documentation and `ome_2016-06.xsd` (`doc/ome-tiff/`, `doc/ome-xml/`).
- Official OME-TIFF sample data (downloads.openmicroscopy.org).

## OME-Zarr (`src/formats/zarr.rs`)

- OME-NGFF 0.4 and 0.5 specifications, Zarr v2 specification, Zarr v3 core
  specification and the v3 codec specifications for sharding, blosc, bytes,
  gzip, crc32c and transpose (`doc/ome-zarr/`, CC BY 3.0).
- numcodecs documentation of the Blosc and LZ4 codecs.
- Public IDR OME-Zarr data (idr0062A/6001240 v0.4 and v0.5, idr0001A/2551
  plate), downloaded by `tests/scripts/fetch_ome_zarr.py`; synthetic stores
  written from the specifications by `tests/scripts/make_zarr_samples.py`.

## Olympus/Evident VSI + ETS (`src/formats/vsi*`)

- OpenReadout VSI format notes and provenance log (MIT OR Apache-2.0;
  written from public sample files and permissively licensed readers, with
  Bio-Formats only run as a black box) and bioio-vsi format notes (BSD-3;
  partly from slideio's BSD-3 driver) in `doc/olympus-vsi/`.
- Own analysis of sample files: record-tree type words (`0x90000000` marks
  nested volumes, `0x40000000` inline values, `0x08000000` an index word),
  embedded single-file datasets (planes as further TIFF IFDs), pyramid level
  sizes checked against Bio-Formats' series sizes.
- Synthetic dataset written from the documented layout by
  `tests/scripts/make_vsi_samples.py`.

## Nikon ND2 (`src/formats/nd2*`)

- OpenReadout ND2 format notes and provenance log (MIT OR Apache-2.0) and
  the nd2-rs data structure notes (MIT, see `doc/nikon-nd2/LICENSE-nd2-rs-MIT.txt`;
  they follow the BSD-3 `nd2` Python package) in `doc/nikon-nd2/`. No Nikon
  SDK used.
- Own analysis of sample files: loop nesting (spectral loops without a
  count, XY validity masks), padded rows (`uiWidthBytes`), zlib frames,
  RGB frames stored blue first (checked on H&E slides: the plausible colour
  order is BGR in the file).
- Synthetic samples written from the documented layout by
  `tests/scripts/make_nd2_samples.py`.

## Zeiss CZI (`src/formats/czi*`)

- OpenReadout CZI format notes and provenance log (MIT OR Apache-2.0;
  derived from public samples, czifile's BSD-3 documentation and the libCZI
  documentation pages) in `doc/zeiss-czi/`.
- ZEISS libCZI documentation pages (MIT per libCZI's REUSE file: `docs/*`;
  libCZI's source code is LGPL and was not consulted): chunked compression,
  resolution protocol, valid-pixel mask concept, reader compatibility.
- Own analysis of sample files: mosaic tiles are drawn in directory order
  (later entries on top), checked pixel by pixel against Bio-Formats as a
  black box; pyramid levels from the downsampling ratio of each subblock,
  snapped to a power of two (level sizes agree with Bio-Formats).
- Multi-file documents follow the inferred layout of the OpenReadout notes
  (part k next to the master as `<stem> (k).czi` or `<stem>(k).czi`, header
  `file_part` k and the master's primary GUID; entries naming an absent part
  read from the master when the segment there repeats the entry). No public
  multi-file CZI exists; the synthetic `multipart.czi` set was read by
  Bio-Formats as a black box, which found the same part files and returned
  the same pixels for every plane.
- Synthetic samples written from the documented layout by
  `tests/scripts/make_czi_samples.py`.

## Leica LIF (`src/formats/lif.rs`)

- OpenReadout LIF format notes and provenance log (MIT OR Apache-2.0;
  derived from public samples and liffile's BSD-3 documentation) in
  `doc/leica-lif/`. readlif (GPL) was not used.
- Own analysis of sample files: memory block headers (version 2, 64-bit
  lengths), planar and interleaved channels addressed by `BytesInc`, tile
  scans as one series per tile (as Bio-Formats lists them).
- Synthetic samples written from the documented layout by
  `tests/scripts/make_lif_samples.py`.
- LOF and the XML containers (XLEF, XLCF, XLLF, XLIF): liffile
  (BSD-3-Clause, Christoph Gohlke; notice in THIRD_PARTY_NOTICES.md) read as
  documentation: LOF layout (header text `LMS_Object_File`, memory block
  before the XML), `Children/Reference` and `Memory/Frame` elements,
  case-insensitive reference paths. Checked against Bio-Formats (black box)
  on the OME `Leica-XLEF` samples; synthetic files written by
  `tests/scripts/make_xlef_samples.py`.

## Aperio SVS (`src/formats/svs.rs`, `src/formats/slide.rs`)

- Aperio "Digital Slides and Third-Party Data Interchange" (2008) in
  `doc/aperio-svs/`: IFD order (baseline, thumbnail, pyramid, label, macro).
- openslide.org Aperio format page (prose, CC BY-SA 4.0): subfile types of
  label (1) and macro (9), JPEG 2000 compressions 33003 (YCbCr, no colour
  transform) and 33005 (RGB). OpenSlide's C code (LGPL) was not consulted.
- Own analysis of sample files; the YCbCr samples of 33003 tiles were
  checked against OpenJPEG (ImageMagick, black box): identical components,
  JFIF YCbCr to RGB conversion.
- Synthetic sample written from the documented layout by
  `tests/scripts/make_svs_samples.py`.

## Hamamatsu NDPI (`src/formats/ndpi.rs`, `src/formats/slide/restart.rs`)

- openslide.org Hamamatsu format page (prose, CC BY-SA 4.0) in
  `doc/hamamatsu-ndpi-vms/`: 64-bit header and IFD links, high 32 bits of
  entry values after each IFD, tags 65420/65421 (source lens; -1 macro, -2
  map), 65426/65432 (restart offsets). OpenSlide's C code (LGPL) was not
  consulted.
- ITU-T T.81 (JPEG): restart intervals reset the DC predictions, so each
  interval decodes on its own with the stream's tables and a patched frame
  size.
- Own analysis of sample files: restart offsets are relative to the JPEG
  stream and point after each RSTn marker.
- Synthetic sample written from the documented layout by
  `tests/scripts/make_ndpi_samples.py` (JPEG streams by ImageMagick, run as a
  program).

## Hamamatsu NDPIS (`src/formats/ndpis.rs`)

- Own analysis of the OME sample set `Hamamatsu-NDPI/manuel/test3.ndpis`:
  INI text `[NanoZoomer Digital Pathology Image Set]` with `NoImages` and
  `Image0..` naming one NDPI file per channel next to it. Each file is a
  pseudo-coloured RGB slide with the signal in one component; tag 65434
  holds the channel name, the CR-separated tag 65449 block holds
  `ExcitationWaveLength` / `EmissionWaveLength`.
- Bio-Formats and bioformats-rs run as black boxes to confirm the channel
  order and that each channel is the dominant component (pixels match
  bioformats-rs exactly). No GPL/LGPL code consulted.

## 3DHistech MIRAX (`src/formats/mrxs.rs`)

- openslide.org MIRAX format page (prose, CC BY-SA 4.0) in
  `doc/3dhistech-mrxs/`: Slidedat.ini sections, Index.dat record tables and
  data pages, data files, slide position buffer, associated images.
  OpenSlide's C code and ImHex patterns (LGPL repository) were not used.
- Own analysis of sample files: positions are level-0 pixels of each camera
  photo; stored images span 2^(concat + level) base images; placement of
  per-photo pieces and the drawing order (image number) were found by
  locating decoded images in renderings of OpenSlide 3.4.1 run as a black
  box (`openslide-write-png`).
- Slide version 2.2 and later (openslide.org page): the position buffer is
  DEFLATE-compressed and referenced by
  `StitchingIntensityLayer.StitchingIntensityLevel`; the sample
  `Mirax2.2-1` holds a zlib stream (header `78 9C`) and a second data item,
  which is ignored. Placement checked against OpenSlide crops.
- Synthetic samples written from that model by
  `tests/scripts/make_mrxs_samples.py` (JPEGs by ImageMagick, run as a program).

## Zeiss LSM (`src/formats/lsm.rs`)

- Carl Zeiss "File Format Description - LSM 5xx Release 2.0"
  (`doc/zeiss-lsm/Lsmfile.doc`): directory order, CZ_LSMINFO structure,
  channel colours and names, time stamps.
- Own analysis of sample files: the photometric tag (RGB or a 256-entry
  palette, also for 16-bit data) is for display only; samples are
  intensities.
- Synthetic sample written from the specification by
  `tests/scripts/make_lsm_samples.py`.

## Akoya QPTIFF (`src/formats/qptiff.rs`)

- Akoya "QPTIFF Specification" (appendix A of the PhenoImager HT user
  manual) in `doc/perkinelmer-qptiff/`: directory order, ImageDescription
  XML (ImageType, Name, Color, Objective, ExposureTime), resolution tags.
- Own analysis of sample files (OME sample server, CC BY 4.0).
- Synthetic sample written from the specification by
  `tests/scripts/make_qptiff_samples.py`.

## Leica SCN (`src/formats/scn.rs`)

- openslide.org Leica format page (prose, CC BY-SA 4.0) in `doc/leica-scn/`:
  collection XML, images, dimensions with TIFF directory numbers, macro
  image at the collection's origin and size. OpenSlide's C code (LGPL) was
  not consulted.
- Own analysis of sample files (fluorescence channels `c`, channel
  settings, exposure in microseconds).
- Synthetic sample written from the documented layout by
  `tests/scripts/make_scn_samples.py`.
- JPEG note: on 4:2:0 tiles zune-jpeg's chroma upsampling differs from
  libjpeg-turbo (which Bio-Formats matches exactly) by a mean of up to 0.4
  per channel; recorded as tolerance in `tests/expected/scn_divergences.tsv`.

## Ventana BIF (`src/formats/bif.rs`)

- Roche "BIF image file format for digital pathology" white paper (2020)
  in `doc/ventana-bif/`: directory order, iScan and EncodeInfo XMP, tile
  joints with horizontal overlap (Tile2 covers Tile1, serpentine
  numbering), unscanned tiles shown with ScanWhitePoint.
- openslide.org Ventana page (prose, CC BY-SA 4.0).
- Own analysis of sample files, checked against Bio-Formats as a black box:
  tile tables follow the TIFF tile grid of the padded image (not the order
  of the `Frame` nodes), and the stored lower levels were made from the
  unstitched grid, so their tile-column footprints are moved to the
  stitched positions.
- Volumetric scans from the white paper: ImageDepth tag 32997, tile tables
  holding the nominal plane, then the near planes, then the far planes,
  `Z-layers` (odd) and `Z-spacing` in the iScan node. The order within the
  near and far blocks is not documented; outward from the nominal plane is
  assumed. No public volumetric BIF exists; on the synthetic
  `volumetric.bif` Bio-Formats (black box) returns the same three planes in
  the same order (near, nominal, far).
- iScan slides (OpenSlide's OS-2 sample, iScan software 3.3, no `Frame`
  nodes, several AOIs, overlaps in both directions): the placement was
  derived by locating stored tiles in renderings of OpenSlide 3.4.1 run as a
  black box: one lattice per scanned AOI with the tile size less the mean
  overlap of all joined `TileJointInfo` entries as pitch (fitted pitch
  911.11 px against 911.11 from the mean, residual under 1 px), AOI
  positions from `Pos-X`/`Pos-Y` with stage y pointing up, the stored grid
  position from `AoiOrigin`, and later tiles (rows from the top, columns
  from the left) on top. Bio-Formats stitches the same file to a different
  size (117390 x 76317 against OpenSlide's 114943 x 76349); OpenSlide is the
  reference. Our canvas is 1 px narrower than OpenSlide's.
- Synthetic samples written from that model by
  `tests/scripts/make_bif_samples.py`.

## Imaris IMS (`src/formats/ims.rs`, `src/formats/hdf5.rs`)

- HDF5 File Format Specification version 3 (The HDF Group) in `doc/hdf5/`:
  the read-only subset in `hdf5.rs` (superblocks, object headers,
  groups, attributes, layouts, version 1/2 B-trees, fractal heaps, filter
  pipeline). Own implementation instead of a crate: the pure-Rust readers
  checked (hdf5-reader, hdf5-pure, rust-hdf5) bring their own I/O, caches
  and array types (ndarray), or do not expose chunk addresses for
  positional, cache-free reads; libhdf5 bindings need C.
- Imaris file format description (Bitplane) and the ImarisWriter paper in
  `doc/bitplane-imaris-ims/`; OpenReadout IMS notes (MIT OR Apache-2.0):
  group layout, `|S1` attributes, HDF5 LZ4 filter framing (32004).
- Own analysis of sample files (OME sample server, CC BY 4.0).
- Synthetic sample written by `tests/scripts/make_ims_samples.py` (its own
  minimal HDF5 writer from the specification).

## PNG, JPEG, JPEG 2000 files (`src/formats/plain.rs`)

- PNG specification (W3C, 3rd edition): chunk layout, `pHYs`, bit depths
  and colour types; decoding by the `png` crate (MIT OR Apache-2.0).
- JFIF 1.02 (ITU-T T.871): APP0 density; decoding by `zune-jpeg`.
- JPEG 2000 Part 1 (ITU-T T.800): JP2 boxes (Annex I, `res `/`resc`/`resd`)
  and reduced-resolution image sizes (B.5); decoding by `hayro-jpeg2000`.
  A reduced level differs from the 2 x 2 mean of the level above by 0.93 on
  average (smooth test image), as a wavelet low-pass should.
- Checked against Bio-Formats and bioformats-rs as black boxes on samples
  written by ImageMagick (`tests/scripts/make_plain_samples.py`); both
  return 1- to 4-bit gray PNG values unscaled, which scopeio follows.

## Olympus FluoView OIB/OIF (`src/formats/oif.rs`, `src/formats/cfb.rs`)

- Microsoft [MS-CFB] (published open specification): the compound file
  container of OIB, read by our own `cfb.rs`.
- OpenReadout OIF notes (MIT OR Apache-2.0) in `doc/olympus-oif-oib/`:
  `OibInfo.txt` stream mapping, settings text (UTF-16LE, decimal commas),
  plane file names, `.pty` property files, main settings keys.
- `oiffile` (BSD-3-Clause, Christoph Gohlke; notice in
  THIRD_PARTY_NOTICES.md) read as documentation: container mapping and the
  plane name convention (`_C001Z002T003.tif`).
- Own analysis of the OME sample (`Olympus-FluoView/imagesc-71616`,
  CC BY 4.0); pixels identical to Bio-Formats and bioformats-rs (black
  boxes). Synthetic OIF/OIB written by `tests/scripts/make_oif_samples.py`
  with its own [MS-CFB] writer; bioformats-rs reads its planes in another
  order (see `tests/expected/oif_divergences.tsv`).

## Olympus OIR (`src/formats/oir.rs`)

- OpenReadout OIR notes (MIT OR Apache-2.0) in `doc/olympus-oir/`: header,
  block index, block kinds, chunk tags and names, continuation files, XML
  documents and the fields taken from them.
- `oirfile` (BSD-3-Clause, Christoph Gohlke; notice in
  THIRD_PARTY_NOTICES.md) read as documentation: chunk name pattern,
  channel order by `@order`, frame positions.
- Own analysis of the OME samples (`Olympus-OIR`, CC BY 4.0): files without
  `pixelUnit` give lengths in micrometres. Pixels identical to Bio-Formats
  (black box) on all eleven image samples; bioformats-rs adds empty channels
  (see `tests/expected/oir_divergences.tsv`).
- Synthetic sample with a continuation file written by
  `tests/scripts/make_oir_samples.py`.

## DICOM (`src/formats/dicom.rs`, `src/formats/dicom/elements.rs`)

- DICOM PS3.3 (C.7.6 image modules, C.7.6.17.3 TILED_FULL frame order,
  C.8.12.4 whole slide microscopy), PS3.5 (encoding, A.4 encapsulation,
  Annex G RLE) and PS3.10 (Part 10 files) in `doc/dicom/` and
  dicom.nema.org: our own element parser.
- Checked against Bio-Formats and bioformats-rs (black boxes) on the
  OpenSlide DICOM test slides (CC0: CMU-1-JP2K-33005, Leica-4, Hamamatsu-2,
  3DHISTECH-1): level sizes identical, pixels within JPEG/JPEG 2000 decoder
  rounding, associated images identical. On a synthetic fluorescence image
  Bio-Formats maps optical paths by their identifier value instead of the
  sequence order (see `tests/expected/dicom_divergences.tsv`).
- Synthetic samples written by `tests/scripts/make_dicom_samples.py`.

## Zeiss AxioVision ZVI (`src/formats/zvi.rs`)

- OpenReadout ZVI notes (MIT OR Apache-2.0) in `doc/zeiss-zvi/`: streams,
  typed values, tag lists, raw-image header, tag ids.
- Microsoft [MS-CFB] (container, `cfb.rs`) and [MS-OAUT] (`VARENUM` type
  codes of the typed values).
- Public sample files from Figshare (CC BY 4.0, one CC0; fetched by
  `tests/scripts/fetch_figshare.py`): pixels identical to Bio-Formats
  (black box) on all eleven files it opens; `a1.zvi`, which neither
  Bio-Formats nor bioformats-rs opens, is read. Bio-Formats reports 1 um
  for uncalibrated files where scopeio reports no pixel size.
- Synthetic samples written by `tests/scripts/make_zvi_samples.py` (its
  compound-file writer `cfbwriter.py` orders directory siblings as
  [MS-CFB] 2.6.4 requires).

## MetaMorph STK and ND (`src/formats/tiff/stk.rs`, `src/formats/nd.rs`)

- Molecular Devices "MetaMorph STK and TIFF file formats" (T10243,
  `doc/metamorph-stk/`): UIC1 and UIC2 tags, planes stored after the
  first one.
- tifffile (BSD-3-Clause, Christoph Gohlke; notice in
  THIRD_PARTY_NOTICES.md) read as documentation of the UIC tag layouts.
- ND files: own analysis of the OME `Metamorph/zenodo-13642395` acquisition
  (CC0): keys and data file names.
- Public STK files from Figshare (CC BY 4.0 / CC0, fetched by
  `tests/scripts/fetch_figshare.py`) and the ND acquisition: pixels
  identical to Bio-Formats and bioformats-rs (black boxes). Bio-Formats
  reports 1 um for a stack calibrated in "pixel"; scopeio reports none.
- Synthetic samples written by `tests/scripts/make_stk_samples.py`.

## DeltaVision (`src/formats/dv.rs`)

- Priism IVE image file format reference (UCSF; archived copy in
  `doc/deltavision-dv/`): header fields, pixel modes, section orders,
  extended header.
- OpenReadout DeltaVision notes (MIT OR Apache-2.0, `doc/deltavision-dv/`).
- Own analysis of the OME `DV/siRNAi-HeLa` samples (CC BY 4.0): byte order
  from the identifier, rows stored bottom-up. Pixels identical to
  Bio-Formats and bioformats-rs (black boxes) on all five files.
- Synthetic samples written by `tests/scripts/make_dv_samples.py`.

## PerkinElmer Operetta / Opera Phenix (`src/formats/operetta.rs`)

- Own analysis of the public Harmony exports on the OME sample server
  (`PerkinElmer-Operetta/zenodo-7841360` and `omer`, CC BY 4.0; `59548`,
  whose folder states no licence, is only downloaded, never committed):
  `Index.idx.xml` / `Index.ref.xml` elements, plane files located next to
  the index or in the `<guid>/` folder their URL names. Pixels identical
  to Bio-Formats (black box, one series per run) on zenodo-7841360 and all
  29 omer series, and to bioformats-rs where it reads them. On 59548 neither
  reads the planes (Bio-Formats returns zeros, bioformats-rs rejects the
  index); scopeio's planes there equal the plane files they name.
- Synthetic export written by `tests/scripts/make_operetta_samples.py`.

## MRC / CCP4 (`src/formats/mrc.rs`)

- MRC2014 specification (Cheng et al., "MRC2014: Extensions to the MRC
  format header for electron cryo-microscopy and tomography", J. Struct.
  Biol. 192 (2015)) and the CCP-EM MRC format page: header fields, pixel
  modes 0-2, 6 and 12, machine stamp, `MAP ` marker, space group 401 for
  volume stacks, extended header length `nsymbt`.
- IMOD MRC header documentation (prose): IMOD stamp and signed-byte flag,
  mode 101 (4-bit samples, low nibble first, rows padded to whole bytes).
- Own analysis of the EMDB map EMD-2225 (OME sample server; EMDB data are
  free of copyright restrictions): rows stored bottom-up. Pixels identical
  to Bio-Formats and bioformats-rs (black boxes). Both read mode 12 and
  mode 101 as bytes, mode 0 of MRC2014 files as unsigned and ignore volume
  stacks (space group 401); scopeio follows the specification (see
  `tests/expected/mrc_divergences.tsv`).
- Synthetic samples written by `tests/scripts/make_mrc_samples.py`.

## Olympus SIS TIFF (`src/formats/tiff/sis.rs`)

- tifffile (BSD-3-Clause, Christoph Gohlke; notice in
  THIRD_PARTY_NOTICES.md) read as documentation of the OlympusSIS tag
  (33560) and its `SIS0` record: acquisition time, name, sub-record 1 with
  pixel size (times 10^exponent metres), magnification, camera, picture
  type.
- Own analysis of public analySIS 5.0 TEM images (Figshare 19467233,
  19467287, 19640952; CC BY 4.0, fetched by `tests/scripts/fetch_figshare.py`):
  gray images stored as palette images whose map is a gray ramp with
  overlay colours (entries 1-6, 249-254); the indices are intensities.
  Pixels and pixel sizes identical to Bio-Formats (black box), pixels to
  bioformats-rs.
- Synthetic sample written by `tests/scripts/make_sis_samples.py`.

## Olympus FluoView TIFF (`src/formats/tiff/fluoview.rs`)

- tifffile (BSD-3-Clause, Christoph Gohlke; notice in
  THIRD_PARTY_NOTICES.md) read as documentation of MM_Header (tag 34361):
  record layout, ten dimensions (name, size, origin, resolution, unit)
  and their names (`Z`, `T`/`Time`, `Ch`/`Wavelength`).
- No public FluoView TIFF was found. Synthetic files written by
  `tests/scripts/make_fluoview_samples.py` were read by Bio-Formats as a
  black box: it lists the dimensions after X and Y fastest first (same
  planes, sizes and pixel sizes as scopeio). It needs MM_Stamp (34362) as
  an integer-typed tag of more than 8 doubles. bioformats-rs ignores
  MM_Header (see `tests/expected/fluoview_divergences.tsv`).

## NIfTI (`src/formats/nifti.rs`)

- NIfTI-1 data format specification `nifti1.h` and the NIfTI-2 header
  definition `nifti2.h` (NIfTI Data Format Working Group, public domain) in
  `doc/nifti/`: header fields of both versions, data types, `xyzt_units`,
  `vox_offset`, magic strings of single files and header/image pairs,
  storage order (`dim[1]` fastest, `dim[7]` slowest).
- Official NIfTI test data (public domain; OME sample server `NIfTI/NIH`):
  pixels identical to Bio-Formats and bioformats-rs (black boxes) on all
  twelve `.nii`, `.hdr` + `.img` and `.nii.gz` files; voxels returned as
  stored (no row flip). Bio-Formats assumes micrometres when the header
  gives no spatial unit, reads 5-D files channel-fastest, reports RGB24 as
  int8 and does not read NIfTI-2 (see `tests/expected/nifti_divergences.tsv`).
- Synthetic samples written by `tests/scripts/make_nifti_samples.py`.
- Analyze 7.5 (same module): Mayo Clinic "Analyze 7.5 File Format" document
  and G. Wideman's field compilation (`doc/analyze/`): `sizeof_hdr`,
  `dim`, `vox_units`, `datatype`, `pixdim`, `vox_offset`, `funused1` (SPM
  scale), voxel order as NIfTI-1. No public image pair with a clear licence
  was found; synthetic pairs written by
  `tests/scripts/make_analyze_samples.py`, checked against Bio-Formats and
  bioformats-rs as black boxes (see `tests/expected/analyze_divergences.tsv`).

## Olympus scan^R (`src/formats/scanr.rs`)

- Own analysis of the public scan^R plate of IDR study idr0009
  (`ScanR/idr0009/0307-10--2007-05-30` on the OME sample server, CC BY
  4.0, fetched by `tests/scripts/fetch_ome.py`): `experiment_descriptor.xml`
  is LabVIEW XML (`LVData`, typed `Name`/`Val` elements in `Cluster` and
  `Array`) with the channel configurations, plate name and well selection
  table; plane files `--W<well>--P<position>--Z<z>--T<t>--<channel>.tif`
  in `data/`.
- Checked against Bio-Formats as a black box: the plate is the standard
  layout for the number of wells in the selection table (384: 16 x 24, not
  the descriptor's custom 32 x 12), wells numbered from 1 row by row (all
  357 imaged wells), one series per imaged well and position; pixels
  identical in all 357 series. On a synthetic plate Bio-Formats ignores the
  T index and keeps 12 bits per sample (see
  `tests/expected/scanr_divergences.tsv`).
- Synthetic plate written by `tests/scripts/make_scanr_samples.py`.

## Nikon NIS-Elements TIFF (`src/formats/tiff/nikon.rs`)

- Own analysis of public NIS-Elements TIFF exports (Zenodo 7677868,
  7677827, 8006449, 8245566; CC BY 4.0, fetched by
  `tests/scripts/fetch_zenodo.py`): private tags 65325-65333; tag 65331 is
  a directory of named entries whose `MetadataTiffV1_0` entry holds
  `SLxPictureMetadata` in the ND2 lite-variant encoding (decoded by
  `src/formats/nd2/meta.rs`, see the ND2 section).
- Checked against Bio-Formats and bioformats-rs as black boxes: pixels
  identical on all files; Bio-Formats identifies the files as Nikon
  Elements TIFF but reports no pixel size, objective or stage position,
  which scopeio takes from the picture metadata.
- Synthetic sample written by `tests/scripts/make_nikon_tiff_samples.py`.

## Nikon NEF (`src/formats/tiff/nef.rs`)

- TIFF 6.0 and TIFF/EP: CFA photometric (32803), CFARepeatPatternDim
  (33421), CFAPattern (33422); tifffile (BSD-3-Clause) for the code of
  Nikon's compression (34713), which is not implemented: every permissive
  route was checked (rawloader, rawler, quickraw: LGPL-2.1; LibRaw: LGPL /
  CDDL; dcraw: unclear licence) and none was used.
- Own analysis of public Nikon D300 samples (raw.pixls.us, CC0, fetched by
  `tests/scripts/fetch_rawpixls.py`): IFD0 thumbnail, SubIFDs with JPEG
  preview and raw image, 12-bit samples packed MSB first. The raw values
  are returned as one channel; per-colour 8 x 8 cell means agree with the
  demosaiced channels of Bio-Formats (black box) within 1 % on average.
- Synthetic sample written by `tests/scripts/make_nef_samples.py`.

## Olympus OMP2INFO (`src/formats/omp2info.rs`)

- Own analysis of the public multi-area acquisition
  `Olympus-OIR/gh-4205/zenodo-13680725` (OME sample server, CC BY 4.0):
  `matl:properties` XML (stage overlap, `matl:areaInfo`, `matl:area` with
  OIR file and grid indices, `matl:stitchPOCParam`), vendor stitch
  `Stitch_A01_G001.oir`. The tile OIRs, the map OIR and HDF5 and the
  stitched OIR hold no per-tile placement, so the matched positions of the
  vendor stitch cannot be reconstructed; the vendor stitch is returned as
  it is and the tiles with nominal positions.
- Bio-Formats run as a black box: it builds one mosaic whose tile pitch
  depends on the declared tile count (466 x 462 px here), not used.
- Synthetic acquisition written by `tests/scripts/make_omp2info_samples.py`
  (copies of the synthetic OIR).

## Micro-Manager (`src/formats/micromanager.rs`)

- Micro-Manager file format page (micro-manager.org, `doc/micro-manager/`):
  image file stack header (index map offset, display settings, comments
  and summary headers), IFD layout with the MicroManagerMetadata tag
  51123, index map (channel, slice, frame, position, IFD offset), separate
  image files with `metadata.txt`.
- NDTiff specification v1 to v3 (README of micro-manager/NDStorage,
  BSD-3-Clause, `doc/micro-manager/`): file header, `NDTiff.index` entries
  (axes JSON, file name, pixel offset, size, pixel type, compression,
  metadata offset and length), `Full resolution` folder of v2.
- Own analysis of the public Micro-Manager samples on the OME sample server
  (`Micro-Manager/`, CC BY 4.0: 1.4.16 separate files, 1.4.22 stacks and
  separate files in 1.4 and 2.0-beta layout, 1.4.22 two-file stack, 1.4.23
  aborted stack) and the NDTiff test datasets (Zenodo 7065552, CC BY 4.0):
  `FrameKey-<t>-<c>-<z>` and `Coords-<file>` / `Metadata-<file>` entries,
  summary keys (`ChNames`, `ChColors` as Java ARGB, `PixelSize_um`,
  `z-step_um`, `Interval_ms`, `StartTime` / `Time`,
  `InitialPositionList` / `StagePositions`), image metadata keys
  (`ElapsedTime-ms`, `Exposure-ms`, `X/Y/ZPositionUm`, `PixelSizeUm`),
  NDTiff string and negative axis values.
- Checked against Bio-Formats and bioformats-rs as black boxes (see
  `tests/expected/micromanager_divergences.tsv`).
- Synthetic datasets written by `tests/scripts/make_micromanager_samples.py`.

## BigDataViewer XML/HDF5 (`src/formats/bdv.rs`)

- BigDataViewer documentation (ImageJ wiki) and Pietzsch et al., "BigDataViewer:
  visualization and processing for large image data sets", Nat. Methods
  2015 (arXiv 1412.0488), in `doc/bdv/`: setups, time points, views,
  multi-resolution pyramids of chunked volumes, `.xml` + `.h5` pairs.
- HDF5 File Format Specification version 3 (`doc/hdf5/`), shared HDF5
  reader with the IMS reader.
- HDF5 scale-offset filter (id 6, integer mode): the algorithm as
  implemented in `H5Zscaleoffset.c` of the HDF5 library (BSD-3-Clause, read
  as documentation; notice in THIRD_PARTY_NOTICES.md): 21-byte header with
  minimum bits and minimum value, values packed most significant bit
  first, all-ones code as fill value, full precision stored unpacked.
- Own analysis of the public BDV samples on the OME sample server
  (`BDV/samples`, public domain; `BDV/idr0038`, `BDV/idr0048` XML files,
  CC BY 4.0): `t<tp:05>/s<setup:02>/<level>/cells` (z, y, x) int16 volumes
  holding unsigned values, `s<setup:02>/resolutions` and `subdivisions`,
  `ViewSetup` attributes and `Attributes` entity names, `MissingViews`.
- Checked against Bio-Formats and bioformats-rs as black boxes (see
  `tests/expected/bdv_divergences.tsv`).
- Synthetic dataset written by `tests/scripts/make_bdv_samples.py`.

## Hamamatsu VMS (`src/formats/vms.rs`, `src/formats/slide/restart.rs`)

- openslide.org Hamamatsu format page (prose, CC BY-SA 4.0,
  `doc/hamamatsu-ndpi-vms/`): VMS index keys (`NoJpegColumns`,
  `NoJpegRows`, `ImageFile(z,x,y)`, `MapFile`, `MacroImage`,
  `PhysicalWidth`/`Height` in nm, `SourceLens`), JPEG files with restart
  markers, incomplete optimisation file. OpenSlide's C code (LGPL) was not
  consulted.
- ITU-T T.81 (JPEG): restart markers and intervals, frame header.
- Own analysis of the OpenSlide VMS test slide CMU-1 (CC0): grid of four
  JPEG files of 4096 x 8 px restart intervals, map at 1/8 scale.
- Checked against Bio-Formats as a black box on three crops (both file
  boundaries), the macro and the map (see
  `tests/expected/vms_divergences.tsv`).
- Synthetic slide written by `tests/scripts/make_vms_samples.py`.

## OME-XML (`src/formats/tiff/omexml.rs`)

- OME 2016-06 schema (`ome_2016-06.xsd`: `BinData` with `Compression`,
  `BigEndian`, `Length`; `MetadataOnly`) and the OME data model pages
  (`doc/ome-xml/`); metadata shared with the OME-TIFF reader.
- OME-XML sample files of the OME sample server (`OME-XML/2016-06`,
  `OME-XML/2008-09`, written by the OME project): one `BinData` per plane
  in `DimensionOrder` order, schema 2008-09 `PixelType` and
  `DefaultPixels`.
- Checked against Bio-Formats and bioformats-rs as black boxes (see
  `tests/expected/omexml_divergences.tsv`).
- Synthetic files written by `tests/scripts/make_omexml_samples.py`.

## Zeiss Xradia TXM / TXRM / XRM (`src/formats/xrm.rs`)

- Library of Congress format description fdd000581 (`doc/zeiss-xrm-txrm/`):
  OLE2 compound file; [MS-CFB] for the container (shared `cfb.rs`).
- `dxchange` (BSD-3-Clause, UChicago Argonne; read as documentation,
  notice in THIRD_PARTY_NOTICES.md): `ImageInfo` stream names, data types
  5 (uint16) and 10 (float32), `ImageData<ceil(n/100)>/Image<n>`.
- Own analysis of the Xradia Versa 520 reconstruction of Zenodo record
  4432446 (CC BY 4.0): `PixelSize` in micrometres, 999 slices, per-image
  arrays of 999 floats.
- Checked against Bio-Formats and bioformats-rs as black boxes (see
  `tests/expected/xrm_divergences.tsv`).
- Synthetic files written by `tests/scripts/make_xrm_samples.py`.

## Blosc (`src/codecs/blosc.rs`)

- c-blosc1 `README_CHUNK_FORMAT.rst` (`doc/codecs/blosc/`). Observed on
  public data: block start offsets count from the start of the chunk
  (the document says from the end of the offset table).

## Codecs (`src/codecs*`)

Implemented by permissively licensed crates (see THIRD_PARTY_NOTICES.md);
the wrappers follow the specifications in `doc/codecs/`:

| Codec | Crate | Licence | Choice |
|---|---|---|---|
| Deflate / zlib | flate2 + zlib-rs | MIT/Apache-2.0, Zlib | fastest pure-Rust inflate |
| LZW | weezl | MIT/Apache-2.0 | TIFF code-size switch built in |
| PackBits | own (Apple TN1023) | - | a few lines |
| JPEG (8-bit baseline, progressive) | zune-jpeg | MIT/Apache-2.0/Zlib | decodes into caller buffers, SIMD |
| Zstandard | ruzstd (default), zstd + libzstd (`zstd-c`) | MIT; BSD-3-Clause | see below |
| LZ4 (Blosc, numcodecs) | lz4_flex | MIT | fastest Rust LZ4 |
| Blosc 1 | own (chunk format document) | - | no mature pure-Rust crate; blusc depends on C zstd |

Zstandard benchmark (2026-10-24, Intel Core Ultra 9 185H, one thread):
reading all planes of idr0062A/6001240 v0.5 (sharded, Blosc/zstd, 70 MB
decoded) takes 1.40 s CPU with ruzstd and 0.21 s with libzstd; output is
identical. ruzstd stays the default to keep the default build pure Rust;
`--features zstd-c` selects libzstd.

| JPEG 2000 | hayro-jpeg2000 | Apache-2.0 OR MIT | see below |

JPEG 2000 benchmark (2026-10-24, one thread, 64 tiles of 512 x 512 16-bit
from a VS200 slide): hayro-jpeg2000 32.6 Mpx/s, dicom-toolkit-jpeg2000 (a
fork of hayro adding native-depth output) 20.6 Mpx/s. hayro's samples agree
with OpenJPEG (via ImageMagick, black box) on 99 % of pixels, the rest by
rounding; dicom-toolkit differs more. Block means of decoded tiles agree with
Bio-Formats within 0.05 of 65535. openjp2 (BSD-2, Rust port of OpenJPEG) was
not benchmarked.

| JPEG XR | openreadout-jpegxr | BSD-2-Clause (port of Microsoft jxrlib) | see below |

JPEG XR benchmark (2026-10-24, one thread, 40 subblocks of 1504 x 2048
16-bit gray from `2023_11_30__RecognizedCode-27.czi`): openreadout-jpegxr
48.5 Mpx/s, jxr-native (MIT/Apache-2.0) 33.2 Mpx/s with about three times
the CPU time (rayon). Output of both is bit-identical on every subblock.
openreadout-jpegxr is `forbid(unsafe_code)` and has no dependencies.
jpegxr-pure-rs (BSD-3) was not considered: 693 `unsafe` blocks; jpegxr
(BSD-3) binds the C jxrlib.

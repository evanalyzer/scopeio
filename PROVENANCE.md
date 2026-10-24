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

## Imaris IMS (`src/formats/ims.rs`, `src/formats/ims/hdf5.rs`)

- HDF5 File Format Specification version 3 (The HDF Group) in `doc/hdf5/`:
  the read-only subset in `ims/hdf5.rs` (superblocks, object headers,
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

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
not benchmarked. JPEG-XR is not supported yet (needed by CZI).

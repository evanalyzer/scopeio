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

## Codecs (`src/codecs*`)

Implemented by permissively licensed crates (see THIRD_PARTY_NOTICES.md);
the wrappers follow the specifications in `doc/codecs/`:

| Codec | Crate | Licence | Choice |
|---|---|---|---|
| Deflate / zlib | flate2 + zlib-rs | MIT/Apache-2.0, Zlib | fastest pure-Rust inflate |
| LZW | weezl | MIT/Apache-2.0 | TIFF code-size switch built in |
| PackBits | own (Apple TN1023) | - | a few lines |
| JPEG (8-bit baseline, progressive) | zune-jpeg | MIT/Apache-2.0/Zlib | decodes into caller buffers, SIMD |
| Zstandard | ruzstd | MIT | pure Rust |

JPEG 2000 and JPEG-XR are not supported yet; options are evaluated with
the formats that need them (VSI, CZI, SVS).

# Changelog

All notable changes to scopeio. Earlier history (TIFF core, OME-Zarr,
whole-slide, vendor and HCS formats up to Zeiss Xradia XRM) is in the git
log.

## Unreleased

### Added

- Hamamatsu DCIMG reader (`dcimg`): version 7 and 0x1000000 files, frames
  as time points read straight from the file, frame time stamps, camera
  model / serial / sub-array, the ORCA-Flash4.0 stored pixels restored per
  frame.
- AmiraMesh / Avizo reader (`amira`): 2D / 3D lattice fields, BINARY
  (both byte orders) and ASCII data, HxZip and HxByteRLE, all integer and
  float types, vector components and label fields as channels, pixel size
  in the file's unit.
- AVI reader (`avi` feature): uncompressed (8-bit palette, 16/24/32-bit,
  `Y800` gray), MSRLE (RLE8 / RLE4 with delta frames) and Motion JPEG
  (also without Huffman tables) video streams; OpenDML files above 1 GB
  (`indx` / `ix##`, `RIFF AVIX`); `idx1` with relative or absolute
  offsets, or a scan of `movi`; dropped frames. Motion JPEG is lossy and
  not suited to quantitative intensity analysis. Inter-frame codecs are
  rejected with a hint to convert the file.
- Fuzz targets (`fuzz/`, cargo-fuzz) for the TIFF core, OME-TIFF, NDPI,
  ND2, CZI, VSI, LIF, Imaris / HDF5, DICOM, compound files (ZVI, OIB,
  Xradia), AVI and OME-Zarr, seeded from `tests/data` (`fuzz/seed.py`).
- Corrupt-input tests for TIFF, OME-TIFF, ND2, VSI, Operetta and OME-Zarr
  (multi-file datasets through `corrupt_tree_sweep`).
- CI (`.github/workflows/ci.yml`): format, clippy, tests, every feature on
  its own (cargo-hack), cargo-deny, a one-minute run of each fuzz target and
  line coverage.
- `extract_region` example: one region as an 8-bit PGM / PPM.
- Crate metadata (description, repository, keywords); the package ships the
  library, examples and licence files only.
- Keller Lab Block reader (`klb`): bzip2 / uncompressed blocks, all
  channels and time points of a file.
- NRRD reader (`nrrd`): attached and detached headers; raw, gzip, ASCII
  and hex encodings; axis roles from `kinds`.
- MetaXpress HTD reader (`metaxpress`), Yokogawa CV7000 / CV8000 reader
  (`cv7000`), Opera Flex (`flex`), Gatan DigitalMicrograph DM3 / DM4
  (`gatan`), Becker & Hickl SDT (`sdt`), IN Cell Analyzer (`incell`),
  Imspector OBF (`obf`) and ICS (`ics`) readers.

### Changed

- Performance: SDT ZIP blocks are inflated per requested region and kept
  plane by plane (64 x 64 crop of a 2 GB block: 0.10 s / 49 MiB instead of
  0.99 s / 2 GiB; a full read 3.6 s instead of 17 s); compressed HDF5
  chunks of several z planes are cached when a plane's chunks of all
  channels fit in 64 MiB (BigDataViewer HisYFP-SPIM 10.6 s instead of
  78 s), uncompressed chunks are read one plane at a time (an Imaris file
  of 47 planes: 0.017 s instead of 0.042 s); short uncompressed rows (under
  8 KiB) of TIFF, LIF, ICS / NRRD / AmiraMesh and contiguous HDF5 data are
  read in 256 KiB bands instead of one system call per row (414 MB OME-TIFF:
  0.045 s instead of 0.078 s CPU; a 227 MB AmiraMesh tomogram 0.027 s instead
  of 0.075 s).

- The BMP reader's DIB header, colour table, bit mask and RLE code moved
  to a module shared with the AVI reader.
- One-plane MetaMorph TIFF files get their UIC1 calibration (pixel size).
- ICS samples are read by a strided reader shared with NRRD.
- `rustfmt.toml` (max width 150); the crate passes `cargo fmt --check` and
  `cargo clippy --all-features -- -D warnings`.

### Fixed

Found by fuzzing; each returned an error before or now no longer panics,
aborts or allocates without bound:

- TIFF: `SamplesPerPixel` or `BitsPerSample` above 65535 wrapped to 0 and
  panicked (found by CI fuzzing in a SubIFD pyramid level); panic on an
  empty `BitsPerSample`; out-of-bounds copy when a level
  or plane IFD has a different pixel layout than its series (also NDPI,
  SVS and the other TIFF-based readers).
- TIFF: Deflate, LZW and PackBits chunks too small to decode to the rows
  requested fail before their buffer is allocated (a 428-byte file claiming
  a 13-megapixel-wide strip took 2.4 s per read).
- AVI: frames above 512 MiB (was 2 GiB) are rejected; RLE frames expand
  their palette without an allocation per pixel (a 1.5 KB file claiming an
  8 x 50-megapixel RLE frame took 9 s).
- NDPI: panic on adjacent or unordered restart-interval offsets, and an
  unchecked restart count from the JPEG header.
- HDF5 (Imaris, BigDataViewer): panic when the element size differs from
  the series pixel type; unbounded LZ4 block buffer; panic on a short last
  LZ4 block.
- OME-Zarr: division by zero for a shard shape of 0.
- DICOM: division by zero for an empty total matrix.
- CZI: overflow and multi-gigabyte allocations from corrupt C / Z / T
  coordinates.
- ND2: allocations from corrupt component counts and experiment loops.
- Micro-Manager: plane tables of corrupt index coordinates far beyond the
  stored images.

### Security

- Scratch buffers are capped at 2 GiB: a read that would need more (sizes
  from a corrupt header) fails with `Error::Corrupt` instead of exhausting
  memory.

- zstd 0.13.4 / zstd-safe 7.4.0 (RUSTSEC-2026-0391 and the zstd-safe
  buffer advisory) for the optional `zstd-c` feature.

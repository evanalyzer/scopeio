# Changelog

All notable changes to scopeio. Earlier history (TIFF core, OME-Zarr,
whole-slide, vendor and HCS formats up to Zeiss Xradia XRM) is in the git
log.

## Unreleased

### Added

- AVI reader (`avi` feature): uncompressed (8-bit palette, 16/24/32-bit,
  `Y800` gray), MSRLE (RLE8 / RLE4 with delta frames) and Motion JPEG
  (also without Huffman tables) video streams; OpenDML files above 1 GB
  (`indx` / `ix##`, `RIFF AVIX`); `idx1` with relative or absolute
  offsets, or a scan of `movi`; dropped frames. Motion JPEG is lossy and
  not suited to quantitative intensity analysis. Inter-frame codecs are
  rejected with a hint to convert the file.
- Fuzz target for the AVI reader (`fuzz/`, cargo-fuzz).
- Keller Lab Block reader (`klb`): bzip2 / uncompressed blocks, all
  channels and time points of a file.
- NRRD reader (`nrrd`): attached and detached headers; raw, gzip, ASCII
  and hex encodings; axis roles from `kinds`.
- MetaXpress HTD reader (`metaxpress`), Yokogawa CV7000 / CV8000 reader
  (`cv7000`), Opera Flex (`flex`), Gatan DigitalMicrograph DM3 / DM4
  (`gatan`), Becker & Hickl SDT (`sdt`), IN Cell Analyzer (`incell`),
  Imspector OBF (`obf`) and ICS (`ics`) readers.

### Changed

- The BMP reader's DIB header, colour table, bit mask and RLE code moved
  to a module shared with the AVI reader.
- One-plane MetaMorph TIFF files get their UIC1 calibration (pixel size).
- ICS samples are read by a strided reader shared with NRRD.
- `rustfmt.toml` (max width 150); the crate passes `cargo fmt --check` and
  `cargo clippy --all-features -- -D warnings`.

### Security

- zstd 0.13.4 / zstd-safe 7.4.0 (RUSTSEC-2026-0391 and the zstd-safe
  buffer advisory) for the optional `zstd-c` feature.

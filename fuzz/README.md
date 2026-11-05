# Fuzz targets

One [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) target per parser
family. Each writes the input as a file (OME-Zarr: a folder tree), opens it
with `scopeio::open` and reads metadata, original metadata, small regions of
every level and plane corner, and the associated images
(`src/lib.rs`). Errors are expected; panics, aborts, timeouts and
allocations above the limits are findings.

| Target | Covers |
|---|---|
| `tiff` | TIFF / BigTIFF core, also SVS, QPTIFF, SCN, BIF, LSM, STK |
| `ome_tiff` | OME-TIFF (TIFF core plus OME-XML), Micro-Manager stacks |
| `ndpi` | Hamamatsu NDPI, JPEG restart intervals |
| `nd2` | Nikon ND2 |
| `czi` | Zeiss CZI |
| `vsi` | Olympus VSI (without ETS companions) |
| `lif` | Leica LIF |
| `ims` | Imaris IMS (HDF5) |
| `dicom` | DICOM |
| `zvi` | Compound files: Zeiss ZVI, OIB, Xradia |
| `avi` | AVI |
| `amira` | AmiraMesh / Avizo |
| `dcimg` | Hamamatsu DCIMG |
| `zarr` | OME-Zarr v2 / v3 |

The `zarr` input is a sequence of entries `relative path, 0x00, u32 LE
length, bytes`.

## Running

Needs a nightly toolchain and `cargo install cargo-fuzz`.

```sh
python3 -I fuzz/seed.py            # corpus from tests/data
cd fuzz
cargo +nightly fuzz run -O tiff corpus/tiff -- \
    -rss_limit_mb=3072 -malloc_limit_mb=2200 -timeout=30 -max_len=1048576
```

Single reads may allocate up to 2 GiB (`scratch::LEN_MAX`), so the memory
limits are above that. CI runs every target for one minute and fails on
crashes; inputs that only exceed the time or memory limit are uploaded as
artifacts. Run the targets for hours locally before a release. Reproduce a finding with
`cargo +nightly fuzz run -O <target> artifacts/<target>/<file>`.

## Licence

This crate is separate from the library's dependency graph:
`libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA` and is a development
tool only, never shipped with scopeio.

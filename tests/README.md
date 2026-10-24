# Tests

`cargo test` needs nothing besides this repository. Reference results from
Bio-Formats and bioformats-rs are committed in `tests/expected/`; tests for
sample files that are not present locally are skipped.

## Layout

| Path | Committed | Content |
|---|---|---|
| `data/<format>/` | yes | Small synthetic samples written from the format specification, plus `truth.tsv` with the CRC-32 each file must decode to |
| `expected/<format>.tsv` | yes | Metadata and pixel CRC-32 from both reference readers |
| `expected/<format>_divergences.tsv` | yes | Known, explained differences from the reference readers |
| `expected/<format>_bench.md` | yes | CPU time and peak RAM of scopeio and both reference readers |
| `scripts/` | yes | Sample generators, `oracle.py`, `measure.rs` |
| `external/`, `reference/` | no | Larger real-world sample files |
| `tools/` | no | Pinned reference readers, see below |

## Reference readers

Both are used strictly as black boxes: only their command-line tools are run
as separate processes. They are never linked or distributed, and their source
code must not be read (see the licence notes in the project instructions).

Pinned versions, installed into `tests/tools/`:

```sh
# Bio-Formats command-line tools 8.5.0 (needs Java 11 or newer)
mkdir -p tests/tools && cd tests/tools
curl -LO https://downloads.openmicroscopy.org/bio-formats/8.5.0/artifacts/bftools.zip
unzip bftools.zip && rm bftools.zip
cd ../..

# bioformats-rs 0.1.8 (crate `bioformats`), only its CLI binary
cargo install bioformats --version 0.1.8 --locked --root tests/tools/bioformats-rs
```

## Downloaded samples

Larger public samples are not committed; scripts download them into
gitignored folders:

```sh
# OME-Zarr from IDR (CC BY 4.0): v0.4 image, v0.5 sharded image, v0.4 plate
python3 -I tests/scripts/fetch_ome_zarr.py https://uk1s3.embassy.ebi.ac.uk/idr/zarr/v0.4/idr0062A/6001240.zarr --levels 2
python3 -I tests/scripts/fetch_ome_zarr.py https://uk1s3.embassy.ebi.ac.uk/idr/zarr/v0.5/idr0062A/6001240_labels.zarr --levels 2
python3 -I tests/scripts/fetch_ome_zarr.py https://uk1s3.embassy.ebi.ac.uk/idr/zarr/v0.4/idr0001A/2551.zarr --levels 1 --wells 2
```

## Regenerating references

```sh
python3 -I tests/scripts/make_bmp_samples.py      # only when samples change
python3 -I tests/scripts/make_tiff_samples.py
python3 -I tests/scripts/make_zarr_samples.py
python3 -I tests/scripts/make_vsi_samples.py
python3 -I tests/scripts/make_nd2_samples.py
python3 -I tests/scripts/make_czi_samples.py
python3 -I tests/scripts/oracle.py bmp tests/data/bmp/*.bmp \
    "tests/reference/ome/Leica-XLEF/format test/format test BMP/"*.bmp
cargo test
```

Large or multi-series files are compared on parts: `--series N` converts one
Bio-Formats series (pyramid levels count as series there), `--crop x,y,w,h`
one region, `--ours-series S` (or `assoc:thumbnail`) names the scopeio
series compared with it, `--keep-series N` records one series of the
output, and `--append` adds the run to the existing references. VSI:

```sh
E=tests/external
python3 -I tests/scripts/oracle.py vsi $E/olympus_01.vsi --series 1 --ours-series 0
python3 -I tests/scripts/oracle.py vsi $E/histo_01.vsi --series 1 --ours-series 0 --append
python3 -I tests/scripts/oracle.py vsi $E/olympus_02.vsi --series 0 --ours-series 0 --crop 8192,8192,1024,1024 --append
python3 -I tests/scripts/oracle.py vsi $E/whole_slide_olympus_01.vsi --series 13 --ours-series 0 --crop 4096,12288,1024,1024 --append
for f in olympus_01 histo_01; do python3 -I tests/scripts/oracle.py vsi $E/$f.vsi --keep-series 0 --ours-series assoc:thumbnail --append; done
```

ND2:

```sh
find tests/reference/ome/ND2 -name "*.nd2" -size -120M -print0 | xargs -0 python3 -I tests/scripts/oracle.py nd2 tests/external/nicon_01.nd2
for f in nicon_02 nicon_03; do python3 -I tests/scripts/oracle.py nd2 --series 0 --crop 1000,1000,512,512 --append tests/external/$f.nd2; done
```

CZI (`--per-series`: bfconvert returns wrong pixels for later series when
converting all series of some multi-scene files in one run, so each series
is converted alone):

```sh
find tests/reference/zenodo-7015307 tests/reference/zenodo-8305531 -name "*.czi" -size -120M -print0 | sort -z | xargs -0 python3 -I tests/scripts/oracle.py czi --per-series
R=tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi
python3 -I tests/scripts/oracle.py czi --series 0 --crop 12288,8192,2048,2048 --append $R
python3 -I tests/scripts/oracle.py czi --series 0 --crop 3000,2000,2048,2048 --append tests/reference/zenodo-8305531/test_gray.czi
python3 -I tests/scripts/oracle.py czi --repeat 1 --series 6 --ours-series assoc:label --append $R
python3 -I tests/scripts/oracle.py czi --repeat 1 --series 7 --ours-series assoc:macro --append $R
```

Divergence files may name one reference series in an optional `series`
column. Options go before the file names. Every reader runs under `measure`, which
kills it once its process group exceeds `MEASURE_MAX_RSS_MIB` (default
8192) of resident memory. bioformats-rs otherwise grows to more than 50 GiB
on `Exception9_e3.nd2`.

`oracle.py` converts each file with both readers to ICS, records metadata and
a CRC-32 of the pixels, and measures CPU time and peak RAM of each reader
(median of `--repeat` runs, default 3). Bio-Formats is run with `-expand`, so
8-bit palette images come out as RGB. If a reference differs from scopeio, verify
scopeio against the format specification and the raw file bytes. If scopeio
is right, add the case to `<format>_divergences.tsv` with the reason.

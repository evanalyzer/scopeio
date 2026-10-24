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

```sh
# Zenodo records (CC BY 4.0) with LSM files
python3 -I tests/scripts/fetch_zenodo.py 14510432 10-01.lsm
python3 -I tests/scripts/fetch_zenodo.py 10046394 "Suppl Fig 1B.lsm" "Fig 4A.lsm"
python3 -I tests/scripts/fetch_zenodo.py 16916709 Lan_Larval_Chitin_DMSO_control_01.lsm
python3 -I tests/scripts/fetch_zenodo.py 13286257 "sofa eye_zebrafish_spectral and zstack_01.lsm"
# OME sample server (check each folder's readme/COPYING; QPTIFF: CC BY 4.0)
P=Vectra-QPTIFF/perkinelmer
python3 -I tests/scripts/fetch_ome.py $P/PKI_fields/HnE_3_1x1component_data.tif "$P/PKI_fields/LuCa-7color_[13860,52919]_1x1component_data.tif" $P/PKI_scans/HandEcompressed_Scan1.qptiff $P/PKI_scans/LuCa-7color_Scan1.qptiff
# OpenSlide test slides (CC0-1.0 or "distributable", SHA-256 checked); --large adds multi-GB NDPI files
python3 -I tests/scripts/fetch_openslide.py
```

## Regenerating references

```sh
python3 -I tests/scripts/make_bmp_samples.py      # only when samples change
python3 -I tests/scripts/make_plain_samples.py    # PNG, JPEG, JPEG 2000; needs ImageMagick
python3 -I tests/scripts/make_oif_samples.py      # Olympus OIF/OIB (own compound-file writer)
python3 -I tests/scripts/make_oir_samples.py      # Olympus OIR with a continuation file
python3 -I tests/scripts/make_dicom_samples.py    # DICOM WSI (native, RLE, sparse, fluorescence) and plain
python3 -I tests/scripts/make_tiff_samples.py
python3 -I tests/scripts/make_zarr_samples.py
python3 -I tests/scripts/make_vsi_samples.py
python3 -I tests/scripts/make_nd2_samples.py
python3 -I tests/scripts/make_czi_samples.py
python3 -I tests/scripts/make_lif_samples.py
python3 -I tests/scripts/make_svs_samples.py
python3 -I tests/scripts/make_ndpi_samples.py   # needs ImageMagick
python3 -I tests/scripts/make_mrxs_samples.py   # needs ImageMagick
python3 -I tests/scripts/make_lsm_samples.py
python3 -I tests/scripts/make_qptiff_samples.py
python3 -I tests/scripts/make_scn_samples.py
python3 -I tests/scripts/make_bif_samples.py
python3 -I tests/scripts/make_ims_samples.py
# PNG, JPEG, JPEG 2000: the committed samples, then larger generated images
# for the benchmark (tests/reference/plain/, not committed)
python3 -I tests/scripts/oracle.py plain tests/data/plain/*.png tests/data/plain/*.jpg \
    tests/data/plain/*.jp2 tests/data/plain/*.j2k
REP=3 sh tests/scripts/make_plain_bench.sh
# Olympus OIB: OME sample (CC BY 4.0) and the synthetic set
python3 -I tests/scripts/fetch_ome.py "Olympus-FluoView/imagesc-71616/20220824_4492_cord_dapi__iba568_60x.oib"
python3 -I tests/scripts/oracle.py oif --repeat 3 tests/reference/ome/Olympus-FluoView/imagesc-71616/*.oib
python3 -I tests/scripts/oracle.py oif --append tests/data/oif/synthetic.oib tests/data/oif/synthetic.oif
# Olympus OIR: OME samples (CC BY 4.0; the stitched one with its 0.8 GB continuation file)
O=Olympus-OIR; G=$O/gh-4205
python3 -I tests/scripts/fetch_ome.py "$O/etienne/amy slice z stack_0001.oir" "$O/etienne/coupe shg stack_0001.oir" \
    "$O/etienne/venus stack.oir" $O/imagesc-105684/1202-interval_10sec_sequence_frame.oir \
    "$O/imagesc-105684/1202-interval_30sec_sequence_frame_z stack.oir" $O/imagesc-105684/1202-interval_freeRun_sequence_frame.oir \
    $O/imagesc-105684/1202-interval_30sec_sequence_frame.oir $G/zenodo-12773657/DAPI-mCherry_3T_4Z_5Lambda-420-630-w10nm-s50nm.oir \
    $G/zenodo-12773657/DAPI_mCherry_22Lambda-420-630-w10nm-s10nm.oir $G/zenodo-12773657/DAPI_mCherry_4T_5Lambda-420-630-w10nm-s50nm.oir \
    $G/zenodo-12773657/DAPI_mCherry_4Z_5Lambda-420-630-w10nm-s50nm.oir $G/zenodo-13680725/Map_A01.oir \
    $G/zenodo-13680725/tiling-sample-brain-section_A01_G001_0001.oir \
    $G/zenodo-13680725/Stitch_A01_G001.oir $G/zenodo-13680725/Stitch_A01_G001_00001
find tests/reference/ome/Olympus-OIR -name "*.oir" ! -name "*0002.oir" ! -name "*0003.oir" ! -name "Stitch*" -print0 | \
    sort -z | xargs -0 python3 -I tests/scripts/oracle.py oir --repeat 3
python3 -I tests/scripts/oracle.py oir --repeat 1 --series 0 --crop 1500,1000,512,512 --append \
    tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Stitch_A01_G001.oir
python3 -I tests/scripts/oracle.py oir --append tests/data/oir/synthetic.oir
# DICOM: OpenSlide test slides (fetch_openslide.py) and the synthetic files
D=tests/reference/openslide/DICOM; O="python3 -I tests/scripts/oracle.py dicom --append"
python3 -I tests/scripts/oracle.py dicom --series 0 --crop 20000,15000,2048,2048 $D/CMU-1-JP2K-33005/DCM_0.dcm
$O --series 1 --ours-series 0/1 --crop 4000,3000,1024,1024 $D/CMU-1-JP2K-33005/DCM_0.dcm
$O --repeat 1 --series 2 --ours-series 0/2 $D/CMU-1-JP2K-33005/DCM_0.dcm
$O --repeat 1 --series 3 --ours-series assoc:thumbnail $D/CMU-1-JP2K-33005/DCM_0.dcm
$O --repeat 1 --series 4 --ours-series assoc:macro $D/CMU-1-JP2K-33005/DCM_0.dcm
$O --repeat 1 --series 5 --ours-series assoc:label $D/CMU-1-JP2K-33005/DCM_0.dcm
L=$(ls $D/Leica-4/*.dcm | head -1); H="$D/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm"
$O --series 0 --crop 10000,10000,2048,2048 "$L"
$O --repeat 1 --series 1 --ours-series 0/1 --crop 2000,2000,1024,1024 "$L"
$O --repeat 1 --series 2 --ours-series 0/2 "$L"
$O --series 0 --crop 15000,20000,2048,2048 "$H"
$O --repeat 1 --series 3 --ours-series 0/3 "$H"
$O --repeat 1 --series 7 --ours-series 0/7 "$H"
$O --series 0 --crop 28000,30000,2048,2048 $D/3DHISTECH-1/000001.dcm
$O --repeat 1 --series 4 --ours-series 0/4 $D/3DHISTECH-1/000001.dcm
$O --repeat 1 tests/data/dicom/wsi/level0.dcm tests/data/dicom/fluo/level0.dcm tests/data/dicom/plain.dcm
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

LIF (Bio-Formats fails on the whole FRET project, "Invalid plane index",
but converts each of its series):

```sh
L=tests/reference/ome/Leica-LIF
F="$L/imagesc-30856/20191025 Test FRET 585. 423, 426.lif"
python3 -I tests/scripts/oracle.py lif --per-series tests/external/laica_01.lif $L/michael/PR2729_frameOrderCombinedScanTypes.lif "$F" $L/zenodo-6606445/Project007.lif
for k in 0 1 2 3 4 5 6 7; do python3 -I tests/scripts/oracle.py lif --repeat 1 --series $k --append "$F"; done
```

SVS (`--ours-series 0/2` compares a Bio-Formats pyramid series with level 2
of series 0; slides are compared on crops):

```sh
A=tests/reference/openslide/Aperio
python3 -I tests/scripts/oracle.py svs --keep-series 0 $A/CMU-1-Small-Region.svs
python3 -I tests/scripts/oracle.py svs --repeat 1 --series 1 --ours-series assoc:label --append $A/CMU-1-Small-Region.svs
python3 -I tests/scripts/oracle.py svs --repeat 1 --series 2 --ours-series assoc:macro --append $A/CMU-1-Small-Region.svs
for f in CMU-1 CMU-1-JP2K-33005; do
  python3 -I tests/scripts/oracle.py svs --series 0 --crop 20000,15000,2048,2048 --append $A/$f.svs
  python3 -I tests/scripts/oracle.py svs --series 2 --ours-series 0/2 --append $A/$f.svs
  python3 -I tests/scripts/oracle.py svs --repeat 1 --series 3 --ours-series assoc:label --append $A/$f.svs
  python3 -I tests/scripts/oracle.py svs --repeat 1 --series 4 --ours-series assoc:macro --append $A/$f.svs
done
python3 -I tests/scripts/oracle.py svs --series 0 --crop 6000,8000,2048,2048 --append $A/JP2K-33003-1.svs
python3 -I tests/scripts/oracle.py svs --series 0 --crop 30000,20000,2048,2048 --append tests/reference/ome/SVS/77928.svs
```

NDPI:

```sh
H=tests/reference/openslide/Hamamatsu; M=tests/reference/ome/Hamamatsu-NDPI/manuel
python3 -I tests/scripts/oracle.py ndpi --series 0 --crop 20000,15000,2048,2048 $H/CMU-1.ndpi
python3 -I tests/scripts/oracle.py ndpi --series 2 --ours-series 0/2 --append $H/CMU-1.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 3 --ours-series 0/3 --append $H/CMU-1.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 4 --ours-series assoc:macro --append $H/CMU-1.ndpi
python3 -I tests/scripts/oracle.py ndpi --series 0 --crop 10000,20000,2048,2048 --append $H/Hamamatsu-2.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 5 --ours-series 0/5 --append $H/Hamamatsu-2.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 6 --ours-series assoc:macro --append $H/Hamamatsu-2.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 7 --ours-series assoc:map --append $H/Hamamatsu-2.ndpi
for f in "$M"/*.ndpi; do
  python3 -I tests/scripts/oracle.py ndpi --keep-series 0 --append "$f"
  python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 1 --ours-series 0/1 --append "$f"
done
# NDPIS set; bfconvert fails on the whole series 0 ("Invalid plane index"), a full-size crop works
python3 -I tests/scripts/oracle.py ndpi --series 0 --crop 0,0,3968,4864 --append $M/test3.ndpis
python3 -I tests/scripts/oracle.py ndpi --tools bioformats-rs --keep-series 0 --append $M/test3.ndpis
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 1 --ours-series 0/1 --append $M/test3.ndpis
# Above 4 GiB (fetch_openslide.py --large): 64-bit NDPI offsets
python3 -I tests/scripts/oracle.py ndpi --series 0 --crop 120000,60000,2048,2048 --append $H/Hamamatsu-1.ndpi
python3 -I tests/scripts/oracle.py ndpi --repeat 1 --series 4 --ours-series 0/4 --append $H/Hamamatsu-1.ndpi
```

LSM:

```sh
(ls tests/reference/zenodo-*/*.lsm; echo tests/data/lsm/synthetic.lsm) | tr '\n' '\0' | xargs -0 python3 -I tests/scripts/oracle.py lsm
```

QPTIFF: `sh tests/scripts/oracle_qptiff.sh`; SCN: `sh tests/scripts/oracle_scn.sh`
(Bio-Formats lists SCN macro images as series); BIF: `sh tests/scripts/oracle_bif.sh`
(OpenSlide 3.4.1 rejects the sample); IMS: `sh tests/scripts/oracle_ims.sh`
(`REP=3` for timings).

MIRAX: Bio-Formats and bioformats-rs only read the preview JPEG that the
`.mrxs` file is, so OpenSlide (`apt install openslide-tools`, run as a black
box) is the reference. `--tools openslide --openslide-bounds` renders the same
crop with `openslide-write-png` (our canvas starts at OpenSlide's bounds):

```sh
M=tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs
python3 -I tests/scripts/oracle.py mrxs --tools openslide --openslide-bounds --ours-series 0/0 --crop 1600,2400,1024,1024 $M
# all crops: bash tests/scripts/oracle_mrxs.sh (REP=3 for timings);
# the Mirax2.2-1 crops need fetch_openslide.py --large
```

Divergence files may name one reference series (`series` column) or one
cropped reference (`crop` column). Options go before the file names. Every reader runs under `measure`, which
kills it once its process group exceeds `MEASURE_MAX_RSS_MIB` (default
8192) of resident memory. bioformats-rs otherwise grows to more than 50 GiB
on `Exception9_e3.nd2`.

`oracle.py` converts each file with both readers to ICS, records metadata and
a CRC-32 of the pixels, and measures CPU time and peak RAM of each reader
(median of `--repeat` runs, default 3). Bio-Formats is run with `-expand`, so
8-bit palette images come out as RGB. If a reference differs from scopeio, verify
scopeio against the format specification and the raw file bytes. If scopeio
is right, add the case to `<format>_divergences.tsv` with the reason.

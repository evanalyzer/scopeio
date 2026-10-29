# Tests

`cargo test` needs nothing besides this repository. Reference results from
Bio-Formats and bioformats-rs are committed in `tests/expected/`; tests for
sample files that are not present locally are skipped.

## Layout

| Path | Committed | Content |
|---|---|---|
| `data/<format>/` | yes | Small synthetic samples written from the format specification, plus `truth.tsv` with the CRC-32 each file must decode to |
| `expected/<format>.tsv` | yes | Metadata and pixel CRC-32 from both reference readers |
| `expected/<format>_divergences.tsv` | yes | Known, explained differences from the reference readers; optional columns: `tolerance` (block means, lossy codecs), `ignore` (metadata fields not compared, e.g. `phys_x_nm,phys_y_nm`) |
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
python3 -I tests/scripts/make_zvi_samples.py      # Zeiss ZVI (shares cfbwriter.py with the OIF generator)
python3 -I tests/scripts/make_xlef_samples.py     # Leica LOF and XLEF/XLCF/XLIF
python3 -I tests/scripts/make_stk_samples.py      # MetaMorph STK
python3 -I tests/scripts/make_dv_samples.py       # DeltaVision
python3 -I tests/scripts/make_mrc_samples.py      # MRC
python3 -I tests/scripts/make_sis_samples.py      # Olympus SIS TIFF
python3 -I tests/scripts/make_fluoview_samples.py # Olympus FluoView TIFF
python3 -I tests/scripts/make_nifti_samples.py    # NIfTI
python3 -I tests/scripts/make_analyze_samples.py  # Analyze 7.5
python3 -I tests/scripts/make_scanr_samples.py    # Olympus scan^R
python3 -I tests/scripts/make_nikon_tiff_samples.py # Nikon NIS-Elements TIFF
python3 -I tests/scripts/make_nef_samples.py      # Nikon NEF
python3 -I tests/scripts/make_omp2info_samples.py # Olympus OMP2INFO (after make_oir_samples.py)
python3 -I tests/scripts/make_operetta_samples.py # PerkinElmer Operetta (Harmony export)
python3 -I tests/scripts/make_micromanager_samples.py # Micro-Manager stacks, separate files, NDTiff
python3 -I tests/scripts/make_bdv_samples.py     # BigDataViewer XML/HDF5 (uses the writer of make_ims_samples.py)
python3 -I tests/scripts/make_tiff_samples.py
python3 -I tests/scripts/make_zarr_samples.py
python3 -I tests/scripts/make_vsi_samples.py
python3 -I tests/scripts/make_nd2_samples.py
python3 -I tests/scripts/make_czi_samples.py
python3 -I tests/scripts/make_lif_samples.py
python3 -I tests/scripts/make_svs_samples.py
python3 -I tests/scripts/make_ndpi_samples.py   # needs ImageMagick
python3 -I tests/scripts/make_vms_samples.py    # needs ImageMagick
python3 -I tests/scripts/make_omexml_samples.py   # OME-XML with inline pixels
python3 -I tests/scripts/make_xrm_samples.py      # Zeiss Xradia TXRM / XRM (uses cfbwriter.py)
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
# Zeiss ZVI: Figshare deposits (CC BY 4.0 / CC0) and the synthetic files
python3 -I tests/scripts/fetch_figshare.py "26880337:Figure 1A+I.zvi" "26880337:Figure 4C.zvi" 24769857 15042921 \
    12133278 11341661 32769939 31813711 10732844 6798824 6840530 12132993
find tests/reference/figshare -name "*.zvi" -print0 | sort -z | xargs -0 python3 -I tests/scripts/oracle.py zvi --repeat 3
python3 -I tests/scripts/oracle.py zvi --append --repeat 1 tests/data/zvi/gray.zvi tests/data/zvi/colour.zvi
# MetaMorph STK: Figshare deposits (CC BY 4.0 / CC0); ND acquisition: OME sample (CC0), one series per run
python3 -I tests/scripts/fetch_figshare.py --ext .stk 7583960 12981617 6852419
find tests/reference/figshare -name "*.stk" -print0 | sort -z | xargs -0 python3 -I tests/scripts/oracle.py stk --repeat 3
python3 -I tests/scripts/fetch_ome.py Metamorph/zenodo-13642395/
python3 -I tests/scripts/oracle.py stk --append --per-series --repeat 1 tests/reference/ome/Metamorph/zenodo-13642395/test_timelapse_20240816.nd
# DeltaVision: OME siRNAi-HeLa samples (CC BY 4.0) and the synthetic files
D=tests/reference/ome/DV/siRNAi-HeLa
python3 -I tests/scripts/fetch_ome.py DV/siRNAi-HeLa/IN_02.r3d DV/siRNAi-HeLa/IN_02.r3d_D3D.dv DV/siRNAi-HeLa/IN_02.r3d_D3D_PRJ.dv \
    DV/siRNAi-HeLa/VRAQ_01.r3d DV/siRNAi-HeLa/CSFV_10.r3d_D3D_PRJ.dv
python3 -I tests/scripts/oracle.py dv --repeat 3 $D/IN_02.r3d $D/IN_02.r3d_D3D.dv $D/IN_02.r3d_D3D_PRJ.dv $D/VRAQ_01.r3d $D/CSFV_10.r3d_D3D_PRJ.dv
python3 -I tests/scripts/oracle.py dv --append --repeat 1 tests/data/dv/wzt_be.r3d tests/data/dv/ztw_le.dv
# PerkinElmer Operetta (Harmony exports): OME samples zenodo-7841360 and omer (CC BY 4.0) and 59548 (no
# licence stated: downloaded only). 59548 keeps its planes gzipped in <guid>/; unpack them next to the .gz files.
H=tests/reference/ome/PerkinElmer-Operetta
python3 -I tests/scripts/fetch_ome.py PerkinElmer-Operetta/zenodo-7841360/ PerkinElmer-Operetta/59548/ PerkinElmer-Operetta/omer/
for g in $H/59548/*/*.tiff.gz; do [ -e "${g%.gz}" ] || gzip -dkc "$g" > "${g%.gz}"; done
python3 -I tests/scripts/oracle.py operetta --per-series --repeat 3 "$H/zenodo-7841360/Hoechst__2023-03-07T10_17_54-Measurement 2/Images/Index.idx.xml"
python3 -I tests/scripts/oracle.py operetta --repeat 1 --append $H/59548/59548__*/Images/Index.ref.xml
# omer: --per-series, bfconvert returns wrong pixels for later series when converting all at once
python3 -I tests/scripts/oracle.py operetta --per-series --repeat 1 --append "$H/omer/006P_M3/006P__2017-08-19T12_42_59-Measurement 3/Images/Index.idx.xml"
# MRC: EMDB map EMD-2225 (EMDB data are free of copyright restrictions) and the synthetic files
python3 -I tests/scripts/fetch_ome.py MRC/EMDB/EMD-2225/EMD-2225.map
python3 -I tests/scripts/oracle.py mrc --repeat 3 tests/reference/ome/MRC/EMDB/EMD-2225/EMD-2225.map
python3 -I tests/scripts/oracle.py mrc --append --repeat 1 tests/data/mrc/*.mrc tests/data/mrc/*.mrcs tests/data/mrc/*.st
# Olympus SIS TIFF: analySIS TEM images from Figshare (CC BY 4.0) and the synthetic file
python3 -I tests/scripts/fetch_figshare.py --ext .tif 19467287 19640952 19467233
python3 -I tests/scripts/oracle.py sis --repeat 3 tests/reference/figshare/19467233/*.TIF tests/reference/figshare/19467287/*.TIF tests/reference/figshare/19640952/*.TIF
python3 -I tests/scripts/oracle.py sis --append --repeat 1 tests/data/sis/palette.tif
# Olympus FluoView TIFF: synthetic files only (no public sample found)
python3 -I tests/scripts/oracle.py fluoview --repeat 3 tests/data/fluoview/zc.tif tests/data/fluoview/ct.tif
# NIfTI: official NIfTI test data (public domain) and the synthetic files
N=NIfTI/NIH; D=tests/reference/ome/$N; T=tests/data/nifti
python3 -I tests/scripts/fetch_ome.py $N/avg152T1_LR_nifti.hdr $N/avg152T1_LR_nifti.img $N/avg152T1_LR_nifti.nii $N/avg152T1_LR_nifti.nii.gz \
    $N/avg152T1_RL_nifti.hdr $N/avg152T1_RL_nifti.img $N/avg152T1_RL_nifti.nii $N/minimal.hdr $N/minimal.img $N/minimal.nii \
    $N/zstat1.nii $N/zstat1.nii.gz $N/filtered_func_data.nii $N/filtered_func_data.nii.gz $N/newsirp_final_XML.nii
python3 -I tests/scripts/oracle.py nifti --repeat 3 $D/*.nii $D/*.hdr $D/*.nii.gz
python3 -I tests/scripts/oracle.py nifti --append --repeat 1 $T/le_u16_5d.nii $T/rgb.nii $T/v2_f64.nii.gz $T/v2_pair.hdr
# Olympus scan^R: IDR idr0009 plate (CC BY 4.0, 2.9 GB) and the synthetic plate
python3 -I tests/scripts/fetch_ome.py ScanR/idr0009/0307-10--2007-05-30/
python3 -I tests/scripts/oracle.py scanr --repeat 1 tests/reference/ome/ScanR/idr0009/0307-10--2007-05-30/experiment_descriptor.xml
python3 -I tests/scripts/oracle.py scanr --append --repeat 1 tests/data/scanr/plate/experiment_descriptor.xml
# Nikon NIS-Elements TIFF: public exports from Zenodo (CC BY 4.0) and the synthetic file
python3 -I tests/scripts/fetch_zenodo.py 7677868 h4k20me002xy13c2.tif
python3 -I tests/scripts/fetch_zenodo.py 7677827 single_plane_1-2000_2xy01c1.tif
python3 -I tests/scripts/fetch_zenodo.py 8006449 D4_E1_M_ROI.tif
python3 -I tests/scripts/fetch_zenodo.py 8245566 C12_E3_ROI.tif
python3 -I tests/scripts/oracle.py nikon-tiff --repeat 3 tests/reference/zenodo-7677868/h4k20me002xy13c2.tif \
    tests/reference/zenodo-7677827/single_plane_1-2000_2xy01c1.tif tests/reference/zenodo-8006449/D4_E1_M_ROI.tif \
    tests/reference/zenodo-8245566/C12_E3_ROI.tif
python3 -I tests/scripts/oracle.py nikon-tiff --append --repeat 1 tests/data/nikon-tiff/gray.tif
# Nikon NEF: D300 samples from raw.pixls.us (CC0) and the synthetic file
python3 -I tests/scripts/fetch_rawpixls.py 929 924 928
python3 -I tests/scripts/oracle.py nef --repeat 3 tests/reference/rawpixls/929.nef tests/reference/rawpixls/924.nef tests/reference/rawpixls/928.nef
python3 -I tests/scripts/oracle.py nef --append --repeat 1 tests/data/nef/raw12.nef
# Olympus OMP2INFO: the zenodo-13680725 tiling (CC BY 4.0, 25 tiles + vendor stitch, about 3.5 GB)
O=Olympus-OIR/gh-4205/zenodo-13680725
python3 -I tests/scripts/fetch_ome.py $O/matl.omp2info $O/Map_A01.oir $O/Stitch_A01_G001.oir $O/Stitch_A01_G001_00001 \
    $(for i in $(seq -w 1 25); do echo $O/tiling-sample-brain-section_A01_G001_00$i.oir; done)
python3 -I tests/scripts/oracle.py omp2info --repeat 1 tests/reference/ome/$O/matl.omp2info
# Micro-Manager: OME samples (CC BY 4.0), NDTiff test datasets (Zenodo 7065552, CC BY 4.0), synthetic files
M=Micro-Manager; R=tests/reference/ome/$M; T=tests/data/micromanager
python3 -I tests/scripts/fetch_ome.py $M/1.4.22/thomas/ $M/1.4.23/jens/ $M/1.4.16/serge/Pos0/ $M/1.4.16/serge/Pos1/
python3 -I tests/scripts/fetch_zenodo.py 7065552 NDTiff_test_data.zip
(cd tests/reference/zenodo-7065552 && python3 -I -m zipfile -e NDTiff_test_data.zip extracted)
python3 -I tests/scripts/oracle.py micromanager --repeat 3 $R/1.4.22/thomas/test/test_stack/test_stack_MMStack_Pos0.ome.tif \
    $R/1.4.22/thomas/test/test2_stack/MMStack_Pos0.ome.tif $R/1.4.22/thomas/test/test_sep/Pos0/metadata.txt \
    $R/1.4.22/thomas/test/test2_sep/Pos0/metadata.txt $R/1.4.23/jens/fucci_GFP-geminin_RFP-cdt1_2_MMStack_Pos0.ome.tif \
    $R/1.4.16/serge/Pos0/metadata.txt \
    tests/reference/zenodo-7065552/extracted/NDTiff_test_data/v3/mm_mda_tcz_15/mm_mda_tcz_15_NDTiffStack.tif
python3 -I tests/scripts/oracle.py micromanager --append --repeat 1 $T/stack/syn_MMStack_Pos0.ome.tif \
    $T/sep/Pos0/metadata.txt $T/sep2/Pos0/metadata.txt $T/nd/nd_NDTiffStack.tif
# Micro-Manager two-file stack (4.6 GB): no reference reader reads it
python3 -I tests/scripts/fetch_ome.py $M/1.4.22/sebastien/
python3 -I tests/scripts/oracle.py micromanager --append --repeat 1 $R/1.4.22/sebastien/Test_Dataset_MultiPosition_Stack_1_MMStack_Pos0.ome.tif
# BigDataViewer: OME BDV samples (public domain) and the synthetic dataset
python3 -I tests/scripts/fetch_ome.py BDV/samples/
python3 -I tests/scripts/oracle.py bdv --repeat 1 tests/reference/ome/BDV/samples/drosophila.xml tests/reference/ome/BDV/samples/HisYFP-SPIM.xml
python3 -I tests/scripts/oracle.py bdv --append --repeat 1 tests/data/bdv/synthetic.xml
# Analyze 7.5: synthetic files only
python3 -I tests/scripts/oracle.py analyze --repeat 3 tests/data/analyze/*.hdr
# Hamamatsu VMS: OpenSlide test slide (CC0); the map needs a larger Bio-Formats heap
python3 -I tests/scripts/fetch_openslide.py Hamamatsu-vms/CMU-1.zip
V="tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms"; O="python3 -I tests/scripts/oracle.py vms"
$O --series 0 --crop 60928,30000,1024,1024 "$V"
$O --append --series 0 --crop 40000,60928,2048,1024 "$V"
$O --append --repeat 1 --series 0 --crop 101376,75264,1024,1024 "$V"
$O --append --repeat 1 --series 1 --ours-series assoc:macro "$V"
BF_MAX_MEM=8g $O --append --repeat 1 --series 2 --ours-series 0/1 --tools bioformats "$V"
# OME-XML: OME sample files (written by the OME project) and the synthetic files
python3 -I tests/scripts/fetch_ome.py OME-XML/2016-06/ OME-XML/2008-09/
python3 -I tests/scripts/oracle.py omexml --repeat 1 tests/reference/ome/OME-XML/2016-06/*.xml tests/reference/ome/OME-XML/2008-09/*.xml
python3 -I tests/scripts/oracle.py omexml --append --repeat 1 tests/data/omexml/*.xml
# Zeiss Xradia: Zenodo 4432446 (CC BY 4.0) and the synthetic files
python3 -I tests/scripts/fetch_zenodo.py 4432446 XCT_predeformation_XradiaVersa520.txm
python3 -I tests/scripts/oracle.py xrm --repeat 1 tests/reference/zenodo-4432446/XCT_predeformation_XradiaVersa520.txm
python3 -I tests/scripts/oracle.py xrm --append --repeat 1 tests/data/xrm/projections.txrm tests/data/xrm/single.xrm
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

Leica LOF and XLEF (OME `Leica-XLEF` samples; the folder states no licence,
so the files are only downloaded, never committed; its `lif/` folder holds
plain LIF files):

```sh
python3 -I tests/scripts/fetch_ome.py Leica-XLEF/
X=tests/reference/ome/Leica-XLEF; Y="$X/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small"
find $X -path "$X/lif" -prune -o -name "*.xlef" -print0 | sort -z | xargs -0 python3 -I tests/scripts/oracle.py xlef --per-series --repeat 1
python3 -I tests/scripts/oracle.py xlef --per-series --repeat 1 --append "$Y/Image001.lof" "$Y/Series002.lof"
# Bio-Formats fails on these as a whole ("Invalid plane index") but converts each series
R="$X/rgb channel test"; T="$X/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef"
for k in 0 1 2; do for f in "$R/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef" "$R/XLEF-TIF rgb channel test/rgb channel test.xlef"; do
    python3 -I tests/scripts/oracle.py xlef --repeat 1 --series $k --append "$f"; done; done
for k in 0 1 2 3 4; do python3 -I tests/scripts/oracle.py xlef --repeat 1 --series $k --append "$T"; done
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

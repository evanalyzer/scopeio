# micromanager: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_stack/test_stack_MMStack_Pos0.ome.tif | 2852 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_stack/test_stack_MMStack_Pos0.ome.tif | 2852 KiB | bioformats | ok | 1.244 | 0.632 | 147.5 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_stack/test_stack_MMStack_Pos0.ome.tif | 2852 KiB | bioformats-rs | ok | 0.008 | 0.009 | 10.9 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_stack/MMStack_Pos0.ome.tif | 2727 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_stack/MMStack_Pos0.ome.tif | 2727 KiB | bioformats | ok | 1.185 | 0.619 | 143.0 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_stack/MMStack_Pos0.ome.tif | 2727 KiB | bioformats-rs | ok | 0.005 | 0.006 | 10.6 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_sep/Pos0/metadata.txt | 31 KiB | scopeio | ok | 0.002 | 0.003 | 4.7 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_sep/Pos0/metadata.txt | 31 KiB | bioformats | ok | 4.546 | 1.117 | 369.4 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test_sep/Pos0/metadata.txt | 31 KiB | bioformats-rs | ok | 0.004 | 0.006 | 13.5 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_sep/Pos0/metadata.txt | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_sep/Pos0/metadata.txt | 1 KiB | bioformats | ok | 2.585 | 1.131 | 135.6 |
| tests/reference/ome/Micro-Manager/1.4.22/thomas/test/test2_sep/Pos0/metadata.txt | 1 KiB | bioformats-rs | error: Error opening <file>: Format error: MicroManager: mis | 0.003 | 0.004 | 12.9 |
| tests/reference/ome/Micro-Manager/1.4.23/jens/fucci_GFP-geminin_RFP-cdt1_2_MMStack_Pos0.ome.tif | 424844 KiB | scopeio | ok | 0.080 | 0.082 | 9.2 |
| tests/reference/ome/Micro-Manager/1.4.23/jens/fucci_GFP-geminin_RFP-cdt1_2_MMStack_Pos0.ome.tif | 424844 KiB | bioformats | ok | 4.973 | 2.572 | 374.2 |
| tests/reference/ome/Micro-Manager/1.4.23/jens/fucci_GFP-geminin_RFP-cdt1_2_MMStack_Pos0.ome.tif | 424844 KiB | bioformats-rs | ok | 0.760 | 0.765 | 841.5 |
| tests/reference/ome/Micro-Manager/1.4.16/serge/Pos0/metadata.txt | 302 KiB | scopeio | ok | 0.343 | 0.345 | 29.8 |
| tests/reference/ome/Micro-Manager/1.4.16/serge/Pos0/metadata.txt | 302 KiB | bioformats | ok | 13.987 | 3.025 | 715.1 |
| tests/reference/ome/Micro-Manager/1.4.16/serge/Pos0/metadata.txt | 302 KiB | bioformats-rs | ok | 0.338 | 0.343 | 461.5 |
| tests/reference/zenodo-7065552/extracted/NDTiff_test_data/v3/mm_mda_tcz_15/mm_mda_tcz_15_NDTiffStack.tif | 57963 KiB | scopeio | ok | 0.019 | 0.020 | 4.8 |
| tests/reference/zenodo-7065552/extracted/NDTiff_test_data/v3/mm_mda_tcz_15/mm_mda_tcz_15_NDTiffStack.tif | 57963 KiB | bioformats | ok | 2.103 | 1.098 | 337.3 |
| tests/reference/zenodo-7065552/extracted/NDTiff_test_data/v3/mm_mda_tcz_15/mm_mda_tcz_15_NDTiffStack.tif | 57963 KiB | bioformats-rs | ok | 0.067 | 0.069 | 123.7 |
| tests/data/micromanager/stack/syn_MMStack_Pos0.ome.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/micromanager/stack/syn_MMStack_Pos0.ome.tif | 2 KiB | bioformats | ok | 1.336 | 0.642 | 116.9 |
| tests/data/micromanager/stack/syn_MMStack_Pos0.ome.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.005 | 11.0 |
| tests/data/micromanager/sep/Pos0/metadata.txt | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.3 |
| tests/data/micromanager/sep/Pos0/metadata.txt | 1 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.767 | 0.439 | 101.6 |
| tests/data/micromanager/sep/Pos0/metadata.txt | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Hitachi: mi | 0.002 | 0.003 | 12.8 |
| tests/data/micromanager/sep2/Pos0/metadata.txt | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/data/micromanager/sep2/Pos0/metadata.txt | 1 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.781 | 0.465 | 101.4 |
| tests/data/micromanager/sep2/Pos0/metadata.txt | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Hitachi: mi | 0.003 | 0.004 | 12.9 |
| tests/data/micromanager/nd/nd_NDTiffStack.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/data/micromanager/nd/nd_NDTiffStack.tif | 2 KiB | bioformats | ok | 1.353 | 0.639 | 120.3 |
| tests/data/micromanager/nd/nd_NDTiffStack.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.5 |
| tests/reference/ome/Micro-Manager/1.4.22/sebastien/Test_Dataset_MultiPosition_Stack_1_MMStack_Pos0.ome.tif | 4195853 KiB | scopeio | ok | 2.649 | 5.607 | 5.1 |
| tests/reference/ome/Micro-Manager/1.4.22/sebastien/Test_Dataset_MultiPosition_Stack_1_MMStack_Pos0.ome.tif | 4195853 KiB | bioformats | error: loci.formats.FormatException: Missing file Test_Datas | 8.765 | 2.932 | 720.4 |
| tests/reference/ome/Micro-Manager/1.4.22/sebastien/Test_Dataset_MultiPosition_Stack_1_MMStack_Pos0.ome.tif | 4195853 KiB | bioformats-rs | error: killed: exceeded the RSS limit of 8192 MiB | 4401.228 | 4464.471 | 8202.8 |

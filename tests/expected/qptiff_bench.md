# qptiff: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif | 23586 KiB | scopeio | ok | 0.126 | 0.128 | 13.8 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif | 23586 KiB | bioformats | ok | 1.939 | 1.026 | 269.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif | 23586 KiB | bioformats-rs | ok | 0.161 | 0.163 | 49.5 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif (series 1) | 23586 KiB | scopeio | ok | 0.128 | 0.129 | 13.5 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif (series 1) | 23586 KiB | bioformats | ok | 1.352 | 0.581 | 178.8 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/HnE_3_1x1component_data.tif (series 1) | 23586 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/LuCa-7color_[13860,52919]_1x1component_data.tif | 83347 KiB | scopeio | ok | 0.479 | 0.480 | 13.7 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/LuCa-7color_[13860,52919]_1x1component_data.tif | 83347 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 3.263 | 2.071 | 477.5 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_fields/LuCa-7color_[13860,52919]_1x1component_data.tif | 83347 KiB | bioformats-rs | ok | 0.561 | 0.564 | 170.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 411939 KiB | scopeio | ok | 0.061 | 0.062 | 17.1 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 411939 KiB | bioformats | ok | 2.194 | 1.040 | 291.7 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 411939 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 2041287 KiB | scopeio | ok | 0.095 | 0.096 | 8.4 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 2041287 KiB | bioformats | ok | 2.255 | 1.066 | 290.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 0) crop 20000,15000,2048,2048 | 2041287 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 3) | 2041287 KiB | scopeio | ok | 0.202 | 0.203 | 16.8 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 3) | 2041287 KiB | bioformats | ok | 2.844 | 1.387 | 427.4 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 3) | 2041287 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 7) | 2041287 KiB | scopeio | ok | 10.716 | 10.722 | 826.7 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 7) | 2041287 KiB | bioformats | ok | 2.685 | 1.447 | 289.3 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 7) | 2041287 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 8) | 2041287 KiB | scopeio | ok | 10.636 | 10.642 | 827.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 8) | 2041287 KiB | bioformats | ok | 1.815 | 0.684 | 205.6 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/LuCa-7color_Scan1.qptiff (series 8) | 2041287 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 3) | 411939 KiB | scopeio | ok | 0.103 | 0.105 | 41.8 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 3) | 411939 KiB | bioformats | ok | 2.795 | 1.657 | 436.6 |
| tests/reference/ome/Vectra-QPTIFF/perkinelmer/PKI_scans/HandEcompressed_Scan1.qptiff (series 3) | 411939 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

# lif: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/external/laica_01.lif | 110676 KiB | scopeio | ok | 0.035 | 0.038 | 4.4 |
| tests/external/laica_01.lif | 110676 KiB | bioformats | ok | 4.399 | 1.286 | 521.5 |
| tests/external/laica_01.lif | 110676 KiB | bioformats-rs | ok | 0.115 | 0.116 | 227.7 |
| tests/reference/ome/Leica-LIF/michael/PR2729_frameOrderCombinedScanTypes.lif | 222 KiB | scopeio | ok | 0.002 | 0.003 | 3.3 |
| tests/reference/ome/Leica-LIF/michael/PR2729_frameOrderCombinedScanTypes.lif | 222 KiB | bioformats | ok | 2.462 | 0.808 | 228.2 |
| tests/reference/ome/Leica-LIF/michael/PR2729_frameOrderCombinedScanTypes.lif | 222 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.9 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif | 48157 KiB | scopeio | ok | 0.026 | 0.027 | 4.3 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif | 48157 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.966 | 1.890 | 581.2 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif | 48157 KiB | bioformats-rs | ok | 0.010 | 0.011 | 17.8 |
| tests/reference/ome/Leica-LIF/zenodo-6606445/Project007.lif | 109027 KiB | scopeio | ok | 0.053 | 0.054 | 4.6 |
| tests/reference/ome/Leica-LIF/zenodo-6606445/Project007.lif | 109027 KiB | bioformats | ok | 11.790 | 2.528 | 727.6 |
| tests/reference/ome/Leica-LIF/zenodo-6606445/Project007.lif | 109027 KiB | bioformats-rs | ok | 0.017 | 0.018 | 25.6 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 0) | 48157 KiB | scopeio | ok | 0.004 | 0.005 | 4.1 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 0) | 48157 KiB | bioformats | ok | 9.122 | 1.814 | 569.5 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 0) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 1) | 48157 KiB | scopeio | ok | 0.005 | 0.006 | 4.4 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 1) | 48157 KiB | bioformats | ok | 9.323 | 1.819 | 566.5 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 1) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 2) | 48157 KiB | scopeio | ok | 0.004 | 0.005 | 4.1 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 2) | 48157 KiB | bioformats | ok | 9.445 | 1.920 | 565.6 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 2) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 3) | 48157 KiB | scopeio | ok | 0.008 | 0.009 | 4.1 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 3) | 48157 KiB | bioformats | ok | 9.011 | 1.942 | 568.9 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 3) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 4) | 48157 KiB | scopeio | ok | 0.004 | 0.005 | 4.3 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 4) | 48157 KiB | bioformats | ok | 9.115 | 1.839 | 552.9 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 4) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 5) | 48157 KiB | scopeio | ok | 0.005 | 0.006 | 4.3 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 5) | 48157 KiB | bioformats | ok | 8.856 | 1.799 | 546.7 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 5) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 6) | 48157 KiB | scopeio | ok | 0.009 | 0.010 | 4.2 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 6) | 48157 KiB | bioformats | ok | 10.282 | 2.124 | 595.7 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 6) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 7) | 48157 KiB | scopeio | ok | 0.014 | 0.015 | 4.3 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 7) | 48157 KiB | bioformats | ok | 9.781 | 1.983 | 649.0 |
| tests/reference/ome/Leica-LIF/imagesc-30856/20191025 Test FRET 585. 423, 426.lif (series 7) | 48157 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

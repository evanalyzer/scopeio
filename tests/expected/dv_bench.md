# dv: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d | 81946 KiB | scopeio | ok | 0.058 | 0.059 | 4.2 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d | 81946 KiB | bioformats | ok | 7.410 | 1.791 | 677.4 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d | 81946 KiB | bioformats-rs | ok | 0.105 | 0.107 | 172.3 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D.dv | 81946 KiB | scopeio | ok | 0.027 | 0.028 | 4.2 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D.dv | 81946 KiB | bioformats | ok | 5.826 | 1.588 | 632.3 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D.dv | 81946 KiB | bioformats-rs | ok | 0.084 | 0.087 | 172.0 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D_PRJ.dv | 2050 KiB | scopeio | ok | 0.002 | 0.003 | 4.4 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D_PRJ.dv | 2050 KiB | bioformats | ok | 1.252 | 0.575 | 139.4 |
| tests/reference/ome/DV/siRNAi-HeLa/IN_02.r3d_D3D_PRJ.dv | 2050 KiB | bioformats-rs | ok | 0.006 | 0.007 | 15.3 |
| tests/reference/ome/DV/siRNAi-HeLa/VRAQ_01.r3d | 81946 KiB | scopeio | ok | 0.058 | 0.060 | 4.5 |
| tests/reference/ome/DV/siRNAi-HeLa/VRAQ_01.r3d | 81946 KiB | bioformats | ok | 5.370 | 1.536 | 608.2 |
| tests/reference/ome/DV/siRNAi-HeLa/VRAQ_01.r3d | 81946 KiB | bioformats-rs | ok | 0.102 | 0.104 | 171.6 |
| tests/reference/ome/DV/siRNAi-HeLa/CSFV_10.r3d_D3D_PRJ.dv | 2050 KiB | scopeio | ok | 0.002 | 0.003 | 4.3 |
| tests/reference/ome/DV/siRNAi-HeLa/CSFV_10.r3d_D3D_PRJ.dv | 2050 KiB | bioformats | ok | 1.259 | 0.584 | 139.6 |
| tests/reference/ome/DV/siRNAi-HeLa/CSFV_10.r3d_D3D_PRJ.dv | 2050 KiB | bioformats-rs | ok | 0.006 | 0.007 | 15.0 |
| tests/data/dv/wzt_be.r3d | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/data/dv/wzt_be.r3d | 4 KiB | bioformats | ok | 1.446 | 0.586 | 150.6 |
| tests/data/dv/wzt_be.r3d | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.2 |
| tests/data/dv/ztw_le.dv | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/data/dv/ztw_le.dv | 2 KiB | bioformats | ok | 1.094 | 0.557 | 115.4 |
| tests/data/dv/ztw_le.dv | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |

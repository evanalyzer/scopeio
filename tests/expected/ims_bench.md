# ims: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_confocal.ims | 7217 KiB | scopeio | ok | 0.003 | 0.004 | 5.6 |
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_confocal.ims | 7217 KiB | bioformats | ok | 1.621 | 0.758 | 180.4 |
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_confocal.ims | 7217 KiB | bioformats-rs | ok | 0.021 | 0.022 | 24.9 |
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_2x2grid_confocal.ims | 23654 KiB | scopeio | ok | 0.081 | 0.083 | 12.7 |
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_2x2grid_confocal.ims | 23654 KiB | bioformats | ok | 1.994 | 1.029 | 356.1 |
| tests/reference/ome/Imaris-IMS/davemason/Convallaria_3C_1T_2x2grid_confocal.ims | 23654 KiB | bioformats-rs | ok | 0.123 | 0.124 | 67.6 |
| tests/reference/ome/Imaris-IMS/eliana/retina_large.ims | 46037 KiB | scopeio | ok | 2.592 | 2.599 | 7.9 |
| tests/reference/ome/Imaris-IMS/eliana/retina_large.ims | 46037 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.015 | 5.005 | 617.6 |
| tests/reference/ome/Imaris-IMS/eliana/retina_large.ims | 46037 KiB | bioformats-rs | ok | 3.416 | 3.420 | 909.2 |
| tests/reference/ome/Imaris-IMS/eliana/2016-07-26_17.11.03_Sample Prefix_5P298C2_Protocol.ims | 130728 KiB | scopeio | ok | 0.030 | 0.031 | 5.6 |
| tests/reference/ome/Imaris-IMS/eliana/2016-07-26_17.11.03_Sample Prefix_5P298C2_Protocol.ims | 130728 KiB | bioformats | ok | 3.418 | 1.505 | 458.8 |
| tests/reference/ome/Imaris-IMS/eliana/2016-07-26_17.11.03_Sample Prefix_5P298C2_Protocol.ims | 130728 KiB | bioformats-rs | ok | 0.258 | 0.259 | 205.6 |

# ndpi: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | scopeio | ok | 0.029 | 0.031 | 16.6 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | bioformats | ok | 1.296 | 0.628 | 255.5 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | scopeio | ok | 0.074 | 0.075 | 26.5 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | bioformats | ok | 1.375 | 0.626 | 232.8 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | scopeio | ok | 0.007 | 0.008 | 9.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | bioformats | ok | 1.459 | 0.662 | 186.7 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | scopeio | ok | 8.158 | 8.162 | 5592.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | bioformats | ok | 1.455 | 0.683 | 183.4 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | scopeio | ok | 0.081 | 0.082 | 16.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | bioformats | ok | 1.706 | 0.766 | 270.6 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | scopeio | ok | 0.012 | 0.013 | 7.3 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | bioformats | ok | 1.870 | 0.783 | 210.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | scopeio | ok | 5.998 | 6.000 | 3446.5 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | bioformats | ok | 1.872 | 0.806 | 209.5 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | scopeio | ok | 6.045 | 6.048 | 3446.6 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | bioformats | ok | 1.495 | 0.617 | 178.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | scopeio | ok | 0.090 | 0.091 | 58.5 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | bioformats | ok | 2.485 | 1.383 | 477.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | bioformats-rs | ok | 0.428 | 0.430 | 175.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | scopeio | ok | 0.023 | 0.024 | 17.1 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | bioformats | ok | 2.017 | 1.038 | 282.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | scopeio | ok | 0.109 | 0.111 | 58.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | bioformats | ok | 2.444 | 1.367 | 456.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | bioformats-rs | ok | 0.453 | 0.454 | 175.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | scopeio | ok | 0.031 | 0.032 | 17.1 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | bioformats | ok | 1.949 | 0.999 | 281.9 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | scopeio | ok | 0.093 | 0.095 | 58.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | bioformats | ok | 2.531 | 1.415 | 483.1 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | bioformats-rs | ok | 0.434 | 0.436 | 175.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | scopeio | ok | 0.027 | 0.028 | 17.2 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | bioformats | ok | 1.918 | 1.012 | 277.9 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | scopeio | ok | 0.055 | 0.057 | 26.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | bioformats | ok | 1.691 | 0.860 | 363.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | scopeio | ok | 0.746 | 0.748 | 226.5 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | bioformats | ok | 2.967 | 1.904 | 762.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | scopeio | ok | 0.354 | 0.357 | 77.5 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 2.963 | 1.627 | 440.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats-rs | ok | 1.471 | 1.472 | 219.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | scopeio | ok | 0.082 | 0.084 | 22.2 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | bioformats | ok | 1.903 | 0.993 | 377.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | scopeio | ok | 0.350 | 0.352 | 77.5 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | bioformats | ok | 1.963 | 1.088 | 428.6 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | scopeio | ok | 0.350 | 0.352 | 77.5 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats-rs | ok | 1.418 | 1.420 | 219.4 |

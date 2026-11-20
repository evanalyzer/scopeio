# ndpi: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- scopeio rows re-measured 2026-11-20 (hayro-jpeg2000 0.5, pyramid levels and associated images as in `ours_series`).
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | scopeio | ok | 0.030 | 0.032 | 18.8 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | bioformats | ok | 1.296 | 0.628 | 255.5 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 0) crop 20000,15000,2048,2048 | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | scopeio | ok | 0.080 | 0.082 | 28.6 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | bioformats | ok | 1.375 | 0.626 | 232.8 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 2) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | scopeio | ok | 0.007 | 0.008 | 11.1 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | bioformats | ok | 1.459 | 0.662 | 186.7 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 3) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | scopeio | ok | 10.870 | 10.877 | 5593.8 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | bioformats | ok | 1.455 | 0.683 | 183.4 |
| tests/reference/openslide/Hamamatsu/CMU-1.ndpi (series 4) | 193390 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | scopeio | ok | 0.094 | 0.096 | 18.2 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | bioformats | ok | 1.706 | 0.766 | 270.6 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 0) crop 10000,20000,2048,2048 | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | scopeio | ok | 0.034 | 0.037 | 9.4 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | bioformats | ok | 1.870 | 0.783 | 210.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 5) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | scopeio | ok | 13.702 | 13.745 | 3448.8 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | bioformats | ok | 1.872 | 0.806 | 209.5 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 6) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | scopeio | ok | 14.735 | 14.806 | 3448.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | bioformats | ok | 1.495 | 0.617 | 178.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-2.ndpi (series 7) | 198371 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | scopeio | ok | 0.094 | 0.096 | 60.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | bioformats | ok | 2.485 | 1.383 | 477.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi | 3461 KiB | bioformats-rs | ok | 0.428 | 0.430 | 175.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | scopeio | ok | 0.025 | 0.031 | 19.4 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | bioformats | ok | 2.017 | 1.038 | 282.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-DAPI 2 (387) .ndpi (series 1) | 3461 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | scopeio | ok | 0.115 | 0.117 | 60.6 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | bioformats | ok | 2.444 | 1.367 | 456.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi | 4379 KiB | bioformats-rs | ok | 0.453 | 0.454 | 175.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | scopeio | ok | 0.028 | 0.035 | 19.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | bioformats | ok | 1.949 | 0.999 | 281.9 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-FITC 2 (485).ndpi (series 1) | 4379 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | scopeio | ok | 0.099 | 0.100 | 60.7 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | bioformats | ok | 2.531 | 1.415 | 483.1 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi | 2993 KiB | bioformats-rs | ok | 0.434 | 0.436 | 175.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | scopeio | ok | 0.027 | 0.033 | 19.3 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | bioformats | ok | 1.918 | 1.012 | 277.9 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3-TRITC 2 (560).ndpi (series 1) | 2993 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | scopeio | ok | 0.056 | 0.058 | 28.1 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | bioformats | ok | 1.691 | 0.860 | 363.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 0) crop 120000,60000,2048,2048 | 6739285 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | scopeio | ok | 0.941 | 0.944 | 228.6 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | bioformats | ok | 2.967 | 1.904 | 762.7 |
| tests/reference/openslide/Hamamatsu/Hamamatsu-1.ndpi (series 4) | 6739285 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | scopeio | ok | 0.097 | 0.099 | 23.9 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 2.963 | 1.627 | 440.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats-rs | ok | 1.471 | 1.472 | 219.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | scopeio | ok | 0.100 | 0.101 | 24.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | bioformats | ok | 1.903 | 0.993 | 377.8 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 1) | 0 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | scopeio | ok | 0.342 | 0.345 | 79.2 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | bioformats | ok | 1.963 | 1.088 | 428.6 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis (series 0) crop 0,0,3968,4864 | 0 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | scopeio | ok | 0.385 | 0.387 | 79.2 |
| tests/reference/ome/Hamamatsu-NDPI/manuel/test3.ndpis | 0 KiB | bioformats-rs | ok | 1.418 | 1.420 | 219.4 |

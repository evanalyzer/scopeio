# svs: CPU and RAM compared with the reference readers

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
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs | 1894 KiB | scopeio | ok | 0.034 | 0.035 | 24.7 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs | 1894 KiB | bioformats | ok | 2.115 | 1.144 | 256.8 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs | 1894 KiB | bioformats-rs | ok | 0.385 | 0.124 | 47.2 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 1) | 1894 KiB | scopeio | ok | 0.035 | 0.037 | 24.8 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 1) | 1894 KiB | bioformats | ok | 1.232 | 0.581 | 145.4 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 1) | 1894 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 2) | 1894 KiB | scopeio | ok | 0.033 | 0.034 | 24.8 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 2) | 1894 KiB | bioformats | ok | 1.299 | 0.600 | 161.8 |
| tests/reference/openslide/Aperio/CMU-1-Small-Region.svs (series 2) | 1894 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 0) crop 20000,15000,2048,2048 | 173391 KiB | scopeio | ok | 0.012 | 0.013 | 18.3 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 0) crop 20000,15000,2048,2048 | 173391 KiB | bioformats | ok | 2.065 | 0.963 | 253.7 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 0) crop 20000,15000,2048,2048 | 173391 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 2) | 173391 KiB | scopeio | ok | 0.031 | 0.032 | 22.7 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 2) | 173391 KiB | bioformats | ok | 2.151 | 1.072 | 258.9 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 2) | 173391 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 3) | 173391 KiB | scopeio | ok | 5.431 | 5.435 | 4337.9 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 3) | 173391 KiB | bioformats | ok | 1.503 | 0.603 | 168.8 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 3) | 173391 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 4) | 173391 KiB | scopeio | ok | 5.190 | 5.193 | 4337.7 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 4) | 173391 KiB | bioformats | ok | 1.687 | 0.639 | 188.2 |
| tests/reference/openslide/Aperio/CMU-1.svs (series 4) | 173391 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 0) crop 20000,15000,2048,2048 | 129458 KiB | scopeio | ok | 0.157 | 0.158 | 19.7 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 0) crop 20000,15000,2048,2048 | 129458 KiB | bioformats | ok | 3.679 | 1.406 | 363.8 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 0) crop 20000,15000,2048,2048 | 129458 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 2) | 129458 KiB | scopeio | ok | 0.217 | 0.219 | 24.3 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 2) | 129458 KiB | bioformats | ok | 3.777 | 1.647 | 433.1 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 2) | 129458 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 3) | 129458 KiB | scopeio | ok | 51.170 | 51.185 | 4336.2 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 3) | 129458 KiB | bioformats | ok | 1.586 | 0.620 | 172.7 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 3) | 129458 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 4) | 129458 KiB | scopeio | ok | 48.094 | 48.111 | 4336.4 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 4) | 129458 KiB | bioformats | ok | 1.725 | 0.640 | 185.8 |
| tests/reference/openslide/Aperio/CMU-1-JP2K-33005.svs (series 4) | 129458 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Aperio/JP2K-33003-1.svs (series 0) crop 6000,8000,2048,2048 | 62351 KiB | scopeio | ok | 0.221 | 0.223 | 19.6 |
| tests/reference/openslide/Aperio/JP2K-33003-1.svs (series 0) crop 6000,8000,2048,2048 | 62351 KiB | bioformats | ok | 3.381 | 1.420 | 422.8 |
| tests/reference/openslide/Aperio/JP2K-33003-1.svs (series 0) crop 6000,8000,2048,2048 | 62351 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/SVS/77928.svs (series 0) crop 30000,20000,2048,2048 | 594897 KiB | scopeio | ok | 0.281 | 0.283 | 20.5 |
| tests/reference/ome/SVS/77928.svs (series 0) crop 30000,20000,2048,2048 | 594897 KiB | bioformats | ok | 4.037 | 1.646 | 383.3 |
| tests/reference/ome/SVS/77928.svs (series 0) crop 30000,20000,2048,2048 | 594897 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

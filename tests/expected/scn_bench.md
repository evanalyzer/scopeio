# scn: CPU and RAM compared with the reference readers

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
| tests/reference/openslide/Leica/Leica-1.scn (series 3) crop 15000,15000,2048,2048 | 284974 KiB | scopeio | ok | 0.035 | 0.038 | 19.4 |
| tests/reference/openslide/Leica/Leica-1.scn (series 3) crop 15000,15000,2048,2048 | 284974 KiB | bioformats | ok | 2.762 | 1.226 | 219.6 |
| tests/reference/openslide/Leica/Leica-1.scn (series 3) crop 15000,15000,2048,2048 | 284974 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-1.scn (series 5) | 284974 KiB | scopeio | ok | 0.032 | 0.034 | 23.0 |
| tests/reference/openslide/Leica/Leica-1.scn (series 5) | 284974 KiB | bioformats | ok | 2.895 | 1.318 | 221.9 |
| tests/reference/openslide/Leica/Leica-1.scn (series 5) | 284974 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-1.scn (series 0) | 284974 KiB | scopeio | ok | 6.903 | 6.909 | 4057.1 |
| tests/reference/openslide/Leica/Leica-1.scn (series 0) | 284974 KiB | bioformats | ok | 3.107 | 1.430 | 230.7 |
| tests/reference/openslide/Leica/Leica-1.scn (series 0) | 284974 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 6) | 21231 KiB | scopeio | ok | 0.227 | 0.233 | 34.7 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 6) | 21231 KiB | bioformats | ok | 3.024 | 1.206 | 321.6 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 6) | 21231 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 7) | 21231 KiB | scopeio | ok | 0.021 | 0.022 | 8.1 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 7) | 21231 KiB | bioformats | ok | 2.386 | 0.858 | 220.1 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 7) | 21231 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 0) | 21231 KiB | scopeio | ok | 0.233 | 0.236 | 34.8 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 0) | 21231 KiB | bioformats | ok | 3.451 | 1.526 | 260.0 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 0) | 21231 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 3) | 21231 KiB | scopeio | ok | 0.238 | 0.241 | 34.8 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 3) | 21231 KiB | bioformats | ok | 3.322 | 1.460 | 255.5 |
| tests/reference/openslide/Leica/Leica-Fluorescence-1.scn (series 3) | 21231 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

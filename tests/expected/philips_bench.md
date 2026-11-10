# philips: CPU and RAM compared with the reference readers

Generated 2026-11-10 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-42-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Philips-TIFF/Philips-1.tiff (series 0) crop 20000,15000,2048,2048 | 318952 KiB | scopeio | ok | 0.039 | 0.040 | 19.1 |
| tests/reference/openslide/Philips-TIFF/Philips-1.tiff (series 0) crop 20000,15000,2048,2048 | 318952 KiB | bioformats | ok | 3.087 | 1.244 | 264.9 |
| tests/reference/openslide/Philips-TIFF/Philips-1.tiff (series 0) crop 20000,15000,2048,2048 | 318952 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Philips-TIFF/Philips-2.tiff (series 0) crop 52736,146944,2048,2048 | 893052 KiB | scopeio | ok | 0.038 | 0.039 | 22.4 |
| tests/reference/openslide/Philips-TIFF/Philips-2.tiff (series 0) crop 52736,146944,2048,2048 | 893052 KiB | bioformats | ok | 3.595 | 1.501 | 362.6 |
| tests/reference/openslide/Philips-TIFF/Philips-2.tiff (series 0) crop 52736,146944,2048,2048 | 893052 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 76800,41472,2048,2048 | 284171 KiB | scopeio | ok | 0.025 | 0.026 | 19.5 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 76800,41472,2048,2048 | 284171 KiB | bioformats | ok | 3.365 | 1.261 | 263.7 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 76800,41472,2048,2048 | 284171 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 15360,15360,2048,2048 | 284171 KiB | scopeio | ok | 0.009 | 0.011 | 19.3 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 15360,15360,2048,2048 | 284171 KiB | bioformats | ok | 3.301 | 1.210 | 268.1 |
| tests/reference/openslide/Philips-TIFF/Philips-4.tiff (series 0) crop 15360,15360,2048,2048 | 284171 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

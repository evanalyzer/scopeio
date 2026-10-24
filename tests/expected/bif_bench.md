# bif: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 4000,8000,4096,2048 | 222048 KiB | scopeio | ok | 0.105 | 0.107 | 34.5 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 4000,8000,4096,2048 | 222048 KiB | bioformats | ok | 2.647 | 1.380 | 420.3 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 4000,8000,4096,2048 | 222048 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 0,0,2048,1024 | 222048 KiB | scopeio | ok | 0.015 | 0.017 | 15.8 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 0,0,2048,1024 | 222048 KiB | bioformats | ok | 2.024 | 0.859 | 244.1 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 0) crop 0,0,2048,1024 | 222048 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 3) | 222048 KiB | scopeio | ok | 2.334 | 2.337 | 30.9 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 3) | 222048 KiB | bioformats | ok | 2.806 | 1.357 | 425.7 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 3) | 222048 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 8) | 222048 KiB | scopeio | ok | 2.860 | 2.874 | 1452.1 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 8) | 222048 KiB | bioformats | ok | 1.951 | 0.859 | 266.5 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 8) | 222048 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 9) | 222048 KiB | scopeio | ok | 2.786 | 2.790 | 1451.8 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 9) | 222048 KiB | bioformats | ok | 1.426 | 0.580 | 181.7 |
| tests/reference/openslide/Ventana/Ventana-1.bif (series 9) | 222048 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

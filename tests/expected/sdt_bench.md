# sdt: CPU and RAM compared with the reference readers

Generated 2026-11-05 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 0) crop 200,200,64,64 | 29335 KiB | scopeio | ok | 0.106 | 0.107 | 48.6 |
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 0) crop 200,200,64,64 | 29335 KiB | bioformats | ok | 27.561 | 24.649 | 983.7 |
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 0) crop 200,200,64,64 | 29335 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 1) crop 300,100,64,64 | 29335 KiB | scopeio | ok | 0.104 | 0.105 | 56.6 |
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 1) crop 300,100,64,64 | 29335 KiB | bioformats | ok | 30.650 | 27.432 | 1086.7 |
| tests/reference/ome/SDT/gh-4198/FocalCheck_A1_20x_8xzoom_800nm.sdt (series 1) crop 300,100,64,64 | 29335 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/data/sdt/image.sdt | 2 KiB | scopeio | ok | 0.001 | 0.002 | 6.1 |
| tests/data/sdt/image.sdt | 2 KiB | bioformats | error: java.lang.IllegalArgumentException: 0 must be non-nul | 0.697 | 0.398 | 100.1 |
| tests/data/sdt/image.sdt | 2 KiB | bioformats-rs | ok | 0.003 | 0.005 | 12.7 |
| tests/data/sdt/decay.sdt | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/sdt/decay.sdt | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: 0 must be non-nul | 0.688 | 0.400 | 101.5 |
| tests/data/sdt/decay.sdt | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |

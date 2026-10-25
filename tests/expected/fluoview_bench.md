# fluoview: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/fluoview/zc.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/fluoview/zc.tif | 3 KiB | bioformats | ok | 1.460 | 0.745 | 108.9 |
| tests/data/fluoview/zc.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.7 |
| tests/data/fluoview/ct.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.3 |
| tests/data/fluoview/ct.tif | 3 KiB | bioformats | ok | 1.299 | 0.601 | 109.0 |
| tests/data/fluoview/ct.tif | 3 KiB | bioformats-rs | ok | 0.004 | 0.005 | 9.8 |

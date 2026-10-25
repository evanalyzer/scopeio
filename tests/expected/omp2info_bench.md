# omp2info: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/matl.omp2info | 10 KiB | scopeio | ok | 1.217 | 2.378 | 95.7 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/matl.omp2info | 10 KiB | bioformats | ok | 359.031 | 351.857 | 594.2 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/matl.omp2info | 10 KiB | bioformats-rs | ok | 186.470 | 187.227 | 6271.1 |

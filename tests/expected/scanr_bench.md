# scanr: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/ScanR/idr0009/0307-10--2007-05-30/experiment_descriptor.xml | 50 KiB | scopeio | ok | 0.603 | 0.607 | 8.3 |
| tests/reference/ome/ScanR/idr0009/0307-10--2007-05-30/experiment_descriptor.xml | 50 KiB | bioformats | ok | 13.873 | 12.862 | 415.0 |
| tests/reference/ome/ScanR/idr0009/0307-10--2007-05-30/experiment_descriptor.xml | 50 KiB | bioformats-rs | ok | 0.042 | 0.051 | 29.5 |
| tests/data/scanr/plate/experiment_descriptor.xml | 10 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/data/scanr/plate/experiment_descriptor.xml | 10 KiB | bioformats | ok | 1.025 | 0.519 | 111.6 |
| tests/data/scanr/plate/experiment_descriptor.xml | 10 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.1 |

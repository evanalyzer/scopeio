# lsm: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/zenodo-10046394/Fig 4A.lsm | 19571 KiB | scopeio | ok | 0.009 | 0.011 | 22.0 |
| tests/reference/zenodo-10046394/Fig 4A.lsm | 19571 KiB | bioformats | ok | 1.407 | 0.623 | 201.3 |
| tests/reference/zenodo-10046394/Fig 4A.lsm | 19571 KiB | bioformats-rs | ok | 0.032 | 0.033 | 47.8 |
| tests/reference/zenodo-10046394/Suppl Fig 1B.lsm | 25206 KiB | scopeio | ok | 0.011 | 0.012 | 27.7 |
| tests/reference/zenodo-10046394/Suppl Fig 1B.lsm | 25206 KiB | bioformats | ok | 1.559 | 0.677 | 217.6 |
| tests/reference/zenodo-10046394/Suppl Fig 1B.lsm | 25206 KiB | bioformats-rs | ok | 0.039 | 0.040 | 58.7 |
| tests/reference/zenodo-13286257/sofa eye_zebrafish_spectral and zstack_01.lsm | 284590 KiB | scopeio | ok | 0.148 | 0.150 | 4.1 |
| tests/reference/zenodo-13286257/sofa eye_zebrafish_spectral and zstack_01.lsm | 284590 KiB | bioformats | ok | 4.754 | 2.140 | 478.1 |
| tests/reference/zenodo-13286257/sofa eye_zebrafish_spectral and zstack_01.lsm | 284590 KiB | bioformats-rs | ok | 1.255 | 1.257 | 560.1 |
| tests/reference/zenodo-14510432/10-01.lsm | 8430 KiB | scopeio | ok | 0.004 | 0.005 | 5.2 |
| tests/reference/zenodo-14510432/10-01.lsm | 8430 KiB | bioformats | ok | 2.245 | 0.771 | 223.3 |
| tests/reference/zenodo-14510432/10-01.lsm | 8430 KiB | bioformats-rs | ok | 0.023 | 0.024 | 29.7 |
| tests/reference/zenodo-16916709/Lan_Larval_Chitin_DMSO_control_01.lsm | 16475 KiB | scopeio | ok | 0.011 | 0.012 | 3.4 |
| tests/reference/zenodo-16916709/Lan_Larval_Chitin_DMSO_control_01.lsm | 16475 KiB | bioformats | ok | 2.500 | 0.874 | 341.6 |
| tests/reference/zenodo-16916709/Lan_Larval_Chitin_DMSO_control_01.lsm | 16475 KiB | bioformats-rs | ok | 0.028 | 0.030 | 40.1 |
| tests/data/lsm/synthetic.lsm | 5 KiB | scopeio | ok | 0.001 | 0.002 | 3.2 |
| tests/data/lsm/synthetic.lsm | 5 KiB | bioformats | error: java.io.EOFException: Attempting to read beyond end o | 0.732 | 0.419 | 102.4 |
| tests/data/lsm/synthetic.lsm | 5 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.0 |

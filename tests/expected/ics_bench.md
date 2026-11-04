# ics: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/ICS/jan/benchmark_v1_2018_x64y64z5c2s1t11_w1Laser4054BD4BP_5c8bc101d6559_hrm.ics | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/ICS/jan/benchmark_v1_2018_x64y64z5c2s1t11_w1Laser4054BD4BP_5c8bc101d6559_hrm.ics | 4 KiB | bioformats | ok | 1.202 | 0.568 | 165.4 |
| tests/reference/ome/ICS/jan/benchmark_v1_2018_x64y64z5c2s1t11_w1Laser4054BD4BP_5c8bc101d6559_hrm.ics | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.1 |
| tests/reference/ome/ICS/gh-4197/20240612_training_test-tiles_01_stitched.ics | 0 KiB | scopeio | ok | 0.752 | 0.754 | 42.1 |
| tests/reference/ome/ICS/gh-4197/20240612_training_test-tiles_01_stitched.ics | 0 KiB | bioformats | ok | 3.786 | 3.041 | 362.8 |
| tests/reference/ome/ICS/gh-4197/20240612_training_test-tiles_01_stitched.ics | 0 KiB | bioformats-rs | ok | 30.447 | 30.473 | 2426.1 |
| tests/data/ics/be_u16.ics | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/data/ics/be_u16.ics | 0 KiB | bioformats | ok | 1.526 | 0.835 | 117.8 |
| tests/data/ics/be_u16.ics | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 8.9 |
| tests/data/ics/ch_first.ics | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/ics/ch_first.ics | 1 KiB | bioformats | ok | 1.013 | 0.541 | 119.3 |
| tests/data/ics/ch_first.ics | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 8.8 |
| tests/data/ics/gzip.ics | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/ics/gzip.ics | 0 KiB | bioformats | ok | 0.932 | 0.507 | 117.0 |
| tests/data/ics/gzip.ics | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 9.2 |

# vsi: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/external/olympus_01.vsi (series 1) | 41070 KiB | scopeio | ok | 0.014 | 0.015 | 11.0 |
| tests/external/olympus_01.vsi (series 1) | 41070 KiB | bioformats | ok | 1.417 | 0.760 | 265.1 |
| tests/external/olympus_01.vsi (series 1) | 41070 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/external/histo_01.vsi (series 1) | 36324 KiB | scopeio | ok | 0.017 | 0.018 | 38.0 |
| tests/external/histo_01.vsi (series 1) | 36324 KiB | bioformats | ok | 2.582 | 1.730 | 413.9 |
| tests/external/histo_01.vsi (series 1) | 36324 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/external/olympus_02.vsi (series 0) crop 8192,8192,1024,1024 | 144 KiB | scopeio | ok | 0.091 | 0.092 | 9.2 |
| tests/external/olympus_02.vsi (series 0) crop 8192,8192,1024,1024 | 144 KiB | bioformats | ok | 8.943 | 1.756 | 578.1 |
| tests/external/olympus_02.vsi (series 0) crop 8192,8192,1024,1024 | 144 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/external/whole_slide_olympus_01.vsi (series 13) crop 4096,12288,1024,1024 | 923 KiB | scopeio | ok | 0.761 | 0.762 | 10.4 |
| tests/external/whole_slide_olympus_01.vsi (series 13) crop 4096,12288,1024,1024 | 923 KiB | bioformats | ok | 17.856 | 4.572 | 736.7 |
| tests/external/whole_slide_olympus_01.vsi (series 13) crop 4096,12288,1024,1024 | 923 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/external/olympus_01.vsi | 41070 KiB | scopeio | ok | 0.014 | 0.015 | 11.1 |
| tests/external/olympus_01.vsi | 41070 KiB | bioformats | ok | 1.621 | 0.835 | 278.2 |
| tests/external/olympus_01.vsi | 41070 KiB | bioformats-rs | ok | 0.030 | 0.024 | 50.7 |
| tests/external/histo_01.vsi | 36324 KiB | scopeio | ok | 0.019 | 0.020 | 38.0 |
| tests/external/histo_01.vsi | 36324 KiB | bioformats | ok | 2.297 | 1.370 | 408.8 |
| tests/external/histo_01.vsi | 36324 KiB | bioformats-rs | ok | 0.027 | 0.023 | 46.2 |

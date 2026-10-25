# operetta: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/PerkinElmer-Operetta/zenodo-7841360/Hoechst__2023-03-07T10_17_54-Measurement 2/Images/Index.idx.xml | 3 KiB | scopeio | ok | 0.015 | 0.016 | 13.0 |
| tests/reference/ome/PerkinElmer-Operetta/zenodo-7841360/Hoechst__2023-03-07T10_17_54-Measurement 2/Images/Index.idx.xml | 3 KiB | bioformats | ok | 0.953 | 0.519 | 119.1 |
| tests/reference/ome/PerkinElmer-Operetta/zenodo-7841360/Hoechst__2023-03-07T10_17_54-Measurement 2/Images/Index.idx.xml | 3 KiB | bioformats-rs | ok | 0.018 | 0.019 | 17.5 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 0) | 1766 KiB | scopeio | ok | 0.060 | 0.062 | 18.4 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 0) | 1766 KiB | bioformats | ok | 4.638 | 0.944 | 300.0 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 0) | 1766 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 1) | 1766 KiB | scopeio | ok | 0.059 | 0.060 | 18.2 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 1) | 1766 KiB | bioformats | ok | 4.666 | 0.973 | 296.5 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 1) | 1766 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 191) | 1766 KiB | scopeio | ok | 0.072 | 0.073 | 18.8 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 191) | 1766 KiB | bioformats | ok | 4.963 | 0.994 | 305.4 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 191) | 1766 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 383) | 1766 KiB | scopeio | ok | 0.066 | 0.068 | 18.9 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 383) | 1766 KiB | bioformats | ok | 4.598 | 0.916 | 291.7 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml (series 383) | 1766 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml | 1766 KiB | scopeio | ok | 15.146 | 15.159 | 18.7 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml | 1766 KiB | bioformats | ok | 6.830 | 2.092 | 464.1 |
| tests/reference/ome/PerkinElmer-Operetta/59548/59548__2013-04-03T09_16_35-Measurement1/Images/Index.ref.xml | 1766 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.023 | 0.025 | 20.8 |
| tests/reference/ome/PerkinElmer-Operetta/omer/006P_M3/006P__2017-08-19T12_42_59-Measurement 3/Images/Index.idx.xml | 5318 KiB | scopeio | ok | 25.468 | 25.484 | 45.4 |
| tests/reference/ome/PerkinElmer-Operetta/omer/006P_M3/006P__2017-08-19T12_42_59-Measurement 3/Images/Index.idx.xml | 5318 KiB | bioformats | ok | 72.316 | 66.988 | 527.2 |
| tests/reference/ome/PerkinElmer-Operetta/omer/006P_M3/006P__2017-08-19T12_42_59-Measurement 3/Images/Index.idx.xml | 5318 KiB | bioformats-rs | ok | 1.217 | 1.247 | 502.9 |

# flex: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Flex/idr0007/Plate1/001001000.flex | 15806 KiB | scopeio | ok | 0.013 | 0.014 | 7.4 |
| tests/reference/ome/Flex/idr0007/Plate1/001001000.flex | 15806 KiB | bioformats | ok | 1.463 | 0.700 | 253.0 |
| tests/reference/ome/Flex/idr0007/Plate1/001001000.flex | 15806 KiB | bioformats-rs | ok | 0.020 | 0.021 | 41.0 |
| tests/reference/ome/Flex/idr0007/Plate1/001003000.flex | 15806 KiB | scopeio | ok | 0.013 | 0.014 | 7.6 |
| tests/reference/ome/Flex/idr0007/Plate1/001003000.flex | 15806 KiB | bioformats | ok | 1.517 | 0.724 | 255.4 |
| tests/reference/ome/Flex/idr0007/Plate1/001003000.flex | 15806 KiB | bioformats-rs | ok | 0.021 | 0.022 | 40.9 |
| tests/reference/ome/Flex/idr0007/Plate1/001005000.flex | 15806 KiB | scopeio | ok | 0.014 | 0.015 | 7.6 |
| tests/reference/ome/Flex/idr0007/Plate1/001005000.flex | 15806 KiB | bioformats | ok | 1.544 | 0.728 | 254.2 |
| tests/reference/ome/Flex/idr0007/Plate1/001005000.flex | 15806 KiB | bioformats-rs | ok | 0.020 | 0.022 | 41.2 |
| tests/data/flex/well.flex | 7 KiB | scopeio | ok | 0.001 | 0.002 | 5.0 |
| tests/data/flex/well.flex | 7 KiB | bioformats | ok | 0.983 | 0.537 | 107.3 |
| tests/data/flex/well.flex | 7 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.3 |

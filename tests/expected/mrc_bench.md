# mrc: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/MRC/EMDB/EMD-2225/EMD-2225.map | 8193 KiB | scopeio | ok | 0.008 | 0.009 | 3.9 |
| tests/reference/ome/MRC/EMDB/EMD-2225/EMD-2225.map | 8193 KiB | bioformats | ok | 1.751 | 1.012 | 281.3 |
| tests/reference/ome/MRC/EMDB/EMD-2225/EMD-2225.map | 8193 KiB | bioformats-rs | ok | 0.015 | 0.016 | 26.1 |
| tests/data/mrc/f16.mrc | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/mrc/f16.mrc | 1 KiB | bioformats | ok | 1.102 | 0.575 | 105.6 |
| tests/data/mrc/f16.mrc | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.3 |
| tests/data/mrc/i16_be.mrc | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/data/mrc/i16_be.mrc | 2 KiB | bioformats | ok | 1.081 | 0.582 | 105.6 |
| tests/data/mrc/i16_be.mrc | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.4 |
| tests/data/mrc/i8.mrc | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.8 |
| tests/data/mrc/i8.mrc | 1 KiB | bioformats | ok | 1.126 | 0.608 | 104.7 |
| tests/data/mrc/i8.mrc | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.4 |
| tests/data/mrc/nibble.mrc | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/mrc/nibble.mrc | 1 KiB | bioformats | ok | 1.119 | 0.602 | 105.3 |
| tests/data/mrc/nibble.mrc | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: MRC pixel p | 0.004 | 0.012 | 11.9 |
| tests/data/mrc/u16_stack.mrcs | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/mrc/u16_stack.mrcs | 1 KiB | bioformats | ok | 1.169 | 0.620 | 106.0 |
| tests/data/mrc/u16_stack.mrcs | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.7 |
| tests/data/mrc/u8_old.st | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/mrc/u8_old.st | 1 KiB | bioformats | ok | 1.213 | 0.677 | 105.1 |
| tests/data/mrc/u8_old.st | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |

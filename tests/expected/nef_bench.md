# nef: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/rawpixls/929.nef | 18977 KiB | scopeio | ok | 0.032 | 0.034 | 27.9 |
| tests/reference/rawpixls/929.nef | 18977 KiB | bioformats | ok | 1.919 | 1.304 | 501.1 |
| tests/reference/rawpixls/929.nef | 18977 KiB | bioformats-rs | ok | 0.388 | 0.390 | 223.6 |
| tests/reference/rawpixls/924.nef | 14319 KiB | scopeio | error: Error: UnsupportedVariant("Nikon-compressed NEF raw d | 0.001 | 0.002 | 4.2 |
| tests/reference/rawpixls/924.nef | 14319 KiB | bioformats | ok | 3.638 | 2.971 | 658.0 |
| tests/reference/rawpixls/924.nef | 14319 KiB | bioformats-rs | ok | 1.506 | 1.509 | 223.6 |
| tests/reference/rawpixls/928.nef | 8919 KiB | scopeio | error: Error: UnsupportedVariant("Nikon-compressed NEF raw d | 0.001 | 0.002 | 4.1 |
| tests/reference/rawpixls/928.nef | 8919 KiB | bioformats | ok | 2.980 | 2.246 | 655.8 |
| tests/reference/rawpixls/928.nef | 8919 KiB | bioformats-rs | ok | 0.643 | 0.645 | 223.8 |
| tests/data/nef/raw12.nef | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/nef/raw12.nef | 0 KiB | bioformats | ok | 0.918 | 0.500 | 103.9 |
| tests/data/nef/raw12.nef | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.8 |

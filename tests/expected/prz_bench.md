# prz: CPU and RAM compared with the reference readers

Generated 2026-11-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-43-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/external/panta_rhei_01.prz | 8206 KiB | scopeio | ok | 0.006 | 0.007 | 13.8 |
| tests/external/panta_rhei_01.prz | 8206 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.106 | 0.554 | 169.6 |
| tests/external/panta_rhei_01.prz | 8206 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.016 | 0.018 | 25.2 |
| tests/data/prz/deflated.prz | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/data/prz/deflated.prz | 1 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.043 | 0.507 | 107.4 |
| tests/data/prz/deflated.prz | 1 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.004 | 13.2 |
| tests/data/prz/fortran.prz | 2 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/data/prz/fortran.prz | 2 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.031 | 0.500 | 107.8 |
| tests/data/prz/fortran.prz | 2 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.004 | 13.0 |
| tests/data/prz/plain.prz | 3 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/data/prz/plain.prz | 3 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.001 | 0.494 | 108.3 |
| tests/data/prz/plain.prz | 3 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.004 | 13.1 |
| tests/data/prz/tem.prz | 8 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/data/prz/tem.prz | 8 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.022 | 0.484 | 107.3 |
| tests/data/prz/tem.prz | 8 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.005 | 13.0 |
| tests/data/prz/zip64.prz | 0 KiB | scopeio | ok | 0.001 | 0.001 | 5.2 |
| tests/data/prz/zip64.prz | 0 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.755 | 0.427 | 102.4 |
| tests/data/prz/zip64.prz | 0 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.004 | 13.0 |

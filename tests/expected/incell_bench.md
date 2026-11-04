# incell: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/InCell2000/zenodo-14777242/Training_Demo_1.xdce | 183 KiB | scopeio | ok | 0.132 | 0.133 | 7.0 |
| tests/reference/ome/InCell2000/zenodo-14777242/Training_Demo_1.xdce | 183 KiB | bioformats | ok | 7.542 | 4.316 | 436.9 |
| tests/reference/ome/InCell2000/zenodo-14777242/Training_Demo_1.xdce | 183 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.008 | 12.1 |
| tests/reference/ome/InCell2000/zenodo-14769820/Dataset_Fail/Dataset3_RedoAcq_1.xdce | 496 KiB | scopeio | ok | 0.240 | 0.243 | 8.3 |
| tests/reference/ome/InCell2000/zenodo-14769820/Dataset_Fail/Dataset3_RedoAcq_1.xdce | 496 KiB | bioformats | ok | 10.462 | 4.659 | 468.4 |
| tests/reference/ome/InCell2000/zenodo-14769820/Dataset_Fail/Dataset3_RedoAcq_1.xdce | 496 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.003 | 0.005 | 12.5 |
| tests/data/incell/listed/plate.xdce | 7 KiB | scopeio | ok | 0.001 | 0.003 | 4.6 |
| tests/data/incell/listed/plate.xdce | 7 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.706 | 0.379 | 100.8 |
| tests/data/incell/listed/plate.xdce | 7 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.006 | 0.024 | 10.8 |
| tests/data/incell/named/plate.xdce | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/incell/named/plate.xdce | 1 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.686 | 0.382 | 100.8 |
| tests/data/incell/named/plate.xdce | 1 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.002 | 0.003 | 11.7 |

# xrm: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/zenodo-4432446/XCT_predeformation_XradiaVersa520.txm | 1974972 KiB | scopeio | ok | 0.246 | 0.251 | 10.7 |
| tests/reference/zenodo-4432446/XCT_predeformation_XradiaVersa520.txm | 1974972 KiB | bioformats | ok | 7.559 | 5.270 | 415.9 |
| tests/reference/zenodo-4432446/XCT_predeformation_XradiaVersa520.txm | 1974972 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Zeiss XRM/T | 2.105 | 2.136 | 1939.0 |
| tests/data/xrm/projections.txrm | 46 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/data/xrm/projections.txrm | 46 KiB | bioformats | ok | 1.398 | 0.631 | 183.8 |
| tests/data/xrm/projections.txrm | 46 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Zeiss XRM/T | 0.014 | 0.016 | 12.5 |
| tests/data/xrm/single.xrm | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/xrm/single.xrm | 8 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 1.080 | 0.512 | 109.6 |
| tests/data/xrm/single.xrm | 8 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Zeiss XRM/T | 0.003 | 0.004 | 12.4 |

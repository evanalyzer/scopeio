# metaxpress: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/MetaXpress/trim/idr0005/2011-04-19-plate-1.HTD | 1 KiB | scopeio | ok | 0.008 | 0.009 | 7.8 |
| tests/reference/ome/MetaXpress/trim/idr0005/2011-04-19-plate-1.HTD | 1 KiB | bioformats | ok | 1.296 | 0.564 | 169.2 |
| tests/reference/ome/MetaXpress/trim/idr0005/2011-04-19-plate-1.HTD | 1 KiB | bioformats-rs | ok | 0.008 | 0.015 | 17.8 |
| tests/reference/ome/MetaXpress/trim/idr0006/plate 11001.HTD | 1 KiB | scopeio | ok | 0.023 | 0.024 | 7.6 |
| tests/reference/ome/MetaXpress/trim/idr0006/plate 11001.HTD | 1 KiB | bioformats | ok | 2.008 | 0.858 | 330.9 |
| tests/reference/ome/MetaXpress/trim/idr0006/plate 11001.HTD | 1 KiB | bioformats-rs | ok | 0.008 | 0.009 | 23.5 |
| tests/reference/ome/MetaXpress/trim/idr0006/TimePoint_1/plate 11001.HTD | 1 KiB | scopeio | ok | 0.024 | 0.025 | 7.5 |
| tests/reference/ome/MetaXpress/trim/idr0006/TimePoint_1/plate 11001.HTD | 1 KiB | bioformats | ok | 2.252 | 0.885 | 357.3 |
| tests/reference/ome/MetaXpress/trim/idr0006/TimePoint_1/plate 11001.HTD | 1 KiB | bioformats-rs | ok | 0.008 | 0.010 | 23.2 |
| tests/reference/ome/MetaXpress/trim/idr0008/Act1_Plate10-SP-A8.HTD | 4 KiB | scopeio | ok | 0.005 | 0.006 | 5.5 |
| tests/reference/ome/MetaXpress/trim/idr0008/Act1_Plate10-SP-A8.HTD | 4 KiB | bioformats | ok | 1.517 | 0.643 | 153.4 |
| tests/reference/ome/MetaXpress/trim/idr0008/Act1_Plate10-SP-A8.HTD | 4 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.002 | 0.004 | 11.9 |
| tests/reference/ome/MetaXpress/trim/idr0081/BSF018292-1A.HTD | 3 KiB | scopeio | ok | 0.012 | 0.013 | 13.0 |
| tests/reference/ome/MetaXpress/trim/idr0081/BSF018292-1A.HTD | 3 KiB | bioformats | ok | 1.415 | 0.634 | 197.7 |
| tests/reference/ome/MetaXpress/trim/idr0081/BSF018292-1A.HTD | 3 KiB | bioformats-rs | ok | 0.019 | 0.020 | 44.1 |
| tests/data/metaxpress/plate/synth.HTD | 1 KiB | scopeio | ok | 0.002 | 0.003 | 4.8 |
| tests/data/metaxpress/plate/synth.HTD | 1 KiB | bioformats | ok | 1.151 | 0.556 | 127.9 |
| tests/data/metaxpress/plate/synth.HTD | 1 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |

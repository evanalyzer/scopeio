# oif: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Olympus-FluoView/imagesc-71616/20220824_4492_cord_dapi__iba568_60x.oib | 25392 KiB | scopeio | ok | 0.021 | 0.022 | 9.9 |
| tests/reference/ome/Olympus-FluoView/imagesc-71616/20220824_4492_cord_dapi__iba568_60x.oib | 25392 KiB | bioformats | ok | 5.120 | 1.304 | 636.5 |
| tests/reference/ome/Olympus-FluoView/imagesc-71616/20220824_4492_cord_dapi__iba568_60x.oib | 25392 KiB | bioformats-rs | ok | 0.139 | 0.141 | 62.8 |
| tests/data/oif/synthetic.oib | 31 KiB | scopeio | ok | 0.001 | 0.002 | 3.7 |
| tests/data/oif/synthetic.oib | 31 KiB | bioformats | error: java.lang.NumberFormatException: Cannot parse null st | 0.755 | 0.415 | 102.2 |
| tests/data/oif/synthetic.oib | 31 KiB | bioformats-rs | ok | 0.017 | 0.018 | 13.3 |
| tests/data/oif/synthetic.oif | 6 KiB | scopeio | ok | 0.001 | 0.002 | 3.7 |
| tests/data/oif/synthetic.oif | 6 KiB | bioformats | error: java.lang.NumberFormatException: Cannot parse null st | 0.717 | 0.402 | 101.6 |
| tests/data/oif/synthetic.oif | 6 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |

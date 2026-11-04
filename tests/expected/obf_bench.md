# obf: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/OBF/ngladitz/v4/test-v4-uncompressed.obf | 36394 KiB | scopeio | ok | 0.016 | 0.018 | 4.6 |
| tests/reference/ome/OBF/ngladitz/v4/test-v4-uncompressed.obf | 36394 KiB | bioformats | ok | 3.358 | 1.228 | 328.4 |
| tests/reference/ome/OBF/ngladitz/v4/test-v4-uncompressed.obf | 36394 KiB | bioformats-rs | ok | 0.043 | 0.044 | 64.6 |
| tests/reference/ome/OBF/ngladitz/v4/test-v4.obf | 9233 KiB | scopeio | ok | 0.088 | 0.091 | 69.1 |
| tests/reference/ome/OBF/ngladitz/v4/test-v4.obf | 9233 KiB | bioformats | ok | 4.266 | 1.599 | 405.2 |
| tests/reference/ome/OBF/ngladitz/v4/test-v4.obf | 9233 KiB | bioformats-rs | ok | 0.115 | 0.117 | 105.3 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-short-write.obf | 212 KiB | scopeio | ok | 0.006 | 0.008 | 7.2 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-short-write.obf | 212 KiB | bioformats | ok | 2.980 | 1.234 | 387.8 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-short-write.obf | 212 KiB | bioformats-rs | ok | 0.013 | 0.015 | 25.9 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-uncompressed.obf | 37157 KiB | scopeio | ok | 0.025 | 0.026 | 5.9 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-uncompressed.obf | 37157 KiB | bioformats | ok | 3.694 | 1.372 | 368.8 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6-uncompressed.obf | 37157 KiB | bioformats-rs | ok | 0.057 | 0.059 | 101.1 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6.obf | 9245 KiB | scopeio | ok | 0.084 | 0.087 | 68.9 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6.obf | 9245 KiB | bioformats | ok | 4.139 | 1.513 | 379.1 |
| tests/reference/ome/OBF/ngladitz/v6/test-v6.obf | 9245 KiB | bioformats-rs | ok | 0.114 | 0.117 | 105.9 |
| tests/reference/ome/Imspector/zenodo-10476252/test.msr | 268949 KiB | scopeio | error: Error: UnsupportedFormat("/workspaces/scopeio/tests/r | 0.001 | 0.002 | 3.9 |
| tests/reference/ome/Imspector/zenodo-10476252/test.msr | 268949 KiB | bioformats | ok | 6.913 | 2.451 | 585.1 |
| tests/reference/ome/Imspector/zenodo-10476252/test.msr | 268949 KiB | bioformats-rs | ok | 0.589 | 0.602 | 1063.0 |
| tests/data/obf/two.obf | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/data/obf/two.obf | 4 KiB | bioformats | error: java.io.EOFException: Attempting to read beyond end o | 1.083 | 0.518 | 107.7 |
| tests/data/obf/two.obf | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.0 |

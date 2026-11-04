# cv7000: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/CV7000/trim/idr0088/110000251230.wpi | 0 KiB | scopeio | ok | 0.009 | 0.010 | 7.4 |
| tests/reference/ome/CV7000/trim/idr0088/110000251230.wpi | 0 KiB | bioformats | ok | 1.352 | 0.619 | 167.1 |
| tests/reference/ome/CV7000/trim/idr0088/110000251230.wpi | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: AFI file lists no | 0.003 | 0.004 | 12.3 |
| tests/reference/ome/CV7000/trim/idr0093/020.wpi | 0 KiB | scopeio | ok | 0.067 | 0.069 | 15.4 |
| tests/reference/ome/CV7000/trim/idr0093/020.wpi | 0 KiB | bioformats | ok | 1.495 | 0.737 | 288.8 |
| tests/reference/ome/CV7000/trim/idr0093/020.wpi | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: AFI file lists no | 0.002 | 0.003 | 12.0 |
| tests/reference/ome/CV7000/trim/cpg0016/Dest210531-152149.wpi | 0 KiB | scopeio | ok | 0.038 | 0.040 | 6.7 |
| tests/reference/ome/CV7000/trim/cpg0016/Dest210531-152149.wpi | 0 KiB | bioformats | ok | 1.943 | 1.107 | 328.7 |
| tests/reference/ome/CV7000/trim/cpg0016/Dest210531-152149.wpi | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: AFI file lists no | 0.003 | 0.005 | 12.3 |
| tests/data/cv7000/plate/synth.wpi | 0 KiB | scopeio | ok | 0.002 | 0.002 | 5.2 |
| tests/data/cv7000/plate/synth.wpi | 0 KiB | bioformats | ok | 1.155 | 0.518 | 127.9 |
| tests/data/cv7000/plate/synth.wpi | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: AFI file lists no | 0.003 | 0.004 | 12.0 |

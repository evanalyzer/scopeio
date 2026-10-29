# analyze: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/analyze/be_i16.hdr | 0 KiB | scopeio | ok | 0.001 | 0.001 | 4.2 |
| tests/data/analyze/be_i16.hdr | 0 KiB | bioformats | ok | 1.026 | 0.527 | 104.6 |
| tests/data/analyze/be_i16.hdr | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.9 |
| tests/data/analyze/le_u8_4d.hdr | 0 KiB | scopeio | ok | 0.001 | 0.001 | 4.1 |
| tests/data/analyze/le_u8_4d.hdr | 0 KiB | bioformats | ok | 0.960 | 0.500 | 106.9 |
| tests/data/analyze/le_u8_4d.hdr | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.8 |
| tests/data/analyze/f32.hdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/analyze/f32.hdr | 0 KiB | bioformats | ok | 0.970 | 0.533 | 104.5 |
| tests/data/analyze/f32.hdr | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.9 |
| tests/data/analyze/rgb.hdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/analyze/rgb.hdr | 0 KiB | bioformats | error: loci.formats.FormatException: Unsupported data type:  | 0.783 | 0.430 | 101.0 |
| tests/data/analyze/rgb.hdr | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 12.1 |

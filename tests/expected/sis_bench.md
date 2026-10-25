# sis: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/figshare/19467233/J6739 EM11gB18 IW73384.TIF | 9555 KiB | scopeio | ok | 0.007 | 0.009 | 13.1 |
| tests/reference/figshare/19467233/J6739 EM11gB18 IW73384.TIF | 9555 KiB | bioformats | ok | 2.351 | 1.494 | 198.1 |
| tests/reference/figshare/19467233/J6739 EM11gB18 IW73384.TIF | 9555 KiB | bioformats-rs | ok | 0.027 | 0.029 | 28.6 |
| tests/reference/figshare/19467287/J6760 EM11gB10 IW73562.TIF | 9553 KiB | scopeio | ok | 0.007 | 0.008 | 13.2 |
| tests/reference/figshare/19467287/J6760 EM11gB10 IW73562.TIF | 9553 KiB | bioformats | ok | 1.609 | 0.830 | 196.8 |
| tests/reference/figshare/19467287/J6760 EM11gB10 IW73562.TIF | 9553 KiB | bioformats-rs | ok | 0.022 | 0.023 | 28.5 |
| tests/reference/figshare/19640952/J6772 EM11gC3 IW73739.TIF | 9555 KiB | scopeio | ok | 0.006 | 0.007 | 13.4 |
| tests/reference/figshare/19640952/J6772 EM11gC3 IW73739.TIF | 9555 KiB | bioformats | ok | 1.368 | 0.733 | 198.3 |
| tests/reference/figshare/19640952/J6772 EM11gC3 IW73739.TIF | 9555 KiB | bioformats-rs | ok | 0.021 | 0.022 | 28.7 |
| tests/data/sis/palette.tif | 2 KiB | scopeio | ok | 0.001 | 0.005 | 4.0 |
| tests/data/sis/palette.tif | 2 KiB | bioformats | ok | 2.546 | 1.988 | 105.9 |
| tests/data/sis/palette.tif | 2 KiB | bioformats-rs | ok | 0.006 | 0.011 | 10.0 |

# gatan: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0000.dm4 | 4698 KiB | scopeio | ok | 0.004 | 0.005 | 8.4 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0000.dm4 | 4698 KiB | bioformats | ok | 1.779 | 0.661 | 163.6 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0000.dm4 | 4698 KiB | bioformats-rs | ok | 0.008 | 0.010 | 18.7 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0001.dm4 | 4699 KiB | scopeio | ok | 0.004 | 0.005 | 8.4 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0001.dm4 | 4699 KiB | bioformats | ok | 1.707 | 0.657 | 162.2 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0001.dm4 | 4699 KiB | bioformats-rs | ok | 0.007 | 0.009 | 18.6 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0002.dm4 | 4699 KiB | scopeio | ok | 0.004 | 0.005 | 8.3 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0002.dm4 | 4699 KiB | bioformats | ok | 1.720 | 0.677 | 162.9 |
| tests/reference/ome/Gatan/imagesc-36590/SmallMontage0002.dm4 | 4699 KiB | bioformats-rs | ok | 0.008 | 0.009 | 18.6 |
| tests/reference/ome/Gatan/imagesc-36590/191041a_ctl_M2_3VBSED_0045.dm4 | 195913 KiB | scopeio | ok | 0.095 | 0.097 | 195.0 |
| tests/reference/ome/Gatan/imagesc-36590/191041a_ctl_M2_3VBSED_0045.dm4 | 195913 KiB | bioformats | ok | 2.514 | 1.346 | 716.9 |
| tests/reference/ome/Gatan/imagesc-36590/191041a_ctl_M2_3VBSED_0045.dm4 | 195913 KiB | bioformats-rs | ok | 0.207 | 0.208 | 392.3 |
| tests/data/gatan/stack.dm3 | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/data/gatan/stack.dm3 | 2 KiB | bioformats | ok | 1.036 | 0.559 | 104.4 |
| tests/data/gatan/stack.dm3 | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/data/gatan/rgba.dm4 | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.3 |
| tests/data/gatan/rgba.dm4 | 2 KiB | bioformats | ok | 0.947 | 0.463 | 103.7 |
| tests/data/gatan/rgba.dm4 | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |

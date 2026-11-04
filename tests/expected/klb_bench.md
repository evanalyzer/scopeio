# klb: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/KLB/samples/img.klb | 373 KiB | scopeio | ok | 0.017 | 0.018 | 6.0 |
| tests/reference/ome/KLB/samples/img.klb | 373 KiB | bioformats | ok | 1.555 | 0.692 | 161.6 |
| tests/reference/ome/KLB/samples/img.klb | 373 KiB | bioformats-rs | ok | 0.132 | 0.134 | 10.5 |
| tests/reference/ome/KLB/single/stack_a.klb | 3367 KiB | scopeio | ok | 52.754 | 52.782 | 85.5 |
| tests/reference/ome/KLB/single/stack_a.klb | 3367 KiB | bioformats | ok | 305.544 | 303.493 | 742.4 |
| tests/reference/ome/KLB/single/stack_a.klb | 3367 KiB | bioformats-rs | error: killed: exceeded the RSS limit of 8192 MiB | 129.992 | 130.083 | 8192.8 |
| tests/reference/ome/KLB/single/stack_b.klb | 3367 KiB | scopeio | ok | 52.306 | 52.336 | 85.7 |
| tests/reference/ome/KLB/single/stack_b.klb | 3367 KiB | bioformats | ok | 306.141 | 303.920 | 743.7 |
| tests/reference/ome/KLB/single/stack_b.klb | 3367 KiB | bioformats-rs | error: killed: exceeded the RSS limit of 8192 MiB | 128.038 | 128.091 | 8192.7 |
| tests/reference/ome/KLB/single/projection.klb | 27 KiB | scopeio | ok | 0.014 | 0.015 | 21.7 |
| tests/reference/ome/KLB/single/projection.klb | 27 KiB | bioformats | ok | 0.993 | 0.491 | 169.1 |
| tests/reference/ome/KLB/single/projection.klb | 27 KiB | bioformats-rs | ok | 0.030 | 0.031 | 26.1 |
| tests/data/klb/stack.klb | 3 KiB | scopeio | ok | 0.001 | 0.002 | 5.0 |
| tests/data/klb/stack.klb | 3 KiB | bioformats | ok | 0.772 | 0.428 | 120.9 |
| tests/data/klb/stack.klb | 3 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/data/klb/plain.klb | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/klb/plain.klb | 0 KiB | bioformats | ok | 0.776 | 0.440 | 108.2 |
| tests/data/klb/plain.klb | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 8.9 |
| tests/data/klb/int8.klb | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/klb/int8.klb | 0 KiB | bioformats | ok | 0.755 | 0.419 | 109.0 |
| tests/data/klb/int8.klb | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 9.6 |

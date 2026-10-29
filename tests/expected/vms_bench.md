# vms: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- The macro image (series 1) is not timed for scopeio: the script would time a read of the whole 23 GB level 0 instead. The map (series 2) was converted by Bio-Formats with an 8 GB heap (`BF_MAX_MEM=8g`); with the default heap it runs out of memory.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 60928,30000,1024,1024 | 1 KiB | scopeio | ok | 0.489 | 0.491 | 16.9 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 60928,30000,1024,1024 | 1 KiB | bioformats | ok | 3.695 | 2.904 | 342.8 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 60928,30000,1024,1024 | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 40000,60928,2048,1024 | 1 KiB | scopeio | ok | 0.450 | 0.451 | 19.7 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 40000,60928,2048,1024 | 1 KiB | bioformats | ok | 2.332 | 1.787 | 339.1 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 40000,60928,2048,1024 | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 101376,75264,1024,1024 | 1 KiB | scopeio | ok | 0.053 | 0.054 | 15.9 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 101376,75264,1024,1024 | 1 KiB | bioformats | ok | 1.387 | 0.810 | 170.4 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 0) crop 101376,75264,1024,1024 | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 1) | 1 KiB | bioformats | ok | 1.659 | 0.837 | 124.2 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 1) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 2) | 1 KiB | scopeio | ok | 1.358 | 1.362 | 362.1 |
| tests/reference/openslide/Hamamatsu-vms/CMU-1/CMU-1-40x - 2010-01-12 13.24.05.vms (series 2) | 1 KiB | bioformats | ok | 7.538 | 6.571 | 1595.9 |

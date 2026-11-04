# nrrd: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/NRRD/glencoe/gzip/dt-helix.nrrd.gz | 1009 KiB | scopeio | ok | 0.006 | 0.008 | 7.3 |
| tests/reference/ome/NRRD/glencoe/gzip/dt-helix.nrrd.gz | 1009 KiB | bioformats | ok | 1.464 | 0.718 | 155.2 |
| tests/reference/ome/NRRD/glencoe/gzip/dt-helix.nrrd.gz | 1009 KiB | bioformats-rs | ok | 0.143 | 0.146 | 16.3 |
| tests/reference/ome/NRRD/glencoe/version4/dt-helix.nhdr | 0 KiB | scopeio | ok | 0.009 | 0.011 | 4.8 |
| tests/reference/ome/NRRD/glencoe/version4/dt-helix.nhdr | 0 KiB | bioformats | ok | 1.472 | 0.699 | 151.4 |
| tests/reference/ome/NRRD/glencoe/version4/dt-helix.nhdr | 0 KiB | bioformats-rs | ok | 0.030 | 0.032 | 16.3 |
| tests/reference/ome/NRRD/gordon/dt-helix.nhdr | 0 KiB | scopeio | ok | 0.008 | 0.010 | 4.8 |
| tests/reference/ome/NRRD/gordon/dt-helix.nhdr | 0 KiB | bioformats | ok | 2.737 | 1.717 | 150.6 |
| tests/reference/ome/NRRD/gordon/dt-helix.nhdr | 0 KiB | bioformats-rs | ok | 0.026 | 0.028 | 16.0 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask.nhdr | 1 KiB | scopeio | ok | 0.333 | 0.337 | 4.7 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask.nhdr | 1 KiB | bioformats | ok | 2.307 | 1.523 | 559.4 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask.nhdr | 1 KiB | bioformats-rs | ok | 15.309 | 15.314 | 370.7 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask2.nhdr | 1 KiB | scopeio | ok | 0.188 | 0.189 | 4.5 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask2.nhdr | 1 KiB | bioformats | ok | 5.020 | 3.885 | 646.4 |
| tests/reference/ome/NRRD/gordon/gk2-rcc-mask2.nhdr | 1 KiB | bioformats-rs | ok | 25.739 | 25.760 | 371.5 |
| tests/data/nrrd/scalar.nrrd | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/nrrd/scalar.nrrd | 0 KiB | bioformats | ok | 0.880 | 0.435 | 117.8 |
| tests/data/nrrd/scalar.nrrd | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.7 |
| tests/data/nrrd/rgb.nhdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/nrrd/rgb.nhdr | 0 KiB | bioformats | ok | 0.969 | 0.513 | 272.6 |
| tests/data/nrrd/rgb.nhdr | 0 KiB | bioformats-rs | ok | 0.002 | 0.004 | 11.8 |
| tests/data/nrrd/text.nrrd | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/nrrd/text.nrrd | 1 KiB | bioformats | error: loci.formats.UnsupportedCompressionException: Unsuppo | 0.869 | 0.446 | 114.9 |
| tests/data/nrrd/text.nrrd | 1 KiB | bioformats-rs | error: Error reading plane 0: Invalid data: NRRD: malformed  | 0.003 | 0.004 | 11.4 |
| tests/data/nrrd/hex.nhdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/nrrd/hex.nhdr | 0 KiB | bioformats | ok | 0.872 | 0.458 | 116.5 |
| tests/data/nrrd/hex.nhdr | 0 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: NRRD: unsu | 0.003 | 0.004 | 11.7 |
| tests/data/nrrd/tail.nhdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/nrrd/tail.nhdr | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: Negative position | 0.841 | 0.450 | 114.9 |
| tests/data/nrrd/tail.nhdr | 0 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: NRRD byte  | 0.003 | 0.004 | 11.5 |

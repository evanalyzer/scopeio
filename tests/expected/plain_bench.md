# plain: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/plain/gray1.png | 0 KiB | scopeio | ok | 0.001 | 0.001 | 3.6 |
| tests/data/plain/gray1.png | 0 KiB | bioformats | ok | 0.830 | 0.461 | 103.5 |
| tests/data/plain/gray1.png | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.5 |
| tests/data/plain/gray16.png | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.6 |
| tests/data/plain/gray16.png | 1 KiB | bioformats | ok | 0.847 | 0.473 | 103.7 |
| tests/data/plain/gray16.png | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/data/plain/gray2_interlaced.png | 0 KiB | scopeio | ok | 0.001 | 0.002 | 3.6 |
| tests/data/plain/gray2_interlaced.png | 0 KiB | bioformats | error: loci.formats.FormatException: Unknown filter: -86 | 0.846 | 0.467 | 103.7 |
| tests/data/plain/gray2_interlaced.png | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.5 |
| tests/data/plain/gray8.png | 0 KiB | scopeio | ok | 0.001 | 0.002 | 3.4 |
| tests/data/plain/gray8.png | 0 KiB | bioformats | ok | 0.846 | 0.479 | 103.4 |
| tests/data/plain/gray8.png | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.0 |
| tests/data/plain/graya8.png | 0 KiB | scopeio | ok | 0.001 | 0.001 | 3.6 |
| tests/data/plain/graya8.png | 0 KiB | bioformats | ok | 0.847 | 0.471 | 103.7 |
| tests/data/plain/graya8.png | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/data/plain/palette.png | 0 KiB | scopeio | ok | 0.001 | 0.001 | 3.6 |
| tests/data/plain/palette.png | 0 KiB | bioformats | ok | 0.840 | 0.472 | 103.7 |
| tests/data/plain/palette.png | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.6 |
| tests/data/plain/rgb16.png | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.4 |
| tests/data/plain/rgb16.png | 1 KiB | bioformats | ok | 0.854 | 0.485 | 103.8 |
| tests/data/plain/rgb16.png | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.9 |
| tests/data/plain/rgb8.png | 0 KiB | scopeio | ok | 0.001 | 0.002 | 3.6 |
| tests/data/plain/rgb8.png | 0 KiB | bioformats | ok | 0.848 | 0.479 | 103.9 |
| tests/data/plain/rgb8.png | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.9 |
| tests/data/plain/rgb8_interlaced.png | 1 KiB | scopeio | ok | 0.001 | 0.001 | 3.5 |
| tests/data/plain/rgb8_interlaced.png | 1 KiB | bioformats | ok | 0.833 | 0.476 | 103.9 |
| tests/data/plain/rgb8_interlaced.png | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.0 |
| tests/data/plain/rgba8.png | 0 KiB | scopeio | ok | 0.001 | 0.002 | 3.5 |
| tests/data/plain/rgba8.png | 0 KiB | bioformats | ok | 0.850 | 0.488 | 103.5 |
| tests/data/plain/rgba8.png | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/data/plain/gray8.jpg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.3 |
| tests/data/plain/gray8.jpg | 1 KiB | bioformats | ok | 0.981 | 0.575 | 106.7 |
| tests/data/plain/gray8.jpg | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.8 |
| tests/data/plain/rgb8.jpg | 2 KiB | scopeio | ok | 0.001 | 0.002 | 3.4 |
| tests/data/plain/rgb8.jpg | 2 KiB | bioformats | ok | 0.904 | 0.506 | 107.3 |
| tests/data/plain/rgb8.jpg | 2 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.1 |
| tests/data/plain/rgb8.jp2 | 2 KiB | scopeio | ok | 0.001 | 0.002 | 3.7 |
| tests/data/plain/rgb8.jp2 | 2 KiB | bioformats | ok | 1.014 | 0.550 | 107.6 |
| tests/data/plain/rgb8.jp2 | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/data/plain/gray16.j2k | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.7 |
| tests/data/plain/gray16.j2k | 1 KiB | bioformats | ok | 1.057 | 0.575 | 108.2 |
| tests/data/plain/gray16.j2k | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/reference/plain/photo_rgb8.png | 21672 KiB | scopeio | ok | 0.312 | 0.314 | 72.2 |
| tests/reference/plain/photo_rgb8.png | 21672 KiB | bioformats | ok | 1.953 | 1.520 | 353.9 |
| tests/reference/plain/photo_rgb8.png | 21672 KiB | bioformats-rs | ok | 0.568 | 0.574 | 216.4 |
| tests/reference/plain/photo_rgb8.jpg | 4272 KiB | scopeio | ok | 0.292 | 0.294 | 145.2 |
| tests/reference/plain/photo_rgb8.jpg | 4272 KiB | bioformats | ok | 3.892 | 3.110 | 600.7 |
| tests/reference/plain/photo_rgb8.jpg | 4272 KiB | bioformats-rs | ok | 0.504 | 0.507 | 219.8 |
| tests/reference/plain/photo_rgb8.jp2 | 19345 KiB | scopeio | ok | 4.931 | 4.934 | 577.2 |
| tests/reference/plain/photo_rgb8.jp2 | 19345 KiB | bioformats | ok | 14.492 | 11.033 | 715.6 |
| tests/reference/plain/photo_rgb8.jp2 | 19345 KiB | bioformats-rs | ok | 4.568 | 4.579 | 401.3 |
| tests/reference/plain/gray16.png | 27568 KiB | scopeio | ok | 0.105 | 0.107 | 35.5 |
| tests/reference/plain/gray16.png | 27568 KiB | bioformats | ok | 1.561 | 1.063 | 255.7 |
| tests/reference/plain/gray16.png | 27568 KiB | bioformats-rs | ok | 0.218 | 0.219 | 117.5 |

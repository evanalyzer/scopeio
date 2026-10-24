# bmp: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 5 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/bmp/core_pal8.bmp | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/core_pal8.bmp | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.731 | 0.422 | 101.9 |
| tests/data/bmp/core_pal8.bmp | 1 KiB | bioformats-rs | error: Error opening <file>: Invalid data: BMP: unsupported  | 0.002 | 0.003 | 14.3 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | bioformats | ok | 0.882 | 0.493 | 104.7 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | bioformats | ok | 0.869 | 0.474 | 104.2 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.1 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | bioformats | ok | 0.880 | 0.510 | 103.9 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.8 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 1.9 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | bioformats | ok | 0.889 | 0.495 | 104.5 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | bioformats | ok | 0.899 | 0.497 | 104.2 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.1 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.736 | 0.437 | 102.3 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.8 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.757 | 0.433 | 101.5 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.8 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 1.9 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.744 | 0.454 | 101.7 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | bioformats | ok | 0.928 | 0.508 | 104.9 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | bioformats | ok | 0.915 | 0.514 | 104.9 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | bioformats | ok | 0.895 | 0.506 | 104.3 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.8 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | bioformats | ok | 0.893 | 0.508 | 104.4 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/rle4.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rle4.bmp | 0 KiB | bioformats | error: java.lang.ArrayIndexOutOfBoundsException: Index 16 ou | 0.891 | 0.504 | 104.4 |
| tests/data/bmp/rle4.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.8 |
| tests/data/bmp/rle8.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.0 |
| tests/data/bmp/rle8.bmp | 0 KiB | bioformats | ok | 0.885 | 0.502 | 103.8 |
| tests/data/bmp/rle8.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | scopeio | ok | 0.002 | 0.003 | 4.7 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | bioformats | ok | 1.055 | 0.573 | 129.7 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | bioformats-rs | ok | 0.008 | 0.009 | 18.2 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | scopeio | ok | 0.005 | 0.006 | 8.3 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | bioformats | ok | 1.024 | 0.581 | 124.9 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | bioformats-rs | ok | 0.014 | 0.015 | 28.9 |

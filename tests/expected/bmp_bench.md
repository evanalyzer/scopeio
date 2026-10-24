# bmp: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/bmp/core_pal8.bmp | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.4 |
| tests/data/bmp/core_pal8.bmp | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.815 | 0.481 | 101.4 |
| tests/data/bmp/core_pal8.bmp | 1 KiB | bioformats-rs | error: Error opening <file>: Invalid data: BMP: unsupported  | 0.002 | 0.003 | 14.1 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.3 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | bioformats | ok | 0.900 | 0.516 | 105.3 |
| tests/data/bmp/pal1_color.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.5 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | bioformats | ok | 0.925 | 0.512 | 104.0 |
| tests/data/bmp/pal4_color.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.3 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | bioformats | ok | 0.913 | 0.509 | 104.0 |
| tests/data/bmp/pal4_gray.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.4 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | bioformats | ok | 0.932 | 0.532 | 104.9 |
| tests/data/bmp/pal8_color_clrused.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.4 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | bioformats | ok | 0.890 | 0.520 | 104.4 |
| tests/data/bmp/pal8_gray_topdown.bmp | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.763 | 0.446 | 101.1 |
| tests/data/bmp/rgb16_555.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.785 | 0.460 | 102.8 |
| tests/data/bmp/rgb16_565.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.753 | 0.435 | 101.9 |
| tests/data/bmp/rgb16_565_v4.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.4 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | bioformats | ok | 0.868 | 0.499 | 104.3 |
| tests/data/bmp/rgb24_odd_width.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | bioformats | ok | 0.866 | 0.487 | 104.1 |
| tests/data/bmp/rgb32_bitfields_swapped.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | bioformats | ok | 0.917 | 0.528 | 104.4 |
| tests/data/bmp/rgb32_unused_byte.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 13.0 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | bioformats | ok | 0.889 | 0.513 | 104.9 |
| tests/data/bmp/rgba32_v5.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/data/bmp/rle4.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.4 |
| tests/data/bmp/rle4.bmp | 0 KiB | bioformats | error: java.lang.ArrayIndexOutOfBoundsException: Index 16 ou | 0.901 | 0.500 | 104.6 |
| tests/data/bmp/rle4.bmp | 0 KiB | bioformats-rs | ok | 0.001 | 0.002 | 12.8 |
| tests/data/bmp/rle8.bmp | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.5 |
| tests/data/bmp/rle8.bmp | 0 KiB | bioformats | ok | 0.895 | 0.511 | 104.4 |
| tests/data/bmp/rle8.bmp | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.9 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | scopeio | ok | 0.002 | 0.003 | 5.4 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | bioformats | ok | 1.008 | 0.571 | 129.4 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/mono 8bit.bmp | 1876 KiB | bioformats-rs | ok | 0.008 | 0.008 | 18.2 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | scopeio | ok | 0.005 | 0.006 | 8.9 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | bioformats | ok | 1.011 | 0.575 | 125.8 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/rgb 8bit.bmp | 5625 KiB | bioformats-rs | ok | 0.014 | 0.015 | 29.0 |

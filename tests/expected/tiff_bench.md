# tiff: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/tiff/bigtiff_rgb8_be.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/bigtiff_rgb8_be.tif | 1 KiB | bioformats | ok | 0.980 | 0.539 | 104.9 |
| tests/data/tiff/bigtiff_rgb8_be.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/bigtiff_u16_tiles.tif | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/bigtiff_u16_tiles.tif | 3 KiB | bioformats | ok | 1.355 | 0.609 | 117.5 |
| tests/data/tiff/bigtiff_u16_tiles.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.7 |
| tests/data/tiff/f16_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/f16_le.tif | 0 KiB | bioformats | ok | 0.938 | 0.510 | 105.3 |
| tests/data/tiff/f16_le.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/data/tiff/f32_deflate_fpred.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/data/tiff/f32_deflate_fpred.tif | 1 KiB | bioformats | error: loci.formats.FormatException: Unknown Predictor (3) | 0.928 | 0.507 | 104.9 |
| tests/data/tiff/f32_deflate_fpred.tif | 1 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: TIFF predi | 0.003 | 0.004 | 11.6 |
| tests/data/tiff/f32_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.5 |
| tests/data/tiff/f32_le.tif | 0 KiB | bioformats | ok | 0.920 | 0.514 | 105.5 |
| tests/data/tiff/f32_le.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/f64_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/f64_be.tif | 0 KiB | bioformats | error: loci.formats.FormatException: Unsupported image type  | 0.932 | 0.515 | 105.0 |
| tests/data/tiff/f64_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/f64_deflate_fpred_rgb.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/f64_deflate_fpred_rgb.tif | 1 KiB | bioformats | error: loci.formats.FormatException: Unknown Predictor (3) | 0.967 | 0.541 | 105.5 |
| tests/data/tiff/f64_deflate_fpred_rgb.tif | 1 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: TIFF predi | 0.003 | 0.004 | 11.4 |
| tests/data/tiff/gray2_chunky.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/gray2_chunky.tif | 0 KiB | bioformats | ok | 0.946 | 0.512 | 104.8 |
| tests/data/tiff/gray2_chunky.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/gray3_planar.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/gray3_planar.tif | 0 KiB | bioformats | ok | 0.981 | 0.521 | 105.5 |
| tests/data/tiff/gray3_planar.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/i16_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/i16_le.tif | 0 KiB | bioformats | ok | 0.954 | 0.525 | 105.7 |
| tests/data/tiff/i16_le.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.2 |
| tests/data/tiff/i32_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/i32_le.tif | 0 KiB | bioformats | ok | 0.964 | 0.520 | 105.6 |
| tests/data/tiff/i32_le.tif | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.3 |
| tests/data/tiff/i8_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/i8_be.tif | 0 KiB | bioformats | ok | 0.923 | 0.498 | 105.3 |
| tests/data/tiff/i8_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/imagej_hyperstack.tif | 4 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff/imagej_hyperstack.tif | 4 KiB | bioformats | ok | 1.514 | 0.683 | 125.7 |
| tests/data/tiff/imagej_hyperstack.tif | 4 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.4 |
| tests/data/tiff/pages3_u16.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/pages3_u16.tif | 1 KiB | bioformats | ok | 0.952 | 0.521 | 106.2 |
| tests/data/tiff/pages3_u16.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/palette4_gray.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/palette4_gray.tif | 0 KiB | bioformats | ok | 0.952 | 0.513 | 105.6 |
| tests/data/tiff/palette4_gray.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/palette8.tif | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/palette8.tif | 2 KiB | bioformats | ok | 1.416 | 0.633 | 125.0 |
| tests/data/tiff/palette8.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/palette8_green.tif | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/palette8_green.tif | 2 KiB | bioformats | ok | 1.440 | 0.642 | 118.0 |
| tests/data/tiff/palette8_green.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/palette8_rgb16.tif | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/palette8_rgb16.tif | 2 KiB | bioformats | ok | 1.369 | 0.642 | 116.1 |
| tests/data/tiff/palette8_rgb16.tif | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.5 |
| tests/data/tiff/pyramid_reduced.tif | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/pyramid_reduced.tif | 2 KiB | bioformats | ok | 1.430 | 0.682 | 115.3 |
| tests/data/tiff/pyramid_reduced.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/data/tiff/pyramid_subifd.tif | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/pyramid_subifd.tif | 2 KiB | bioformats | ok | 1.387 | 0.650 | 119.9 |
| tests/data/tiff/pyramid_subifd.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/rgb16_planar_tiles_be.tif | 6 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/rgb16_planar_tiles_be.tif | 6 KiB | bioformats | ok | 1.389 | 0.639 | 117.7 |
| tests/data/tiff/rgb16_planar_tiles_be.tif | 6 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.3 |
| tests/data/tiff/rgb8_lzw_pred.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/rgb8_lzw_pred.tif | 1 KiB | bioformats | ok | 0.983 | 0.545 | 106.2 |
| tests/data/tiff/rgb8_lzw_pred.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/rgb8_planar.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/rgb8_planar.tif | 0 KiB | bioformats | ok | 0.935 | 0.514 | 104.9 |
| tests/data/tiff/rgb8_planar.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/rgb8_tiles.tif | 5 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/rgb8_tiles.tif | 5 KiB | bioformats | ok | 1.349 | 0.621 | 116.5 |
| tests/data/tiff/rgb8_tiles.tif | 5 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/rgba16_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/rgba16_be.tif | 0 KiB | bioformats | ok | 1.060 | 0.563 | 105.1 |
| tests/data/tiff/rgba16_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.2 |
| tests/data/tiff/same_rgb16_be_bigtiff_lzw.tif | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/same_rgb16_be_bigtiff_lzw.tif | 3 KiB | bioformats | ok | 1.402 | 0.673 | 117.2 |
| tests/data/tiff/same_rgb16_be_bigtiff_lzw.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.6 |
| tests/data/tiff/same_rgb16_be_planar.tif | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/same_rgb16_be_planar.tif | 3 KiB | bioformats | ok | 1.467 | 0.661 | 116.8 |
| tests/data/tiff/same_rgb16_be_planar.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/same_rgb16_le_chunky.tif | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/same_rgb16_le_chunky.tif | 3 KiB | bioformats | ok | 1.448 | 0.680 | 117.4 |
| tests/data/tiff/same_rgb16_le_chunky.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/same_rgb16_le_tiles_deflate_pred.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/data/tiff/same_rgb16_le_tiles_deflate_pred.tif | 1 KiB | bioformats | ok | 1.088 | 0.588 | 105.4 |
| tests/data/tiff/same_rgb16_le_tiles_deflate_pred.tif | 1 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/data/tiff/two_series.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/two_series.tif | 1 KiB | bioformats | ok | 1.106 | 0.608 | 105.0 |
| tests/data/tiff/two_series.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/u10_rgb_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u10_rgb_be.tif | 0 KiB | bioformats | ok | 1.060 | 0.594 | 105.9 |
| tests/data/tiff/u10_rgb_be.tif | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.3 |
| tests/data/tiff/u12_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/u12_le.tif | 0 KiB | bioformats | ok | 1.058 | 0.588 | 105.1 |
| tests/data/tiff/u12_le.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/u16_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u16_be.tif | 0 KiB | bioformats | ok | 1.073 | 0.610 | 105.8 |
| tests/data/tiff/u16_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/u16_deflate_pred.tif | 0 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff/u16_deflate_pred.tif | 0 KiB | bioformats | ok | 1.128 | 0.618 | 105.8 |
| tests/data/tiff/u16_deflate_pred.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/data/tiff/u16_deflate_pred_be_tiles.tif | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff/u16_deflate_pred_be_tiles.tif | 1 KiB | bioformats | ok | 1.107 | 0.595 | 105.2 |
| tests/data/tiff/u16_deflate_pred_be_tiles.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.9 |
| tests/data/tiff/u16_lzw_big_table.tif | 163 KiB | scopeio | ok | 0.002 | 0.002 | 3.3 |
| tests/data/tiff/u16_lzw_big_table.tif | 163 KiB | bioformats | ok | 1.621 | 0.752 | 128.6 |
| tests/data/tiff/u16_lzw_big_table.tif | 163 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/data/tiff/u1_miniswhite.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u1_miniswhite.tif | 0 KiB | bioformats | ok | 1.114 | 0.630 | 104.8 |
| tests/data/tiff/u1_miniswhite.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 12.3 |
| tests/data/tiff/u32_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u32_be.tif | 0 KiB | bioformats | ok | 1.078 | 0.590 | 105.0 |
| tests/data/tiff/u32_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/u4_be.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u4_be.tif | 0 KiB | bioformats | ok | 1.106 | 0.598 | 105.3 |
| tests/data/tiff/u4_be.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.4 |
| tests/data/tiff/u8_le.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/tiff/u8_le.tif | 0 KiB | bioformats | ok | 1.119 | 0.647 | 104.4 |
| tests/data/tiff/u8_le.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.3 |
| tests/data/tiff/u8_lzw.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/data/tiff/u8_lzw.tif | 1 KiB | bioformats | ok | 1.551 | 0.697 | 117.6 |
| tests/data/tiff/u8_lzw.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.6 |
| tests/data/tiff/u8_packbits.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff/u8_packbits.tif | 0 KiB | bioformats | ok | 1.119 | 0.624 | 105.0 |
| tests/data/tiff/u8_packbits.tif | 0 KiB | bioformats-rs | ok | 0.002 | 0.004 | 12.3 |
| tests/data/tiff_im/gray16_3pages.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/tiff_im/gray16_3pages.tif | 1 KiB | bioformats | ok | 1.595 | 0.761 | 118.2 |
| tests/data/tiff_im/gray16_3pages.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.7 |
| tests/data/tiff_im/gray16_bigtiff.tif | 8 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/tiff_im/gray16_bigtiff.tif | 8 KiB | bioformats | ok | 1.472 | 0.723 | 115.9 |
| tests/data/tiff_im/gray16_bigtiff.tif | 8 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/data/tiff_im/gray16_lzw_tiles_be.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff_im/gray16_lzw_tiles_be.tif | 2 KiB | bioformats | ok | 1.516 | 0.703 | 117.0 |
| tests/data/tiff_im/gray16_lzw_tiles_be.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.6 |
| tests/data/tiff_im/gray16_zip_pred.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff_im/gray16_zip_pred.tif | 0 KiB | bioformats | ok | 1.166 | 0.657 | 105.1 |
| tests/data/tiff_im/gray16_zip_pred.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.8 |
| tests/data/tiff_im/gray16_zstd.tif | 0 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/data/tiff_im/gray16_zstd.tif | 0 KiB | bioformats | error: java.lang.NegativeArraySizeException: -1 | 1.118 | 0.620 | 104.9 |
| tests/data/tiff_im/gray16_zstd.tif | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |
| tests/data/tiff_im/gray32f_zip_fpred.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff_im/gray32f_zip_fpred.tif | 1 KiB | bioformats | error: loci.formats.FormatException: Unknown Predictor (3) | 1.107 | 0.615 | 104.8 |
| tests/data/tiff_im/gray32f_zip_fpred.tif | 1 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: TIFF predi | 0.003 | 0.004 | 11.3 |
| tests/data/tiff_im/gray8_jpeg.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff_im/gray8_jpeg.tif | 1 KiB | bioformats | ok | 1.117 | 0.627 | 107.7 |
| tests/data/tiff_im/gray8_jpeg.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.3 |
| tests/data/tiff_im/rgb16_zip_planar.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff_im/rgb16_zip_planar.tif | 1 KiB | bioformats | ok | 1.531 | 0.694 | 119.6 |
| tests/data/tiff_im/rgb16_zip_planar.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/data/tiff_im/rgb8_jpeg.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/tiff_im/rgb8_jpeg.tif | 2 KiB | bioformats | ok | 1.657 | 0.779 | 120.9 |
| tests/data/tiff_im/rgb8_jpeg.tif | 2 KiB | bioformats-rs | ok | 0.008 | 0.005 | 13.2 |
| tests/data/tiff_im/rgb8_jpeg_rgb.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/tiff_im/rgb8_jpeg_rgb.tif | 1 KiB | bioformats | ok | 1.692 | 0.838 | 121.1 |
| tests/data/tiff_im/rgb8_jpeg_rgb.tif | 1 KiB | bioformats-rs | ok | 0.008 | 0.005 | 13.0 |
| tests/data/tiff_im/rgb8_jpeg_tiles.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/tiff_im/rgb8_jpeg_tiles.tif | 2 KiB | bioformats | ok | 1.650 | 0.788 | 124.0 |
| tests/data/tiff_im/rgb8_jpeg_tiles.tif | 2 KiB | bioformats-rs | ok | 0.011 | 0.005 | 13.1 |
| tests/data/tiff_im/rgb8_packbits.tif | 18 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/data/tiff_im/rgb8_packbits.tif | 18 KiB | bioformats | ok | 1.604 | 0.763 | 117.6 |
| tests/data/tiff_im/rgb8_packbits.tif | 18 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.2 |
| tests/data/tiff_im/rgba8_lzw.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/data/tiff_im/rgba8_lzw.tif | 2 KiB | bioformats | ok | 1.577 | 0.719 | 119.7 |
| tests/data/tiff_im/rgba8_lzw.tif | 2 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.7 |
| tests/reference/ome/TIFF/libtiff/caspian.tif | 291 KiB | scopeio | ok | 0.006 | 0.007 | 4.1 |
| tests/reference/ome/TIFF/libtiff/caspian.tif | 291 KiB | bioformats | error: loci.formats.FormatException: Unsupported image type  | 1.604 | 0.707 | 146.9 |
| tests/reference/ome/TIFF/libtiff/caspian.tif | 291 KiB | bioformats-rs | ok | 0.009 | 0.010 | 15.6 |
| tests/reference/ome/TIFF/libtiff/cramps-tile.tif | 768 KiB | scopeio | ok | 0.001 | 0.002 | 3.2 |
| tests/reference/ome/TIFF/libtiff/cramps-tile.tif | 768 KiB | bioformats | ok | 1.534 | 0.734 | 164.9 |
| tests/reference/ome/TIFF/libtiff/cramps-tile.tif | 768 KiB | bioformats-rs | ok | 0.005 | 0.007 | 13.2 |
| tests/reference/ome/TIFF/libtiff/cramps.tif | 190 KiB | scopeio | ok | 0.001 | 0.002 | 3.1 |
| tests/reference/ome/TIFF/libtiff/cramps.tif | 190 KiB | bioformats | ok | 1.606 | 0.690 | 143.9 |
| tests/reference/ome/TIFF/libtiff/cramps.tif | 190 KiB | bioformats-rs | ok | 0.005 | 0.006 | 12.8 |
| tests/reference/ome/TIFF/libtiff/dscf0013.tif | 639 KiB | scopeio | ok | 0.005 | 0.006 | 3.2 |
| tests/reference/ome/TIFF/libtiff/dscf0013.tif | 639 KiB | bioformats | ok | 1.774 | 0.770 | 172.8 |
| tests/reference/ome/TIFF/libtiff/dscf0013.tif | 639 KiB | bioformats-rs | ok | 0.010 | 0.010 | 13.7 |
| tests/reference/ome/TIFF/libtiff/fax2d.tif | 32 KiB | scopeio | error: Error: UnsupportedVariant("TIFF compression 3") | 0.002 | 0.002 | 4.6 |
| tests/reference/ome/TIFF/libtiff/fax2d.tif | 32 KiB | bioformats | error: loci.formats.MissingLibraryException: Java Advanced I | 1.578 | 0.739 | 128.9 |
| tests/reference/ome/TIFF/libtiff/fax2d.tif | 32 KiB | bioformats-rs | error: Error reading plane 0: Invalid data: CCITT Group 3: i | 0.003 | 0.004 | 11.7 |
| tests/reference/ome/TIFF/libtiff/g3test.tif | 49 KiB | scopeio | error: Error: UnsupportedVariant("TIFF compression 3") | 0.002 | 0.003 | 4.8 |
| tests/reference/ome/TIFF/libtiff/g3test.tif | 49 KiB | bioformats | error: loci.formats.MissingLibraryException: Java Advanced I | 1.602 | 0.709 | 130.8 |
| tests/reference/ome/TIFF/libtiff/g3test.tif | 49 KiB | bioformats-rs | ok | 0.020 | 0.021 | 15.5 |
| tests/reference/ome/TIFF/libtiff/jello.tif | 46 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/TIFF/libtiff/jello.tif | 46 KiB | bioformats | ok | 1.532 | 0.676 | 127.3 |
| tests/reference/ome/TIFF/libtiff/jello.tif | 46 KiB | bioformats-rs | ok | 0.003 | 0.003 | 12.4 |
| tests/reference/ome/TIFF/libtiff/jim___ah.tif | 66 KiB | scopeio | ok | 0.002 | 0.003 | 3.9 |
| tests/reference/ome/TIFF/libtiff/jim___ah.tif | 66 KiB | bioformats | ok | 1.512 | 0.694 | 124.8 |
| tests/reference/ome/TIFF/libtiff/jim___ah.tif | 66 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/TIFF/libtiff/jim___cg.tif | 92 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/TIFF/libtiff/jim___cg.tif | 92 KiB | bioformats | ok | 1.442 | 0.711 | 121.5 |
| tests/reference/ome/TIFF/libtiff/jim___cg.tif | 92 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.1 |
| tests/reference/ome/TIFF/libtiff/jim___dg.tif | 92 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/TIFF/libtiff/jim___dg.tif | 92 KiB | bioformats | ok | 1.467 | 0.715 | 124.5 |
| tests/reference/ome/TIFF/libtiff/jim___dg.tif | 92 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.4 |
| tests/reference/ome/TIFF/libtiff/jim___gg.tif | 92 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/TIFF/libtiff/jim___gg.tif | 92 KiB | bioformats | ok | 1.502 | 0.699 | 122.8 |
| tests/reference/ome/TIFF/libtiff/jim___gg.tif | 92 KiB | bioformats-rs | ok | 0.004 | 0.005 | 11.4 |
| tests/reference/ome/TIFF/libtiff/off_l16.tif | 71 KiB | scopeio | error: Error: UnsupportedVariant("TIFF photometric interpret | 0.001 | 0.001 | 2.6 |
| tests/reference/ome/TIFF/libtiff/off_l16.tif | 71 KiB | bioformats | error: loci.common.enumeration.EnumException: Unable to find | 1.274 | 0.652 | 110.4 |
| tests/reference/ome/TIFF/libtiff/off_l16.tif | 71 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: Unknown TI | 0.003 | 0.004 | 11.2 |
| tests/reference/ome/TIFF/libtiff/off_luv24.tif | 220 KiB | scopeio | error: Error: UnsupportedVariant("TIFF photometric interpret | 0.001 | 0.001 | 2.7 |
| tests/reference/ome/TIFF/libtiff/off_luv24.tif | 220 KiB | bioformats | error: loci.common.enumeration.EnumException: Unable to find | 1.193 | 0.609 | 117.1 |
| tests/reference/ome/TIFF/libtiff/off_luv24.tif | 220 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: Unknown TI | 0.003 | 0.004 | 10.1 |
| tests/reference/ome/TIFF/libtiff/off_luv32.tif | 112 KiB | scopeio | error: Error: UnsupportedVariant("TIFF photometric interpret | 0.001 | 0.001 | 2.6 |
| tests/reference/ome/TIFF/libtiff/off_luv32.tif | 112 KiB | bioformats | error: loci.common.enumeration.EnumException: Unable to find | 1.176 | 0.601 | 112.0 |
| tests/reference/ome/TIFF/libtiff/off_luv32.tif | 112 KiB | bioformats-rs | error: Error reading plane 0: Unsupported format: Unknown TI | 0.003 | 0.004 | 11.3 |
| tests/reference/ome/TIFF/libtiff/oxford.tif | 21 KiB | scopeio | ok | 0.002 | 0.003 | 2.9 |
| tests/reference/ome/TIFF/libtiff/oxford.tif | 21 KiB | bioformats | ok | 1.535 | 0.686 | 131.4 |
| tests/reference/ome/TIFF/libtiff/oxford.tif | 21 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/TIFF/libtiff/quad-jpeg.tif | 24 KiB | scopeio | ok | 0.002 | 0.003 | 3.3 |
| tests/reference/ome/TIFF/libtiff/quad-jpeg.tif | 24 KiB | bioformats | ok | 1.722 | 0.740 | 136.2 |
| tests/reference/ome/TIFF/libtiff/quad-jpeg.tif | 24 KiB | bioformats-rs | ok | 0.007 | 0.008 | 14.2 |
| tests/reference/ome/TIFF/libtiff/quad-lzw.tif | 209 KiB | scopeio | ok | 0.003 | 0.004 | 3.3 |
| tests/reference/ome/TIFF/libtiff/quad-lzw.tif | 209 KiB | bioformats | ok | 1.628 | 0.733 | 146.3 |
| tests/reference/ome/TIFF/libtiff/quad-lzw.tif | 209 KiB | bioformats-rs | error: Error reading plane 0: Codec error: invalid code in L | 0.003 | 0.003 | 11.4 |
| tests/reference/ome/TIFF/libtiff/quad-tile.tif | 204 KiB | scopeio | ok | 0.003 | 0.003 | 3.3 |
| tests/reference/ome/TIFF/libtiff/quad-tile.tif | 204 KiB | bioformats | ok | 1.695 | 0.721 | 143.1 |
| tests/reference/ome/TIFF/libtiff/quad-tile.tif | 204 KiB | bioformats-rs | ok | 0.007 | 0.008 | 13.5 |
| tests/reference/ome/TIFF/libtiff/smallliz.tif | 5 KiB | scopeio | error: Error: UnsupportedVariant("old-style JPEG TIFF compre | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/TIFF/libtiff/smallliz.tif | 5 KiB | bioformats | error: java.lang.NullPointerException: ImageIO returned null | 1.523 | 0.686 | 128.9 |
| tests/reference/ome/TIFF/libtiff/smallliz.tif | 5 KiB | bioformats-rs | error: Error reading plane 0: Codec error: invalid JPEG form | 0.003 | 0.004 | 12.3 |
| tests/reference/ome/TIFF/libtiff/strike.tif | 108 KiB | scopeio | ok | 0.002 | 0.002 | 2.8 |
| tests/reference/ome/TIFF/libtiff/strike.tif | 108 KiB | bioformats | ok | 1.582 | 0.698 | 132.5 |
| tests/reference/ome/TIFF/libtiff/strike.tif | 108 KiB | bioformats-rs | ok | 0.005 | 0.006 | 12.8 |
| tests/reference/ome/TIFF/libtiff/text.tif | 151 KiB | scopeio | error: Error: UnsupportedVariant("TIFF compression 32809") | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/TIFF/libtiff/text.tif | 151 KiB | bioformats | error: loci.formats.MissingLibraryException: Java Advanced I | 1.462 | 0.655 | 132.4 |
| tests/reference/ome/TIFF/libtiff/text.tif | 151 KiB | bioformats-rs | error: Error reading plane 0: Invalid data: Thunderscan data | 0.004 | 0.004 | 10.9 |
| tests/reference/ome/TIFF/libtiff/ycbcr-cat.tif | 71 KiB | scopeio | ok | 0.003 | 0.004 | 3.0 |
| tests/reference/ome/TIFF/libtiff/ycbcr-cat.tif | 71 KiB | bioformats | ok | 1.779 | 0.761 | 130.2 |
| tests/reference/ome/TIFF/libtiff/ycbcr-cat.tif | 71 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/TIFF/libtiff/zackthecat.tif | 8 KiB | scopeio | error: Error: UnsupportedVariant("old-style JPEG TIFF compre | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/TIFF/libtiff/zackthecat.tif | 8 KiB | bioformats | error: java.lang.NullPointerException: ImageIO returned null | 1.619 | 0.760 | 128.6 |
| tests/reference/ome/TIFF/libtiff/zackthecat.tif | 8 KiB | bioformats-rs | error: Error reading plane 0: Codec error: invalid JPEG form | 0.002 | 0.003 | 12.2 |
| tests/reference/ome/TIFF/condensation/A5.pattern1.tif | 6591 KiB | scopeio | ok | 0.007 | 0.008 | 2.8 |
| tests/reference/ome/TIFF/condensation/A5.pattern1.tif | 6591 KiB | bioformats | ok | 2.173 | 0.985 | 272.0 |
| tests/reference/ome/TIFF/condensation/A5.pattern1.tif | 6591 KiB | bioformats-rs | ok | 0.015 | 0.016 | 25.2 |
| tests/external/tif_01.tif | 15947 KiB | scopeio | ok | 0.008 | 0.009 | 18.1 |
| tests/external/tif_01.tif | 15947 KiB | bioformats | ok | 2.083 | 0.861 | 279.1 |
| tests/external/tif_01.tif | 15947 KiB | bioformats-rs | ok | 0.029 | 0.030 | 43.5 |

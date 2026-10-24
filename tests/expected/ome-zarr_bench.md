# ome-zarr: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/zarr/bioformats2raw.zarr | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/data/zarr/bioformats2raw.zarr | 1 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.691 | 0.398 | 101.9 |
| tests/data/zarr/bioformats2raw.zarr | 1 KiB | bioformats-rs | ok | 0.005 | 0.004 | 12.7 |
| tests/data/zarr/plate.zarr | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/zarr/plate.zarr | 3 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.722 | 0.413 | 101.9 |
| tests/data/zarr/plate.zarr | 3 KiB | bioformats-rs | ok | 0.005 | 0.004 | 12.9 |
| tests/data/zarr/v2_fortran_be.zarr | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/data/zarr/v2_fortran_be.zarr | 2 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.681 | 0.401 | 101.2 |
| tests/data/zarr/v2_fortran_be.zarr | 2 KiB | bioformats-rs | ok | 0.007 | 0.004 | 13.0 |
| tests/data/zarr/v2_gzip_dot_sep.zarr | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/zarr/v2_gzip_dot_sep.zarr | 2 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.691 | 0.400 | 101.6 |
| tests/data/zarr/v2_gzip_dot_sep.zarr | 2 KiB | bioformats-rs | ok | 0.010 | 0.004 | 13.1 |
| tests/data/zarr/v2_missing_chunks.zarr | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/data/zarr/v2_missing_chunks.zarr | 1 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.701 | 0.405 | 101.2 |
| tests/data/zarr/v2_missing_chunks.zarr | 1 KiB | bioformats-rs | ok | 0.006 | 0.004 | 12.6 |
| tests/data/zarr/v2_raw_cyx.zarr | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/zarr/v2_raw_cyx.zarr | 2 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.712 | 0.401 | 101.9 |
| tests/data/zarr/v2_raw_cyx.zarr | 2 KiB | bioformats-rs | ok | 0.008 | 0.004 | 12.9 |
| tests/data/zarr/v2_yx.zarr | 1 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/zarr/v2_yx.zarr | 1 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.724 | 0.415 | 101.8 |
| tests/data/zarr/v2_yx.zarr | 1 KiB | bioformats-rs | ok | 0.005 | 0.004 | 12.9 |
| tests/data/zarr/v2_zlib_tczyx.zarr | 5 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/zarr/v2_zlib_tczyx.zarr | 5 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.709 | 0.415 | 101.4 |
| tests/data/zarr/v2_zlib_tczyx.zarr | 5 KiB | bioformats-rs | error: Error opening <file>: Format error: zarr array open / | 0.002 | 0.003 | 11.4 |
| tests/data/zarr/v3_big_endian.zarr | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/data/zarr/v3_big_endian.zarr | 2 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.711 | 0.402 | 101.4 |
| tests/data/zarr/v3_big_endian.zarr | 2 KiB | bioformats-rs | ok | 0.007 | 0.004 | 12.8 |
| tests/data/zarr/v3_gzip_crc.zarr | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/zarr/v3_gzip_crc.zarr | 2 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.707 | 0.401 | 101.6 |
| tests/data/zarr/v3_gzip_crc.zarr | 2 KiB | bioformats-rs | ok | 0.007 | 0.004 | 12.9 |
| tests/data/zarr/v3_shard_gzip.zarr | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/data/zarr/v3_shard_gzip.zarr | 3 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.710 | 0.406 | 102.3 |
| tests/data/zarr/v3_shard_gzip.zarr | 3 KiB | bioformats-rs | ok | 0.008 | 0.004 | 13.2 |
| tests/reference/zarr/6001240.zarr | 41904 KiB | scopeio | ok | 0.077 | 0.079 | 3.3 |
| tests/reference/zarr/6001240.zarr | 41904 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.700 | 0.395 | 101.8 |
| tests/reference/zarr/6001240.zarr | 41904 KiB | bioformats-rs | ok | 0.140 | 0.139 | 146.8 |
| tests/reference/zarr/6001240_labels.zarr | 30992 KiB | scopeio | ok | 1.325 | 1.328 | 3.7 |
| tests/reference/zarr/6001240_labels.zarr | 30992 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.702 | 0.400 | 101.6 |
| tests/reference/zarr/6001240_labels.zarr | 30992 KiB | bioformats-rs | ok | 0.373 | 0.184 | 150.3 |
| tests/reference/zarr/2551.zarr | 537446 KiB | scopeio | ok | 1.213 | 1.216 | 9.9 |
| tests/reference/zarr/2551.zarr | 537446 KiB | bioformats | error: java.io.FileNotFoundException: <file> (Is a directory | 0.677 | 0.395 | 101.4 |
| tests/reference/zarr/2551.zarr | 537446 KiB | bioformats-rs | ok | 0.167 | 0.166 | 187.5 |

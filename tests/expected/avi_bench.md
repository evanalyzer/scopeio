# avi: CPU and RAM compared with the reference readers

Generated 2026-11-04 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/avi/absidx.avi | 2 KiB | scopeio | ok | 0.001 | 0.001 | 4.8 |
| tests/data/avi/absidx.avi | 2 KiB | bioformats | error: java.lang.IllegalArgumentException: 0 must be non-nul | 0.663 | 0.375 | 102.5 |
| tests/data/avi/absidx.avi | 2 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.8 |
| tests/data/avi/bgr24.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/avi/bgr24.avi | 1 KiB | bioformats | ok | 0.844 | 0.431 | 114.6 |
| tests/data/avi/bgr24.avi | 1 KiB | bioformats-rs | error: Error opening <file>: Invalid data: AVI: uncompressed | 0.002 | 0.003 | 11.9 |
| tests/data/avi/bgra32.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/avi/bgra32.avi | 1 KiB | bioformats | ok | 0.830 | 0.433 | 115.0 |
| tests/data/avi/bgra32.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.8 |
| tests/data/avi/dropped.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/avi/dropped.avi | 1 KiB | bioformats | ok | 0.831 | 0.435 | 115.1 |
| tests/data/avi/dropped.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/data/avi/gray8.avi | 1 KiB | scopeio | ok | 0.001 | 0.001 | 4.9 |
| tests/data/avi/gray8.avi | 1 KiB | bioformats | ok | 0.847 | 0.440 | 114.9 |
| tests/data/avi/gray8.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/data/avi/h264.avi | 0 KiB | scopeio | error: Error: UnsupportedVariant("AVI video codec 'H264' is  | 0.001 | 0.002 | 4.5 |
| tests/data/avi/h264.avi | 0 KiB | bioformats | error: loci.formats.UnsupportedCompressionException: 8759670 | 0.672 | 0.375 | 102.4 |
| tests/data/avi/h264.avi | 0 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: AVI compres | 0.003 | 0.004 | 11.9 |
| tests/data/avi/mjpeg.avi | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/avi/mjpeg.avi | 2 KiB | bioformats | ok | 0.896 | 0.479 | 118.5 |
| tests/data/avi/mjpeg.avi | 2 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.3 |
| tests/data/avi/mjpeg_gray.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/avi/mjpeg_gray.avi | 1 KiB | bioformats | error: java.io.EOFException: Attempting to read beyond end o | 0.665 | 0.391 | 102.1 |
| tests/data/avi/mjpeg_gray.avi | 1 KiB | bioformats-rs | error: Error writing /tmp/tmpkbjr1v5v/tmpkmtx5p_r/out.ics: F | 0.002 | 0.003 | 10.9 |
| tests/data/avi/mjpeg_nodht.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/avi/mjpeg_nodht.avi | 1 KiB | bioformats | ok | 0.890 | 0.472 | 119.7 |
| tests/data/avi/mjpeg_nodht.avi | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.3 |
| tests/data/avi/noidx.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/data/avi/noidx.avi | 1 KiB | bioformats | ok | 0.851 | 0.453 | 113.6 |
| tests/data/avi/noidx.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/data/avi/opendml.avi | 2 KiB | scopeio | ok | 0.001 | 0.001 | 4.8 |
| tests/data/avi/opendml.avi | 2 KiB | bioformats | ok | 0.838 | 0.441 | 114.1 |
| tests/data/avi/opendml.avi | 2 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.8 |
| tests/data/avi/palette8.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/avi/palette8.avi | 1 KiB | bioformats | ok | 0.855 | 0.458 | 114.1 |
| tests/data/avi/palette8.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.6 |
| tests/data/avi/rle4.avi | 0 KiB | scopeio | ok | 0.001 | 0.001 | 4.7 |
| tests/data/avi/rle4.avi | 0 KiB | bioformats | error: loci.formats.UnsupportedCompressionException: 2 not s | 0.655 | 0.373 | 102.8 |
| tests/data/avi/rle4.avi | 0 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: AVI compres | 0.002 | 0.003 | 11.9 |
| tests/data/avi/rle8.avi | 1 KiB | scopeio | ok | 0.001 | 0.001 | 4.6 |
| tests/data/avi/rle8.avi | 1 KiB | bioformats | ok | 0.846 | 0.438 | 114.8 |
| tests/data/avi/rle8.avi | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/data/avi/topdown24.avi | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/avi/topdown24.avi | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: -4 must be neithe | 0.667 | 0.384 | 103.1 |
| tests/data/avi/topdown24.avi | 1 KiB | bioformats-rs | error: Error opening <file>: Invalid data: AVI: uncompressed | 0.002 | 0.003 | 11.9 |
| tests/data/avi/truncated.avi | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/avi/truncated.avi | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: newPosition > lim | 0.664 | 0.378 | 101.8 |
| tests/data/avi/truncated.avi | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: AVI chunk extends | 0.003 | 0.003 | 11.8 |
| tests/data/avi/y800.avi | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/avi/y800.avi | 0 KiB | bioformats | error: loci.formats.UnsupportedCompressionException: 8084665 | 0.663 | 0.383 | 102.1 |
| tests/data/avi/y800.avi | 0 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: AVI compres | 0.002 | 0.003 | 11.8 |

## Large generated inputs

Generated by `tests/scripts/make_avi_samples.py --bench` (not committed):
`bench_raw8.avi`, 1000 frames of 1024 x 1024 8-bit gray, an OpenDML file
of 1.0 GB (500 frames in RIFF 'AVI ', 500 in RIFF 'AVIX'); `bench_mjpeg.avi`,
1000 Motion JPEG frames of 1024 x 1024 4:2:0 (one encoded frame repeated).
The reference readers read only the first RIFF chunk of the OpenDML file
(500 of 1000 frames); scopeio reads all 1000. Crop rows read a
512 x 512 region of every frame (bioformats-rs has no crop option). Median
of 3 runs; reference readers also write the pixels to ICS.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/avi-bench/bench_raw8.avi | 1024025 KiB | scopeio | ok | 0.367 | 0.369 | 5.7 |
| tests/reference/avi-bench/bench_raw8.avi | 1024025 KiB | bioformats | ok | 8.746 | 7.564 | 906.4 |
| tests/reference/avi-bench/bench_raw8.avi | 1024025 KiB | bioformats-rs | ok | 0.855 | 0.857 | 1011.3 |
| tests/reference/avi-bench/bench_mjpeg.avi | 65217 KiB | scopeio | ok | 2.915 | 2.917 | 16.9 |
| tests/reference/avi-bench/bench_mjpeg.avi | 65217 KiB | bioformats | ok | 12.710 | 11.191 | 738.6 |
| tests/reference/avi-bench/bench_mjpeg.avi | 65217 KiB | bioformats-rs | ok | 8.728 | 8.734 | 6019.0 |
| tests/reference/avi-bench/bench_raw8.avi crop 256,256,512,512 | 1024025 KiB | scopeio | ok | 0.180 | 0.182 | 4.7 |
| tests/reference/avi-bench/bench_raw8.avi crop 256,256,512,512 | 1024025 KiB | bioformats | ok | 4.842 | 3.782 | 670.0 |
| tests/reference/avi-bench/bench_raw8.avi crop 256,256,512,512 | 1024025 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/avi-bench/bench_mjpeg.avi crop 256,256,512,512 | 65217 KiB | scopeio | ok | 2.818 | 2.821 | 14.8 |
| tests/reference/avi-bench/bench_mjpeg.avi crop 256,256,512,512 | 65217 KiB | bioformats | ok | 11.963 | 10.485 | 827.3 |
| tests/reference/avi-bench/bench_mjpeg.avi crop 256,256,512,512 | 65217 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

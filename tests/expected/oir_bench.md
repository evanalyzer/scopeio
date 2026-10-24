# oir: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Olympus-OIR/etienne/amy slice z stack_0001.oir | 25349 KiB | scopeio | ok | 0.019 | 0.021 | 12.0 |
| tests/reference/ome/Olympus-OIR/etienne/amy slice z stack_0001.oir | 25349 KiB | bioformats | ok | 3.623 | 1.036 | 474.0 |
| tests/reference/ome/Olympus-OIR/etienne/amy slice z stack_0001.oir | 25349 KiB | bioformats-rs | ok | 0.035 | 0.037 | 46.0 |
| tests/reference/ome/Olympus-OIR/etienne/coupe shg stack_0001.oir | 26898 KiB | scopeio | ok | 0.014 | 0.015 | 12.0 |
| tests/reference/ome/Olympus-OIR/etienne/coupe shg stack_0001.oir | 26898 KiB | bioformats | ok | 3.844 | 1.167 | 487.2 |
| tests/reference/ome/Olympus-OIR/etienne/coupe shg stack_0001.oir | 26898 KiB | bioformats-rs | ok | 0.038 | 0.039 | 48.3 |
| tests/reference/ome/Olympus-OIR/etienne/venus stack.oir | 43753 KiB | scopeio | ok | 0.028 | 0.030 | 12.4 |
| tests/reference/ome/Olympus-OIR/etienne/venus stack.oir | 43753 KiB | bioformats | ok | 5.035 | 1.544 | 623.9 |
| tests/reference/ome/Olympus-OIR/etienne/venus stack.oir | 43753 KiB | bioformats-rs | ok | 0.078 | 0.080 | 80.2 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI-mCherry_3T_4Z_5Lambda-420-630-w10nm-s50nm.oir | 35317 KiB | scopeio | ok | 0.019 | 0.021 | 7.5 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI-mCherry_3T_4Z_5Lambda-420-630-w10nm-s50nm.oir | 35317 KiB | bioformats | ok | 4.262 | 1.355 | 516.0 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI-mCherry_3T_4Z_5Lambda-420-630-w10nm-s50nm.oir | 35317 KiB | bioformats-rs | ok | 0.067 | 0.069 | 73.3 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_22Lambda-420-630-w10nm-s10nm.oir | 15415 KiB | scopeio | ok | 0.010 | 0.012 | 7.4 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_22Lambda-420-630-w10nm-s10nm.oir | 15415 KiB | bioformats | ok | 3.683 | 1.058 | 450.6 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_22Lambda-420-630-w10nm-s10nm.oir | 15415 KiB | bioformats-rs | ok | 0.029 | 0.030 | 35.5 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4T_5Lambda-420-630-w10nm-s50nm.oir | 14261 KiB | scopeio | ok | 0.011 | 0.012 | 7.3 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4T_5Lambda-420-630-w10nm-s50nm.oir | 14261 KiB | bioformats | ok | 3.768 | 1.029 | 374.1 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4T_5Lambda-420-630-w10nm-s50nm.oir | 14261 KiB | bioformats-rs | ok | 0.024 | 0.025 | 33.4 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4Z_5Lambda-420-630-w10nm-s50nm.oir | 14253 KiB | scopeio | ok | 0.012 | 0.013 | 7.5 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4Z_5Lambda-420-630-w10nm-s50nm.oir | 14253 KiB | bioformats | ok | 3.550 | 1.110 | 361.4 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-12773657/DAPI_mCherry_4Z_5Lambda-420-630-w10nm-s50nm.oir | 14253 KiB | bioformats-rs | ok | 0.023 | 0.024 | 33.2 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Map_A01.oir | 1661 KiB | scopeio | error: Error: UnsupportedVariant("OIR without image planes ( | 0.005 | 0.007 | 6.4 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Map_A01.oir | 1661 KiB | bioformats | error: java.lang.IllegalArgumentException: 0 must be non-nul | 0.880 | 0.474 | 138.3 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Map_A01.oir | 1661 KiB | bioformats-rs | error: killed: exceeded the RSS limit of 8192 MiB | 2.974 | 2.977 | 8252.1 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/tiling-sample-brain-section_A01_G001_0001.oir | 61228 KiB | scopeio | ok | 0.028 | 0.030 | 10.1 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/tiling-sample-brain-section_A01_G001_0001.oir | 61228 KiB | bioformats | ok | 4.295 | 1.286 | 504.6 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/tiling-sample-brain-section_A01_G001_0001.oir | 61228 KiB | bioformats-rs | ok | 0.152 | 0.153 | 203.5 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_10sec_sequence_frame.oir | 17107 KiB | scopeio | ok | 0.016 | 0.018 | 12.4 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_10sec_sequence_frame.oir | 17107 KiB | bioformats | ok | 5.056 | 1.179 | 450.9 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_10sec_sequence_frame.oir | 17107 KiB | bioformats-rs | ok | 0.032 | 0.033 | 45.2 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame.oir | 24419 KiB | scopeio | ok | 0.021 | 0.022 | 12.6 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame.oir | 24419 KiB | bioformats | ok | 5.660 | 1.278 | 611.9 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame.oir | 24419 KiB | bioformats-rs | ok | 0.052 | 0.054 | 73.6 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame_z stack.oir | 50565 KiB | scopeio | ok | 0.029 | 0.031 | 12.9 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame_z stack.oir | 50565 KiB | bioformats | ok | 6.567 | 1.566 | 646.7 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_30sec_sequence_frame_z stack.oir | 50565 KiB | bioformats-rs | ok | 0.131 | 0.132 | 145.1 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_freeRun_sequence_frame.oir | 24419 KiB | scopeio | ok | 0.016 | 0.017 | 12.8 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_freeRun_sequence_frame.oir | 24419 KiB | bioformats | ok | 5.923 | 1.324 | 600.9 |
| tests/reference/ome/Olympus-OIR/imagesc-105684/1202-interval_freeRun_sequence_frame.oir | 24419 KiB | bioformats-rs | ok | 0.059 | 0.060 | 73.6 |
| tests/data/oir/synthetic.oir | 11 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/data/oir/synthetic.oir | 11 KiB | bioformats | error: java.lang.NullPointerException: Cannot invoke "String | 0.762 | 0.423 | 105.8 |
| tests/data/oir/synthetic.oir | 11 KiB | bioformats-rs | error: Error opening <file>: Format error: Olympus OIR XML m | 0.003 | 0.003 | 12.7 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Stitch_A01_G001.oir (series 0) crop 1500,1000,512,512 | 1061437 KiB | scopeio | ok | 0.040 | 0.041 | 7.7 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Stitch_A01_G001.oir (series 0) crop 1500,1000,512,512 | 1061437 KiB | bioformats | ok | 5.039 | 1.307 | 619.2 |
| tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Stitch_A01_G001.oir (series 0) crop 1500,1000,512,512 | 1061437 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

# czi: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/zenodo-7015307/S=1_3x3_T=3_Z=4_CH=2.czi | 106112 KiB | scopeio | ok | 0.043 | 0.044 | 6.9 |
| tests/reference/zenodo-7015307/S=1_3x3_T=3_Z=4_CH=2.czi | 106112 KiB | bioformats | ok | 8.821 | 1.786 | 683.5 |
| tests/reference/zenodo-7015307/S=1_3x3_T=3_Z=4_CH=2.czi | 106112 KiB | bioformats-rs | ok | 0.088 | 0.090 | 160.4 |
| tests/reference/zenodo-7015307/S=1_CH=2.czi | 1540 KiB | scopeio | ok | 0.005 | 0.006 | 5.4 |
| tests/reference/zenodo-7015307/S=1_CH=2.czi | 1540 KiB | bioformats | ok | 7.821 | 1.546 | 555.9 |
| tests/reference/zenodo-7015307/S=1_CH=2.czi | 1540 KiB | bioformats-rs | error: Error opening <file>: IO error: CZI directory entry 1 | 0.005 | 0.006 | 13.5 |
| tests/reference/zenodo-7015307/S=2_2x2_CH=1.czi | 2525 KiB | scopeio | ok | 0.007 | 0.008 | 5.6 |
| tests/reference/zenodo-7015307/S=2_2x2_CH=1.czi | 2525 KiB | bioformats | ok | 7.715 | 1.567 | 544.9 |
| tests/reference/zenodo-7015307/S=2_2x2_CH=1.czi | 2525 KiB | bioformats-rs | ok | 0.009 | 0.010 | 13.0 |
| tests/reference/zenodo-7015307/S=2_2x2_T=1_Z=4_CH=1.czi | 6327 KiB | scopeio | ok | 0.007 | 0.008 | 5.5 |
| tests/reference/zenodo-7015307/S=2_2x2_T=1_Z=4_CH=1.czi | 6327 KiB | bioformats | ok | 8.607 | 1.543 | 562.6 |
| tests/reference/zenodo-7015307/S=2_2x2_T=1_Z=4_CH=1.czi | 6327 KiB | bioformats-rs | ok | 0.009 | 0.010 | 15.8 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_CH=1.czi | 5059 KiB | scopeio | ok | 0.006 | 0.007 | 5.5 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_CH=1.czi | 5059 KiB | bioformats | ok | 8.057 | 1.656 | 537.2 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_CH=1.czi | 5059 KiB | bioformats-rs | ok | 0.009 | 0.010 | 14.7 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_Z=4_CH=1.czi | 16444 KiB | scopeio | ok | 0.012 | 0.013 | 5.6 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_Z=4_CH=1.czi | 16444 KiB | bioformats | ok | 9.499 | 1.783 | 606.0 |
| tests/reference/zenodo-7015307/S=2_2x2_T=3_Z=4_CH=1.czi | 16444 KiB | bioformats-rs | ok | 0.016 | 0.017 | 22.8 |
| tests/reference/zenodo-7015307/S=2_2x2_Z=4_CH=1.czi | 6324 KiB | scopeio | ok | 0.009 | 0.010 | 5.6 |
| tests/reference/zenodo-7015307/S=2_2x2_Z=4_CH=1.czi | 6324 KiB | bioformats | ok | 8.360 | 1.716 | 549.6 |
| tests/reference/zenodo-7015307/S=2_2x2_Z=4_CH=1.czi | 6324 KiB | bioformats-rs | ok | 0.012 | 0.013 | 15.5 |
| tests/reference/zenodo-7015307/S=2_3x3_T=1_Z=4_CH=2.czi | 70923 KiB | scopeio | ok | 0.036 | 0.037 | 7.1 |
| tests/reference/zenodo-7015307/S=2_3x3_T=1_Z=4_CH=2.czi | 70923 KiB | bioformats | ok | 9.700 | 1.951 | 660.6 |
| tests/reference/zenodo-7015307/S=2_3x3_T=1_Z=4_CH=2.czi | 70923 KiB | bioformats-rs | ok | 0.037 | 0.039 | 61.9 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_CH=2.czi | 54027 KiB | scopeio | ok | 0.027 | 0.028 | 6.8 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_CH=2.czi | 54027 KiB | bioformats | ok | 9.959 | 1.841 | 643.0 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_CH=2.czi | 54027 KiB | bioformats-rs | error: Error opening <file>: IO error: CZI directory entry 4 | 0.025 | 0.026 | 63.4 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_Z=1_CH=2.czi | 53337 KiB | scopeio | ok | 0.028 | 0.030 | 7.1 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_Z=1_CH=2.czi | 53337 KiB | bioformats | ok | 10.089 | 1.888 | 631.8 |
| tests/reference/zenodo-7015307/S=2_3x3_T=3_Z=1_CH=2.czi | 53337 KiB | bioformats-rs | ok | 0.029 | 0.031 | 49.6 |
| tests/reference/zenodo-7015307/S=2_T=3_CH=1.czi | 2039 KiB | scopeio | ok | 0.006 | 0.007 | 5.5 |
| tests/reference/zenodo-7015307/S=2_T=3_CH=1.czi | 2039 KiB | bioformats | ok | 8.401 | 1.619 | 547.2 |
| tests/reference/zenodo-7015307/S=2_T=3_CH=1.czi | 2039 KiB | bioformats-rs | error: Error reading plane 0: Format error: CZI tile X bound | 0.009 | 0.010 | 12.2 |
| tests/reference/zenodo-7015307/S=2_T=3_Z=5_CH=1.czi | 5143 KiB | scopeio | ok | 0.008 | 0.008 | 5.8 |
| tests/reference/zenodo-7015307/S=2_T=3_Z=5_CH=1.czi | 5143 KiB | bioformats | ok | 9.207 | 1.764 | 586.5 |
| tests/reference/zenodo-7015307/S=2_T=3_Z=5_CH=1.czi | 5143 KiB | bioformats-rs | error: Error opening <file>: IO error: CZI directory entry 1 | 0.007 | 0.008 | 15.8 |
| tests/reference/zenodo-7015307/S=3_1Pos_2Mosaic_T=2=Z=3_CH=2.czi | 55725 KiB | scopeio | ok | 0.037 | 0.038 | 7.7 |
| tests/reference/zenodo-7015307/S=3_1Pos_2Mosaic_T=2=Z=3_CH=2.czi | 55725 KiB | bioformats | ok | 11.242 | 2.208 | 670.9 |
| tests/reference/zenodo-7015307/S=3_1Pos_2Mosaic_T=2=Z=3_CH=2.czi | 55725 KiB | bioformats-rs | ok | 0.024 | 0.026 | 39.0 |
| tests/reference/zenodo-7015307/S=3_CH=2.czi | 2047 KiB | scopeio | ok | 0.005 | 0.006 | 5.5 |
| tests/reference/zenodo-7015307/S=3_CH=2.czi | 2047 KiB | bioformats | ok | 8.593 | 1.709 | 541.5 |
| tests/reference/zenodo-7015307/S=3_CH=2.czi | 2047 KiB | bioformats-rs | error: Error reading plane 0: Format error: CZI tile X bound | 0.008 | 0.009 | 12.5 |
| tests/reference/zenodo-7015307/T=1_CH=2.czi | 1545 KiB | scopeio | ok | 0.005 | 0.006 | 5.6 |
| tests/reference/zenodo-7015307/T=1_CH=2.czi | 1545 KiB | bioformats | ok | 8.648 | 1.794 | 555.6 |
| tests/reference/zenodo-7015307/T=1_CH=2.czi | 1545 KiB | bioformats-rs | error: Error opening <file>: IO error: CZI directory entry 1 | 0.005 | 0.006 | 13.4 |
| tests/reference/zenodo-7015307/T=1_Z=5_CH=1.czi | 1916 KiB | scopeio | ok | 0.006 | 0.008 | 5.6 |
| tests/reference/zenodo-7015307/T=1_Z=5_CH=1.czi | 1916 KiB | bioformats | ok | 8.725 | 1.748 | 544.3 |
| tests/reference/zenodo-7015307/T=1_Z=5_CH=1.czi | 1916 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.009 | 0.095 | 11.8 |
| tests/reference/zenodo-7015307/T=2_CH=1.czi | 1530 KiB | scopeio | ok | 0.006 | 0.006 | 5.7 |
| tests/reference/zenodo-7015307/T=2_CH=1.czi | 1530 KiB | bioformats | ok | 8.819 | 1.729 | 564.4 |
| tests/reference/zenodo-7015307/T=2_CH=1.czi | 1530 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.010 | 0.096 | 11.7 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=1.czi | 2561 KiB | scopeio | ok | 0.007 | 0.007 | 5.6 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=1.czi | 2561 KiB | bioformats | ok | 8.625 | 1.796 | 550.4 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=1.czi | 2561 KiB | bioformats-rs | error: Error reading plane 0: Format error: CZI tile X bound | 0.009 | 0.010 | 12.2 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=2.czi | 3863 KiB | scopeio | ok | 0.007 | 0.008 | 5.5 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=2.czi | 3863 KiB | bioformats | ok | 10.995 | 2.340 | 575.4 |
| tests/reference/zenodo-7015307/T=2_Z=5_CH=2.czi | 3863 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.010 | 0.101 | 11.9 |
| tests/reference/zenodo-7015307/T=3_CH=2.czi | 2061 KiB | scopeio | ok | 0.005 | 0.006 | 5.6 |
| tests/reference/zenodo-7015307/T=3_CH=2.czi | 2061 KiB | bioformats | ok | 8.664 | 1.812 | 547.5 |
| tests/reference/zenodo-7015307/T=3_CH=2.czi | 2061 KiB | bioformats-rs | error: Error reading plane 0: Format error: CZI tile X bound | 0.009 | 0.010 | 12.3 |
| tests/reference/zenodo-7015307/T=3_Z=5_CH=2.czi | 5160 KiB | scopeio | ok | 0.007 | 0.009 | 5.6 |
| tests/reference/zenodo-7015307/T=3_Z=5_CH=2.czi | 5160 KiB | bioformats | ok | 9.441 | 1.934 | 580.9 |
| tests/reference/zenodo-7015307/T=3_Z=5_CH=2.czi | 5160 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.010 | 0.098 | 11.8 |
| tests/reference/zenodo-7015307/W96_B2+B4_S=2_T=1=Z=1_C=1_Tile=5x9.czi | 30530 KiB | scopeio | ok | 0.022 | 0.023 | 13.9 |
| tests/reference/zenodo-7015307/W96_B2+B4_S=2_T=1=Z=1_C=1_Tile=5x9.czi | 30530 KiB | bioformats | ok | 8.934 | 1.781 | 567.6 |
| tests/reference/zenodo-7015307/W96_B2+B4_S=2_T=1=Z=1_C=1_Tile=5x9.czi | 30530 KiB | bioformats-rs | ok | 0.024 | 0.025 | 31.7 |
| tests/reference/zenodo-7015307/Z=5_CH=1.czi | 1917 KiB | scopeio | ok | 0.005 | 0.006 | 5.5 |
| tests/reference/zenodo-7015307/Z=5_CH=1.czi | 1917 KiB | bioformats | ok | 8.399 | 1.807 | 550.8 |
| tests/reference/zenodo-7015307/Z=5_CH=1.czi | 1917 KiB | bioformats-rs | error: Error opening <file>: IO error: CZI directory entry 1 | 0.005 | 0.006 | 13.2 |
| tests/reference/zenodo-7015307/Z=5_CH=2.czi | 2573 KiB | scopeio | ok | 0.006 | 0.007 | 5.5 |
| tests/reference/zenodo-7015307/Z=5_CH=2.czi | 2573 KiB | bioformats | ok | 9.222 | 1.777 | 560.9 |
| tests/reference/zenodo-7015307/Z=5_CH=2.czi | 2573 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.008 | 0.093 | 11.8 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(1).czi | 7222 KiB | scopeio | ok | 0.006 | 0.007 | 10.2 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(1).czi | 7222 KiB | bioformats | ok | 1.974 | 0.819 | 283.0 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(1).czi | 7222 KiB | bioformats-rs | ok | 0.014 | 0.015 | 25.3 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(2).czi | 7222 KiB | scopeio | ok | 0.004 | 0.005 | 10.1 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(2).czi | 7222 KiB | bioformats | ok | 1.978 | 0.831 | 281.1 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(2).czi | 7222 KiB | bioformats-rs | ok | 0.013 | 0.014 | 25.2 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(3).czi | 7222 KiB | scopeio | ok | 0.005 | 0.006 | 10.1 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(3).czi | 7222 KiB | bioformats | ok | 1.932 | 0.824 | 281.2 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964(3).czi | 7222 KiB | bioformats-rs | ok | 0.012 | 0.014 | 25.2 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964.czi | 7222 KiB | scopeio | ok | 0.004 | 0.005 | 10.3 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964.czi | 7222 KiB | bioformats | ok | 1.969 | 0.824 | 285.0 |
| tests/reference/zenodo-8305531/Image_1_2023_08_18__14_32_31_964.czi | 7222 KiB | bioformats-rs | ok | 0.014 | 0.015 | 25.5 |
| tests/reference/zenodo-8305531/xt-scan-lsm980.czi | 1049 KiB | scopeio | ok | 0.007 | 0.008 | 6.1 |
| tests/reference/zenodo-8305531/xt-scan-lsm980.czi | 1049 KiB | bioformats | ok | 9.098 | 1.841 | 638.3 |
| tests/reference/zenodo-8305531/xt-scan-lsm980.czi | 1049 KiB | bioformats-rs | ok | 0.014 | 0.015 | 12.4 |
| tests/reference/zenodo-8305531/xz-scan-lsm980.czi | 1821 KiB | scopeio | ok | 0.008 | 0.008 | 6.1 |
| tests/reference/zenodo-8305531/xz-scan-lsm980.czi | 1821 KiB | bioformats | ok | 10.737 | 2.215 | 641.0 |
| tests/reference/zenodo-8305531/xz-scan-lsm980.czi | 1821 KiB | bioformats-rs | ok | 0.051 | 0.053 | 13.5 |
| tests/reference/zenodo-8305531/xzt-scan-lsm980.czi | 5732 KiB | scopeio | ok | 0.009 | 0.010 | 6.5 |
| tests/reference/zenodo-8305531/xzt-scan-lsm980.czi | 5732 KiB | bioformats | ok | 16.432 | 3.972 | 715.0 |
| tests/reference/zenodo-8305531/xzt-scan-lsm980.czi | 5732 KiB | bioformats-rs | ok | 1.106 | 1.108 | 19.4 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 0) crop 12288,8192,2048,2048 | 493632 KiB | scopeio | ok | 0.403 | 0.405 | 24.8 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 0) crop 12288,8192,2048,2048 | 493632 KiB | bioformats | ok | 10.018 | 2.395 | 621.5 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 0) crop 12288,8192,2048,2048 | 493632 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/zenodo-8305531/test_gray.czi (series 0) crop 3000,2000,2048,2048 | 294933 KiB | scopeio | ok | 0.014 | 0.015 | 10.8 |
| tests/reference/zenodo-8305531/test_gray.czi (series 0) crop 3000,2000,2048,2048 | 294933 KiB | bioformats | ok | 1.533 | 0.941 | 324.3 |
| tests/reference/zenodo-8305531/test_gray.czi (series 0) crop 3000,2000,2048,2048 | 294933 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 6) | 493632 KiB | scopeio | ok | 16.476 | 16.489 | 1309.7 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 6) | 493632 KiB | bioformats | ok | 9.217 | 1.885 | 555.8 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 6) | 493632 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 7) | 493632 KiB | scopeio | ok | 16.710 | 16.720 | 1310.1 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 7) | 493632 KiB | bioformats | ok | 9.198 | 1.796 | 582.0 |
| tests/reference/ome/Zeiss-CZI/zenodo-10577186/2023_11_30__RecognizedCode-27.czi (series 7) | 493632 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

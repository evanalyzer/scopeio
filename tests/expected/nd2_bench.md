# nd2: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/external/nicon_01.nd2 | 34612 KiB | scopeio | ok | 0.054 | 0.055 | 13.9 |
| tests/external/nicon_01.nd2 | 34612 KiB | bioformats | ok | 4.074 | 1.320 | 450.2 |
| tests/external/nicon_01.nd2 | 34612 KiB | bioformats-rs | ok | 0.101 | 0.103 | 112.6 |
| tests/reference/ome/ND2/zenodo-12734440/HE_002.nd2 | 17432 KiB | scopeio | ok | 0.045 | 0.046 | 19.4 |
| tests/reference/ome/ND2/zenodo-12734440/HE_002.nd2 | 17432 KiB | bioformats | ok | 4.199 | 1.534 | 296.4 |
| tests/reference/ome/ND2/zenodo-12734440/HE_002.nd2 | 17432 KiB | bioformats-rs | ok | 0.040 | 0.041 | 45.9 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_reconstructed-version.nd2 | 82184 KiB | scopeio | ok | 0.023 | 0.025 | 18.8 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_reconstructed-version.nd2 | 82184 KiB | bioformats | ok | 3.900 | 1.184 | 421.0 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_reconstructed-version.nd2 | 82184 KiB | bioformats-rs | ok | 0.150 | 0.152 | 171.4 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_raw-version.nd2 | 92360 KiB | scopeio | ok | 0.042 | 0.044 | 20.8 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_raw-version.nd2 | 92360 KiB | bioformats | ok | 4.384 | 1.332 | 451.6 |
| tests/reference/ome/ND2/zenodo-15046688/N1_file_bugged_raw-version.nd2 | 92360 KiB | bioformats-rs | ok | 0.221 | 0.222 | 191.5 |
| tests/reference/ome/ND2/jonas/header_test2.nd2 | 14356 KiB | scopeio | ok | 0.009 | 0.012 | 3.6 |
| tests/reference/ome/ND2/jonas/header_test2.nd2 | 14356 KiB | bioformats | ok | 5.287 | 2.024 | 324.4 |
| tests/reference/ome/ND2/jonas/header_test2.nd2 | 14356 KiB | bioformats-rs | ok | 0.111 | 0.113 | 39.7 |
| tests/reference/ome/ND2/zenodo-17186598/Experiment_0001.nd2 | 48624 KiB | scopeio | ok | 0.015 | 0.016 | 5.3 |
| tests/reference/ome/ND2/zenodo-17186598/Experiment_0001.nd2 | 48624 KiB | bioformats | ok | 5.671 | 1.718 | 353.1 |
| tests/reference/ome/ND2/zenodo-17186598/Experiment_0001.nd2 | 48624 KiB | bioformats-rs | error: Error reading plane 0: Format error: ND2: plane 0: Im | 0.049 | 0.050 | 57.8 |
| tests/reference/ome/ND2/zenodo-10277961/MRAP1 KO DN_10X03.nd2 | 17384 KiB | scopeio | ok | 0.063 | 0.065 | 19.2 |
| tests/reference/ome/ND2/zenodo-10277961/MRAP1 KO DN_10X03.nd2 | 17384 KiB | bioformats | ok | 2.707 | 0.977 | 258.8 |
| tests/reference/ome/ND2/zenodo-10277961/MRAP1 KO DN_10X03.nd2 | 17384 KiB | bioformats-rs | ok | 0.038 | 0.039 | 45.7 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception9_e3.nd2 | 8724 KiB | scopeio | ok | 0.014 | 0.016 | 2.7 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception9_e3.nd2 | 8724 KiB | bioformats | ok | 3.622 | 1.507 | 409.4 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception9_e3.nd2 | 8724 KiB | bioformats-rs | error: killed: exceeded the RSS limit of 8192 MiB | 3.253 | 3.258 | 8233.8 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception_2.nd2 | 7486 KiB | scopeio | ok | 0.080 | 0.082 | 4.5 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception_2.nd2 | 7486 KiB | bioformats | ok | 2.656 | 1.285 | 349.8 |
| tests/reference/ome/ND2/jonas/jonas_nd2Test/Exception_2.nd2 | 7486 KiB | bioformats-rs | error: note: run with `RUST_BACKTRACE=1` environment variabl | 0.428 | 0.502 | 241.9 |
| tests/external/nicon_02.nd2 (series 0) crop 1000,1000,512,512 | 792552 KiB | scopeio | ok | 0.076 | 0.078 | 3.5 |
| tests/external/nicon_02.nd2 (series 0) crop 1000,1000,512,512 | 792552 KiB | bioformats | ok | 7.233 | 1.859 | 634.9 |
| tests/external/nicon_02.nd2 (series 0) crop 1000,1000,512,512 | 792552 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/external/nicon_03.nd2 (series 0) crop 1000,1000,512,512 | 643244 KiB | scopeio | ok | 0.057 | 0.059 | 3.5 |
| tests/external/nicon_03.nd2 (series 0) crop 1000,1000,512,512 | 643244 KiB | bioformats | ok | 5.302 | 1.603 | 555.3 |
| tests/external/nicon_03.nd2 (series 0) crop 1000,1000,512,512 | 643244 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |

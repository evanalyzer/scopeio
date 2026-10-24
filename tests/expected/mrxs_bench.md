# mrxs: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- openslide-show-properties 3.4.1, using OpenSlide 3.4.1 (black box): `openslide-write-png` of the same region, PNG encoding included.
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 1600,2400,1024,1024 | 41 KiB | scopeio | ok | 0.005 | 0.006 | 7.2 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 1600,2400,1024,1024 | 41 KiB | openslide | ok | 0.044 | 0.045 | 14.2 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 800,1200,512,512 | 41 KiB | scopeio | ok | 0.005 | 0.006 | 5.1 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 800,1200,512,512 | 41 KiB | openslide | ok | 0.018 | 0.019 | 13.9 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 100,250,256,256 | 41 KiB | scopeio | ok | 0.004 | 0.005 | 4.5 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs crop 100,250,256,256 | 41 KiB | openslide | ok | 0.019 | 0.020 | 14.3 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 14000,20000,1024,1024 | 41 KiB | scopeio | ok | 0.005 | 0.006 | 8.1 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 14000,20000,1024,1024 | 41 KiB | openslide | ok | 0.040 | 0.041 | 15.4 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 3500,5000,512,512 | 41 KiB | scopeio | ok | 0.006 | 0.007 | 5.9 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 3500,5000,512,512 | 41 KiB | openslide | ok | 0.019 | 0.020 | 15.4 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 400,600,256,256 | 41 KiB | scopeio | ok | 0.006 | 0.007 | 5.9 |
| tests/reference/openslide/Mirax/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs crop 400,600,256,256 | 41 KiB | openslide | ok | 0.015 | 0.016 | 15.8 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 5000,3500,1024,1024 | 15 KiB | scopeio | ok | 0.016 | 0.017 | 6.8 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 5000,3500,1024,1024 | 15 KiB | openslide | ok | 0.432 | 0.434 | 18.2 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 2500,1700,512,512 | 15 KiB | scopeio | ok | 0.007 | 0.009 | 4.6 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 2500,1700,512,512 | 15 KiB | openslide | ok | 0.111 | 0.112 | 14.5 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 600,400,256,256 | 15 KiB | scopeio | ok | 0.007 | 0.008 | 4.0 |
| tests/reference/openslide/Mirax/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs crop 600,400,256,256 | 15 KiB | openslide | ok | 0.070 | 0.071 | 12.7 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 44000,61000,1024,1024 | 264 KiB | scopeio | ok | 0.041 | 0.043 | 22.9 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 44000,61000,1024,1024 | 264 KiB | openslide | ok | 0.394 | 0.404 | 46.5 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 11000,15250,512,512 | 264 KiB | scopeio | ok | 0.038 | 0.040 | 23.2 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 11000,15250,512,512 | 264 KiB | openslide | ok | 0.146 | 0.148 | 42.6 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 1300,1800,256,256 | 264 KiB | scopeio | ok | 0.031 | 0.032 | 22.8 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 1300,1800,256,256 | 264 KiB | openslide | ok | 0.089 | 0.099 | 39.5 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 0,0,185,349 | 264 KiB | scopeio | ok | 0.033 | 0.034 | 23.0 |
| tests/reference/openslide/Mirax/Mirax2.2-1/Mirax2.2-1.mrxs crop 0,0,185,349 | 264 KiB | openslide | ok | 0.121 | 0.123 | 39.0 |

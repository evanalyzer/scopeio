# stk: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/figshare/12981617/Figure5e sdc488 rep3.stk | 22532 KiB | scopeio | ok | 0.007 | 0.009 | 5.8 |
| tests/reference/figshare/12981617/Figure5e sdc488 rep3.stk | 22532 KiB | bioformats | ok | 1.699 | 0.709 | 218.1 |
| tests/reference/figshare/12981617/Figure5e sdc488 rep3.stk | 22532 KiB | bioformats-rs | ok | 0.042 | 0.044 | 56.3 |
| tests/reference/figshare/6852419/20160824_idv12_cng_mm_AFDAIY_ramp_14_24_17C.stk | 169799 KiB | scopeio | ok | 0.056 | 0.058 | 4.5 |
| tests/reference/figshare/6852419/20160824_idv12_cng_mm_AFDAIY_ramp_14_24_17C.stk | 169799 KiB | bioformats | ok | 14.996 | 4.104 | 701.5 |
| tests/reference/figshare/6852419/20160824_idv12_cng_mm_AFDAIY_ramp_14_24_17C.stk | 169799 KiB | bioformats-rs | ok | 0.573 | 0.580 | 345.4 |
| tests/reference/figshare/7583960/170227-hoechst only7_w1Laser4914BDGreenOnly_t1.stk | 7172 KiB | scopeio | ok | 0.004 | 0.005 | 4.4 |
| tests/reference/figshare/7583960/170227-hoechst only7_w1Laser4914BDGreenOnly_t1.stk | 7172 KiB | bioformats | ok | 1.858 | 0.727 | 215.5 |
| tests/reference/figshare/7583960/170227-hoechst only7_w1Laser4914BDGreenOnly_t1.stk | 7172 KiB | bioformats-rs | ok | 0.014 | 0.015 | 24.4 |
| tests/reference/ome/Metamorph/zenodo-13642395/test_timelapse_20240816.nd | 0 KiB | scopeio | ok | 0.039 | 0.040 | 5.6 |
| tests/reference/ome/Metamorph/zenodo-13642395/test_timelapse_20240816.nd | 0 KiB | bioformats | ok | 3.506 | 1.408 | 368.7 |
| tests/reference/ome/Metamorph/zenodo-13642395/test_timelapse_20240816.nd | 0 KiB | bioformats-rs | ok | 0.053 | 0.055 | 64.8 |

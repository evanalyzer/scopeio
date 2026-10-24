# zvi: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/figshare/10732844/Figure S2 KFERQ-Cherry.zvi | 8549 KiB | scopeio | ok | 0.009 | 0.010 | 6.6 |
| tests/reference/figshare/10732844/Figure S2 KFERQ-Cherry.zvi | 8549 KiB | bioformats | ok | 2.319 | 0.749 | 254.5 |
| tests/reference/figshare/10732844/Figure S2 KFERQ-Cherry.zvi | 8549 KiB | bioformats-rs | ok | 0.027 | 0.029 | 28.6 |
| tests/reference/figshare/11341661/20170331_HeLaS_Control_2_1.zvi | 5759 KiB | scopeio | ok | 0.009 | 0.009 | 6.8 |
| tests/reference/figshare/11341661/20170331_HeLaS_Control_2_1.zvi | 5759 KiB | bioformats | ok | 1.659 | 0.640 | 188.7 |
| tests/reference/figshare/11341661/20170331_HeLaS_Control_2_1.zvi | 5759 KiB | bioformats-rs | ok | 0.018 | 0.019 | 22.8 |
| tests/reference/figshare/12132993/Ex7 panel a - c floridanus.zvi | 67892 KiB | scopeio | ok | 0.021 | 0.023 | 6.9 |
| tests/reference/figshare/12132993/Ex7 panel a - c floridanus.zvi | 67892 KiB | bioformats | ok | 3.304 | 1.039 | 539.2 |
| tests/reference/figshare/12132993/Ex7 panel a - c floridanus.zvi | 67892 KiB | bioformats-rs | ok | 0.104 | 0.106 | 144.8 |
| tests/reference/figshare/12133278/Ex4 panel a - abda oocyte.zvi | 144092 KiB | scopeio | ok | 0.061 | 0.063 | 12.6 |
| tests/reference/figshare/12133278/Ex4 panel a - abda oocyte.zvi | 144092 KiB | bioformats | ok | 3.389 | 1.557 | 565.1 |
| tests/reference/figshare/12133278/Ex4 panel a - abda oocyte.zvi | 144092 KiB | bioformats-rs | ok | 0.302 | 0.307 | 300.3 |
| tests/reference/figshare/15042921/wt_19h_1.zvi | 5700 KiB | scopeio | ok | 0.007 | 0.008 | 6.7 |
| tests/reference/figshare/15042921/wt_19h_1.zvi | 5700 KiB | bioformats | ok | 1.786 | 0.683 | 189.9 |
| tests/reference/figshare/15042921/wt_19h_1.zvi | 5700 KiB | bioformats-rs | ok | 0.019 | 0.020 | 22.6 |
| tests/reference/figshare/24769857/TUNEL_WT2-L-G+B.zvi | 5751 KiB | scopeio | ok | 0.008 | 0.009 | 6.8 |
| tests/reference/figshare/24769857/TUNEL_WT2-L-G+B.zvi | 5751 KiB | bioformats | ok | 1.808 | 0.669 | 190.3 |
| tests/reference/figshare/24769857/TUNEL_WT2-L-G+B.zvi | 5751 KiB | bioformats-rs | ok | 0.018 | 0.019 | 21.2 |
| tests/reference/figshare/26880337/Figure 1A+I.zvi | 8200 KiB | scopeio | ok | 0.007 | 0.008 | 6.6 |
| tests/reference/figshare/26880337/Figure 1A+I.zvi | 8200 KiB | bioformats | ok | 2.137 | 0.728 | 232.7 |
| tests/reference/figshare/26880337/Figure 1A+I.zvi | 8200 KiB | bioformats-rs | ok | 0.023 | 0.024 | 27.5 |
| tests/reference/figshare/26880337/Figure 4C.zvi | 5487 KiB | scopeio | ok | 0.008 | 0.009 | 6.6 |
| tests/reference/figshare/26880337/Figure 4C.zvi | 5487 KiB | bioformats | ok | 1.745 | 0.673 | 189.8 |
| tests/reference/figshare/26880337/Figure 4C.zvi | 5487 KiB | bioformats-rs | ok | 0.017 | 0.018 | 22.3 |
| tests/reference/figshare/31813711/RC21-CC_EphA2-647_EDA-FN1-100-546_DAPI.zvi | 8600 KiB | scopeio | ok | 0.007 | 0.008 | 6.7 |
| tests/reference/figshare/31813711/RC21-CC_EphA2-647_EDA-FN1-100-546_DAPI.zvi | 8600 KiB | bioformats | ok | 2.270 | 0.771 | 244.6 |
| tests/reference/figshare/31813711/RC21-CC_EphA2-647_EDA-FN1-100-546_DAPI.zvi | 8600 KiB | bioformats-rs | ok | 0.029 | 0.031 | 28.3 |
| tests/reference/figshare/32769939/a1.zvi | 2900 KiB | scopeio | ok | 0.005 | 0.006 | 6.6 |
| tests/reference/figshare/32769939/a1.zvi | 2900 KiB | bioformats | error: java.lang.NullPointerException: Cannot invoke "loci.p | 0.717 | 0.422 | 123.6 |
| tests/reference/figshare/32769939/a1.zvi | 2900 KiB | bioformats-rs | error: Error opening <file>: Format error: ZVI CFB open erro | 0.016 | 0.017 | 14.0 |
| tests/reference/figshare/6798824/PcJun_shT5_NRC10-48h 30-0098.zvi | 8576 KiB | scopeio | ok | 0.007 | 0.009 | 6.7 |
| tests/reference/figshare/6798824/PcJun_shT5_NRC10-48h 30-0098.zvi | 8576 KiB | bioformats | ok | 2.403 | 0.767 | 243.2 |
| tests/reference/figshare/6798824/PcJun_shT5_NRC10-48h 30-0098.zvi | 8576 KiB | bioformats-rs | ok | 0.030 | 0.031 | 28.3 |
| tests/reference/figshare/6840530/SNAP-102235-0001.zvi | 8576 KiB | scopeio | ok | 0.011 | 0.013 | 12.1 |
| tests/reference/figshare/6840530/SNAP-102235-0001.zvi | 8576 KiB | bioformats | ok | 1.467 | 0.643 | 183.2 |
| tests/reference/figshare/6840530/SNAP-102235-0001.zvi | 8576 KiB | bioformats-rs | ok | 0.031 | 0.033 | 28.8 |
| tests/data/zvi/gray.zvi | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/zvi/gray.zvi | 8 KiB | bioformats | ok | 0.946 | 0.492 | 106.1 |
| tests/data/zvi/gray.zvi | 8 KiB | bioformats-rs | error: Error opening <file>: Format error: ZVI: no image pla | 0.005 | 0.006 | 12.5 |
| tests/data/zvi/colour.zvi | 11 KiB | scopeio | ok | 0.001 | 0.002 | 3.7 |
| tests/data/zvi/colour.zvi | 11 KiB | bioformats | error: java.io.EOFException: Attempting to read beyond end o | 0.707 | 0.401 | 102.9 |
| tests/data/zvi/colour.zvi | 11 KiB | bioformats-rs | error: Error opening <file>: Format error: ZVI: no image pla | 0.005 | 0.006 | 13.0 |

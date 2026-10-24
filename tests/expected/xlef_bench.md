# xlef: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS.xlef | 1 KiB | scopeio | ok | 0.061 | 0.062 | 10.1 |
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS.xlef | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.980 | 1.906 | 710.3 |
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS/XLEF-LOF 2020_06_23_sample 3 U2OS.xlef | 1 KiB | bioformats-rs | ok | 0.164 | 0.167 | 115.8 |
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-TIF 2020_06_23_sample 3 U2OS/2020_06_23_sample 3 U2OS.xlef | 1 KiB | scopeio | ok | 0.160 | 0.162 | 14.5 |
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-TIF 2020_06_23_sample 3 U2OS/2020_06_23_sample 3 U2OS.xlef | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 10.068 | 2.068 | 670.8 |
| tests/reference/ome/Leica-XLEF/2020_06_23_sample 3 U2OS/XLEF-TIF 2020_06_23_sample 3 U2OS/2020_06_23_sample 3 U2OS.xlef | 1 KiB | bioformats-rs | ok | 0.112 | 0.114 | 117.7 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-LOF Lysosomes_actin_in_motion/Lysosomes_actin_in_motion.xlef | 1 KiB | scopeio | ok | 0.044 | 0.047 | 5.5 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-LOF Lysosomes_actin_in_motion/Lysosomes_actin_in_motion.xlef | 1 KiB | bioformats | ok | 9.529 | 2.863 | 662.3 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-LOF Lysosomes_actin_in_motion/Lysosomes_actin_in_motion.xlef | 1 KiB | bioformats-rs | ok | 0.109 | 0.111 | 84.5 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-TIF Lysosomes_actin_in_motion/XLEF-TIF Lysosomes_actin_in_motion.xlef | 1 KiB | scopeio | ok | 0.209 | 0.211 | 5.6 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-TIF Lysosomes_actin_in_motion/XLEF-TIF Lysosomes_actin_in_motion.xlef | 1 KiB | bioformats | ok | 10.195 | 3.497 | 708.5 |
| tests/reference/ome/Leica-XLEF/Lysosomes actin in motion/XLEF-TIF Lysosomes_actin_in_motion/XLEF-TIF Lysosomes_actin_in_motion.xlef | 1 KiB | bioformats-rs | ok | 0.723 | 0.725 | 209.2 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-LOF MaF in LDM 12bit 3s z5/MaF in LDM 12bit 3s z5.xlef | 1 KiB | scopeio | ok | 0.008 | 0.009 | 5.6 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-LOF MaF in LDM 12bit 3s z5/MaF in LDM 12bit 3s z5.xlef | 1 KiB | bioformats | ok | 9.978 | 1.915 | 585.6 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-LOF MaF in LDM 12bit 3s z5/MaF in LDM 12bit 3s z5.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.004 | 0.005 | 12.7 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5.xlef | 1 KiB | scopeio | ok | 0.019 | 0.020 | 7.3 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5.xlef | 1 KiB | bioformats | ok | 7.665 | 1.539 | 600.1 |
| tests/reference/ome/Leica-XLEF/MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5/XLEF-TIF MaF in LDM 12bit 3s z5.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.004 | 0.005 | 13.1 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-LOF MaF in Navigator 12bit 3s z5/MaF in Navigator 12bit 3s z5.xlef | 1 KiB | scopeio | ok | 0.010 | 0.011 | 5.9 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-LOF MaF in Navigator 12bit 3s z5/MaF in Navigator 12bit 3s z5.xlef | 1 KiB | bioformats | ok | 7.467 | 1.435 | 596.5 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-LOF MaF in Navigator 12bit 3s z5/MaF in Navigator 12bit 3s z5.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.003 | 0.004 | 12.5 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5.xlef | 1 KiB | scopeio | ok | 0.018 | 0.019 | 6.9 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5.xlef | 1 KiB | bioformats | ok | 7.414 | 1.494 | 609.2 |
| tests/reference/ome/Leica-XLEF/MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5/XLEF-TIF MaF in Navigator 12bit 3s z5.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | scopeio | ok | 0.041 | 0.043 | 4.7 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | bioformats | ok | 3.379 | 1.272 | 381.3 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected/XLEF-LOF Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | bioformats-rs | ok | 0.118 | 0.120 | 167.1 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-TIF Paramecium RoAvrg 12kRS_selected/Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | scopeio | ok | 0.105 | 0.106 | 5.8 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-TIF Paramecium RoAvrg 12kRS_selected/Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | bioformats | ok | 4.361 | 1.524 | 507.2 |
| tests/reference/ome/Leica-XLEF/Paramecium RoAvrg 12kRS_selected/XLEF-TIF Paramecium RoAvrg 12kRS_selected/Paramecium RoAvrg 12kRS_selected.xlef | 1 KiB | bioformats-rs | ok | 0.193 | 0.194 | 184.0 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-LOF Snail/Schnecke.xlef | 1 KiB | scopeio | ok | 0.477 | 0.479 | 9.9 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-LOF Snail/Schnecke.xlef | 1 KiB | bioformats | ok | 5.391 | 1.674 | 536.0 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-LOF Snail/Schnecke.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.005 | 0.007 | 13.1 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-TIF Snail/XLEF-TIF Snail.xlef | 1 KiB | scopeio | ok | 0.592 | 0.594 | 26.6 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-TIF Snail/XLEF-TIF Snail.xlef | 1 KiB | bioformats | ok | 7.712 | 3.529 | 708.1 |
| tests/reference/ome/Leica-XLEF/Snail/XLEF-TIF Snail/XLEF-TIF Snail.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.005 | 0.006 | 13.1 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-LOF annotated XYCZT dimensions/XLEF-LOF annotated dimensions.xlef | 2 KiB | scopeio | ok | 0.059 | 0.060 | 6.6 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-LOF annotated XYCZT dimensions/XLEF-LOF annotated dimensions.xlef | 2 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.888 | 1.537 | 545.6 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-LOF annotated XYCZT dimensions/XLEF-LOF annotated dimensions.xlef | 2 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.012 | 0.014 | 18.3 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-TIF annotated XYCZT dimensions/annotated dimensions.xlef | 2 KiB | scopeio | ok | 0.185 | 0.186 | 11.8 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-TIF annotated XYCZT dimensions/annotated dimensions.xlef | 2 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.664 | 1.640 | 566.2 |
| tests/reference/ome/Leica-XLEF/annotated XYCZT dimensions/XLEF-TIF annotated XYCZT dimensions/annotated dimensions.xlef | 2 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.015 | 0.016 | 13.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | scopeio | ok | 0.021 | 0.022 | 6.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | bioformats | ok | 3.460 | 1.067 | 439.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | bioformats-rs | ok | 0.059 | 0.061 | 85.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | scopeio | ok | 0.005 | 0.006 | 6.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | bioformats | ok | 2.464 | 0.794 | 241.1 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | bioformats-rs | ok | 0.008 | 0.009 | 20.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | scopeio | ok | 0.024 | 0.025 | 6.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | bioformats | ok | 3.505 | 1.055 | 450.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | bioformats-rs | ok | 0.051 | 0.052 | 86.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | scopeio | ok | 0.008 | 0.009 | 6.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | bioformats | ok | 2.971 | 0.912 | 292.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | bioformats-rs | ok | 0.013 | 0.014 | 27.1 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | scopeio | ok | 0.014 | 0.015 | 6.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | bioformats | ok | 3.286 | 1.018 | 380.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | bioformats-rs | ok | 0.026 | 0.028 | 49.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | scopeio | ok | 0.009 | 0.010 | 6.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | bioformats | ok | 2.744 | 0.898 | 285.6 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | bioformats-rs | ok | 0.021 | 0.022 | 34.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | scopeio | ok | 0.061 | 0.062 | 6.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | bioformats | ok | 3.819 | 1.481 | 452.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | bioformats-rs | ok | 0.144 | 0.145 | 232.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | scopeio | ok | 0.060 | 0.062 | 6.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | bioformats | ok | 3.828 | 1.384 | 456.1 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | bioformats-rs | ok | 0.170 | 0.171 | 232.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | scopeio | ok | 0.014 | 0.016 | 6.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | bioformats | ok | 3.172 | 0.951 | 405.9 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | bioformats-rs | ok | 0.039 | 0.040 | 56.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | scopeio | ok | 0.025 | 0.026 | 6.1 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | bioformats | ok | 3.309 | 1.060 | 447.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-LOF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | bioformats-rs | ok | 0.065 | 0.067 | 85.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | scopeio | ok | 0.068 | 0.069 | 11.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | bioformats | ok | 3.721 | 1.178 | 456.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_13_20_32--XYZCS/XYZCS.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.030 | 0.088 | 18.9 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | scopeio | ok | 0.014 | 0.015 | 9.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | bioformats | ok | 2.652 | 0.825 | 255.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_38_56--XYS/XYS.xlef | 1 KiB | bioformats-rs | ok | 0.007 | 0.008 | 17.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | scopeio | ok | 0.072 | 0.073 | 11.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | bioformats | ok | 3.859 | 1.273 | 470.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_40_10--XYCZS/XYCZS.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.007 | 0.009 | 16.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | scopeio | ok | 0.018 | 0.019 | 11.6 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | bioformats | ok | 3.145 | 0.924 | 295.6 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_41_39--XYCS/XYCS.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.007 | 0.008 | 15.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | scopeio | ok | 0.040 | 0.041 | 11.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | bioformats | ok | 3.011 | 0.957 | 344.2 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_42_36--XYZS/XYZS.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.006 | 0.007 | 15.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | scopeio | ok | 0.024 | 0.025 | 11.7 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | bioformats | ok | 2.794 | 0.898 | 284.9 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_43_13--XYST/XYST.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.006 | 0.008 | 15.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | scopeio | ok | 0.223 | 0.225 | 11.6 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | bioformats | ok | 4.898 | 1.772 | 570.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_44_01--XYCZST/XYCZST.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.026 | 0.027 | 20.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | scopeio | ok | 0.203 | 0.205 | 12.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | bioformats | ok | 4.728 | 1.757 | 571.9 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_01--XYZCST/XYZCST.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.030 | 0.031 | 20.5 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | scopeio | ok | 0.046 | 0.047 | 11.4 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | bioformats | ok | 3.615 | 1.029 | 401.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_45_37--XYCST/XYCST.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.008 | 0.009 | 15.3 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | scopeio | ok | 0.077 | 0.078 | 11.8 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | bioformats | ok | 3.745 | 1.180 | 465.0 |
| tests/reference/ome/Leica-XLEF/annotated stage experiments/XLEF-TIF annotated stage experiments/2021_12_10_14_46_14--XYZST/XYZST.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.008 | 0.009 | 16.3 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-LOF collection test/collection test.xlef | 1 KiB | scopeio | ok | 0.064 | 0.066 | 15.0 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-LOF collection test/collection test.xlef | 1 KiB | bioformats | ok | 1.490 | 0.835 | 230.1 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-LOF collection test/collection test.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.003 | 0.005 | 13.0 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-TIF collection test/collection test.xlef | 1 KiB | scopeio | ok | 0.115 | 0.116 | 37.1 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-TIF collection test/collection test.xlef | 1 KiB | bioformats | ok | 2.275 | 1.293 | 332.2 |
| tests/reference/ome/Leica-XLEF/collection test/XLEF-TIF collection test/collection test.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica XLEF  | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/falcon_sample_data_small.xlef | 1 KiB | scopeio | ok | 0.007 | 0.008 | 5.5 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/falcon_sample_data_small.xlef | 1 KiB | bioformats | ok | 7.175 | 1.345 | 523.2 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/falcon_sample_data_small.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.025 | 0.040 | 14.4 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLIF-TIF falcon_sample_data_small/XLIF-TIF falcon_sample_data_small.xlef | 1 KiB | scopeio | ok | 0.008 | 0.010 | 6.4 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLIF-TIF falcon_sample_data_small/XLIF-TIF falcon_sample_data_small.xlef | 1 KiB | bioformats | ok | 7.764 | 1.558 | 517.5 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLIF-TIF falcon_sample_data_small/XLIF-TIF falcon_sample_data_small.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.023 | 0.028 | 14.2 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/format-test bmp.xlef | 1 KiB | scopeio | ok | 0.025 | 0.027 | 16.3 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/format-test bmp.xlef | 1 KiB | bioformats | ok | 3.931 | 1.113 | 319.8 |
| tests/reference/ome/Leica-XLEF/format test/format test BMP/format-test bmp.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.015 | 0.016 | 23.5 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG TIF mixed/format-test jpeg tif mixed.xlef | 1 KiB | scopeio | ok | 0.107 | 0.108 | 53.9 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG TIF mixed/format-test jpeg tif mixed.xlef | 1 KiB | bioformats | ok | 7.893 | 2.082 | 444.2 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG TIF mixed/format-test jpeg tif mixed.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.018 | 0.020 | 29.7 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG/format-test jpeg.xlef | 1 KiB | scopeio | ok | 0.042 | 0.043 | 22.7 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG/format-test jpeg.xlef | 1 KiB | bioformats | ok | 4.245 | 1.334 | 388.3 |
| tests/reference/ome/Leica-XLEF/format test/format test JPEG/format-test jpeg.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.014 | 0.016 | 29.6 |
| tests/reference/ome/Leica-XLEF/format test/format test LOF/format-test lof.xlef | 1 KiB | scopeio | ok | 0.055 | 0.057 | 17.3 |
| tests/reference/ome/Leica-XLEF/format test/format test LOF/format-test lof.xlef | 1 KiB | bioformats | ok | 6.795 | 1.497 | 621.4 |
| tests/reference/ome/Leica-XLEF/format test/format test LOF/format-test lof.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.011 | 0.012 | 18.3 |
| tests/reference/ome/Leica-XLEF/format test/format test PNG/format-test png.xlef | 1 KiB | scopeio | ok | 0.035 | 0.037 | 17.3 |
| tests/reference/ome/Leica-XLEF/format test/format test PNG/format-test png.xlef | 1 KiB | bioformats | ok | 3.742 | 1.043 | 349.5 |
| tests/reference/ome/Leica-XLEF/format test/format test PNG/format-test png.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.017 | 0.019 | 18.6 |
| tests/reference/ome/Leica-XLEF/format test/format test TIF/format-test tif.xlef | 1 KiB | scopeio | ok | 0.102 | 0.104 | 48.5 |
| tests/reference/ome/Leica-XLEF/format test/format test TIF/format-test tif.xlef | 1 KiB | bioformats | ok | 7.315 | 1.906 | 639.5 |
| tests/reference/ome/Leica-XLEF/format test/format test TIF/format-test tif.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.008 | 0.010 | 13.4 |
| tests/reference/ome/Leica-XLEF/format test/format-test TIF compressed/format-test TIF compressed.xlef | 1 KiB | scopeio | ok | 0.127 | 0.128 | 41.5 |
| tests/reference/ome/Leica-XLEF/format test/format-test TIF compressed/format-test TIF compressed.xlef | 1 KiB | bioformats | ok | 7.058 | 1.780 | 566.1 |
| tests/reference/ome/Leica-XLEF/format test/format-test TIF compressed/format-test TIF compressed.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.005 | 0.007 | 13.2 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-LOF xyct 16bit 2ch t11/xyct 16bit 2ch t11.xlef | 1 KiB | scopeio | ok | 0.006 | 0.007 | 4.8 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-LOF xyct 16bit 2ch t11/xyct 16bit 2ch t11.xlef | 1 KiB | bioformats | ok | 4.604 | 1.101 | 366.6 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-LOF xyct 16bit 2ch t11/xyct 16bit 2ch t11.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.011 | 0.012 | 24.6 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11.xlef | 1 KiB | scopeio | ok | 0.016 | 0.017 | 6.7 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11.xlef | 1 KiB | bioformats | ok | 4.493 | 1.109 | 356.3 |
| tests/reference/ome/Leica-XLEF/xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11/XLEF-TIF xyct 16bit 2ch t11.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.006 | 0.007 | 14.0 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Image001.lof | 655 KiB | scopeio | ok | 0.002 | 0.003 | 4.7 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Image001.lof | 655 KiB | bioformats | ok | 4.899 | 2.544 | 480.9 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Image001.lof | 655 KiB | bioformats-rs | ok | 0.608 | 0.609 | 747.3 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Series002.lof | 656 KiB | scopeio | ok | 0.002 | 0.003 | 4.4 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Series002.lof | 656 KiB | bioformats | ok | 5.103 | 2.578 | 491.1 |
| tests/reference/ome/Leica-XLEF/falcon_sample_data_small/XLEF-LOF falcon_sample_data_small/Series002.lof | 656 KiB | bioformats-rs | ok | 0.006 | 0.008 | 14.4 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef | 1 KiB | scopeio | ok | 0.993 | 0.995 | 6.4 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 5.685 | 1.935 | 700.2 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.080 | 0.082 | 177.5 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef | 1 KiB | scopeio | ok | 1.939 | 1.945 | 22.8 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 9.636 | 4.905 | 714.3 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: Leica LOF X | 0.009 | 0.010 | 13.7 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 0) | 1 KiB | scopeio | ok | 0.473 | 0.474 | 6.2 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 0) | 1 KiB | bioformats | ok | 5.465 | 1.483 | 658.7 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 0) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 1) | 1 KiB | scopeio | ok | 0.507 | 0.508 | 6.4 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 1) | 1 KiB | bioformats | ok | 5.286 | 1.566 | 592.6 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 1) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 2) | 1 KiB | scopeio | ok | 0.068 | 0.070 | 6.6 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 2) | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 4.794 | 1.046 | 430.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-LOF rgb channel test/XLEF-LOF rgb channel test.xlef (series 2) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 0) | 1 KiB | scopeio | ok | 0.937 | 0.939 | 22.9 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 0) | 1 KiB | bioformats | ok | 8.286 | 3.486 | 691.9 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 0) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 1) | 1 KiB | scopeio | ok | 0.986 | 0.988 | 22.8 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 1) | 1 KiB | bioformats | ok | 8.219 | 3.418 | 683.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 1) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 2) | 1 KiB | scopeio | ok | 0.140 | 0.142 | 22.6 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 2) | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 5.570 | 1.231 | 481.8 |
| tests/reference/ome/Leica-XLEF/rgb channel test/XLEF-TIF rgb channel test/rgb channel test.xlef (series 2) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef | 1 KiB | scopeio | ok | 0.088 | 0.090 | 6.2 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef | 1 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 8.194 | 1.706 | 687.3 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef | 1 KiB | bioformats-rs | ok | 0.118 | 0.120 | 154.9 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 0) | 1 KiB | scopeio | ok | 0.007 | 0.008 | 6.2 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 0) | 1 KiB | bioformats | ok | 7.622 | 1.422 | 549.5 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 0) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 1) | 1 KiB | scopeio | ok | 0.014 | 0.015 | 6.6 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 1) | 1 KiB | bioformats | ok | 7.795 | 1.606 | 604.4 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 1) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 2) | 1 KiB | scopeio | ok | 0.021 | 0.022 | 6.4 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 2) | 1 KiB | bioformats | ok | 7.963 | 1.704 | 639.5 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 2) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 3) | 1 KiB | scopeio | ok | 0.022 | 0.023 | 6.1 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 3) | 1 KiB | bioformats | ok | 8.621 | 1.662 | 632.5 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 3) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 4) | 1 KiB | scopeio | ok | 0.049 | 0.050 | 6.2 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 4) | 1 KiB | bioformats | ok | 9.168 | 2.086 | 693.4 |
| tests/reference/ome/Leica-XLEF/tau_selection/XLEF-LOF tau_selection/tau_selection.xlef (series 4) | 1 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-LOF Thunder Organoid Tilescan/XLEF-LOF Thunder Organoid Tilescan.xlef | 1 KiB | scopeio | ok | 0.049 | 0.051 | 4.6 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-LOF Thunder Organoid Tilescan/XLEF-LOF Thunder Organoid Tilescan.xlef | 1 KiB | bioformats | ok | 4.039 | 1.740 | 443.3 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-LOF Thunder Organoid Tilescan/XLEF-LOF Thunder Organoid Tilescan.xlef | 1 KiB | bioformats-rs | ok | 0.078 | 0.079 | 156.4 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-TIF Thunder Organoid Tilescan/XLEF-TIF Thunder Organoid Tilescan.xlef | 1 KiB | scopeio | ok | 0.161 | 0.162 | 6.4 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-TIF Thunder Organoid Tilescan/XLEF-TIF Thunder Organoid Tilescan.xlef | 1 KiB | bioformats | ok | 5.604 | 2.198 | 540.2 |
| tests/reference/ome/Leica-XLEF/thunder organoid tilescan/XLEF-TIF Thunder Organoid Tilescan/XLEF-TIF Thunder Organoid Tilescan.xlef | 1 KiB | bioformats-rs | error: Error reading plane 1: Plane index 1 out of range | 0.096 | 0.097 | 26.9 |

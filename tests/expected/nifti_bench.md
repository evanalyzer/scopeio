# nifti: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii | 882 KiB | scopeio | ok | 0.003 | 0.004 | 4.0 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii | 882 KiB | bioformats | ok | 1.505 | 0.803 | 222.0 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii | 882 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.2 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.nii | 882 KiB | scopeio | ok | 0.003 | 0.004 | 4.1 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.nii | 882 KiB | bioformats | ok | 1.473 | 0.780 | 221.2 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.nii | 882 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.2 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii | 30240 KiB | scopeio | ok | 0.067 | 0.069 | 4.1 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii | 30240 KiB | bioformats | ok | 6.327 | 4.631 | 352.5 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii | 30240 KiB | bioformats-rs | ok | 0.058 | 0.060 | 70.6 |
| tests/reference/ome/NIfTI/NIH/minimal.nii | 40 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/reference/ome/NIfTI/NIH/minimal.nii | 40 KiB | bioformats | ok | 1.010 | 0.512 | 110.5 |
| tests/reference/ome/NIfTI/NIH/minimal.nii | 40 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.8 |
| tests/reference/ome/NIfTI/NIH/newsirp_final_XML.nii | 41346 KiB | scopeio | ok | 0.087 | 0.089 | 4.2 |
| tests/reference/ome/NIfTI/NIH/newsirp_final_XML.nii | 41346 KiB | bioformats | ok | 10.146 | 8.419 | 353.2 |
| tests/reference/ome/NIfTI/NIH/newsirp_final_XML.nii | 41346 KiB | bioformats-rs | ok | 0.079 | 0.081 | 92.2 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii | 336 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii | 336 KiB | bioformats | ok | 1.154 | 0.568 | 134.6 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii | 336 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.0 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.hdr | 0 KiB | scopeio | ok | 0.004 | 0.005 | 3.9 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.hdr | 0 KiB | bioformats | ok | 1.411 | 0.751 | 214.4 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.hdr | 0 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.2 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.hdr | 0 KiB | scopeio | ok | 0.003 | 0.005 | 4.1 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.hdr | 0 KiB | bioformats | ok | 1.548 | 0.825 | 214.5 |
| tests/reference/ome/NIfTI/NIH/avg152T1_RL_nifti.hdr | 0 KiB | bioformats-rs | ok | 0.004 | 0.006 | 13.1 |
| tests/reference/ome/NIfTI/NIH/minimal.hdr | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.1 |
| tests/reference/ome/NIfTI/NIH/minimal.hdr | 0 KiB | bioformats | ok | 1.074 | 0.546 | 110.4 |
| tests/reference/ome/NIfTI/NIH/minimal.hdr | 0 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.7 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii.gz | 519 KiB | scopeio | ok | 0.004 | 0.005 | 5.3 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii.gz | 519 KiB | bioformats | ok | 1.691 | 0.963 | 247.2 |
| tests/reference/ome/NIfTI/NIH/avg152T1_LR_nifti.nii.gz | 519 KiB | bioformats-rs | ok | 0.209 | 0.211 | 14.0 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii.gz | 6625 KiB | scopeio | ok | 0.051 | 0.052 | 40.0 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii.gz | 6625 KiB | bioformats | ok | 8.900 | 6.993 | 600.8 |
| tests/reference/ome/NIfTI/NIH/filtered_func_data.nii.gz | 6625 KiB | bioformats-rs | ok | 149.367 | 149.528 | 75.9 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii.gz | 71 KiB | scopeio | ok | 0.002 | 0.002 | 4.5 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii.gz | 71 KiB | bioformats | ok | 1.186 | 0.607 | 180.2 |
| tests/reference/ome/NIfTI/NIH/zstat1.nii.gz | 71 KiB | bioformats-rs | ok | 0.009 | 0.010 | 12.5 |
| tests/data/nifti/le_u16_5d.nii | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/nifti/le_u16_5d.nii | 1 KiB | bioformats | ok | 0.999 | 0.509 | 110.9 |
| tests/data/nifti/le_u16_5d.nii | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.9 |
| tests/data/nifti/rgb.nii | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/nifti/rgb.nii | 1 KiB | bioformats | ok | 0.972 | 0.518 | 106.4 |
| tests/data/nifti/rgb.nii | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.5 |
| tests/data/nifti/v2_f64.nii.gz | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/nifti/v2_f64.nii.gz | 0 KiB | bioformats | error: loci.formats.FormatException: Unsupported data type:  | 0.754 | 0.431 | 149.6 |
| tests/data/nifti/v2_f64.nii.gz | 0 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: NIfTI/Analy | 0.003 | 0.004 | 11.9 |
| tests/data/nifti/v2_pair.hdr | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.9 |
| tests/data/nifti/v2_pair.hdr | 1 KiB | bioformats | error: loci.formats.UnknownFormatException: Unknown file for | 0.697 | 0.400 | 101.4 |
| tests/data/nifti/v2_pair.hdr | 1 KiB | bioformats-rs | error: Error opening <file>: Format error: Format error deco | 0.003 | 0.004 | 12.2 |

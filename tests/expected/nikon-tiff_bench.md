# nikon-tiff: CPU and RAM compared with the reference readers

Generated 2026-10-25 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/zenodo-7677868/h4k20me002xy13c2.tif | 8218 KiB | scopeio | ok | 0.005 | 0.006 | 12.2 |
| tests/reference/zenodo-7677868/h4k20me002xy13c2.tif | 8218 KiB | bioformats | ok | 1.107 | 0.574 | 175.9 |
| tests/reference/zenodo-7677868/h4k20me002xy13c2.tif | 8218 KiB | bioformats-rs | ok | 0.014 | 0.015 | 25.9 |
| tests/reference/zenodo-7677827/single_plane_1-2000_2xy01c1.tif | 8218 KiB | scopeio | ok | 0.005 | 0.006 | 12.0 |
| tests/reference/zenodo-7677827/single_plane_1-2000_2xy01c1.tif | 8218 KiB | bioformats | ok | 1.263 | 0.611 | 175.4 |
| tests/reference/zenodo-7677827/single_plane_1-2000_2xy01c1.tif | 8218 KiB | bioformats-rs | ok | 0.014 | 0.015 | 25.6 |
| tests/reference/zenodo-8006449/D4_E1_M_ROI.tif | 8833 KiB | scopeio | ok | 0.006 | 0.006 | 12.8 |
| tests/reference/zenodo-8006449/D4_E1_M_ROI.tif | 8833 KiB | bioformats | ok | 1.787 | 0.943 | 220.8 |
| tests/reference/zenodo-8006449/D4_E1_M_ROI.tif | 8833 KiB | bioformats-rs | ok | 0.038 | 0.039 | 26.9 |
| tests/reference/zenodo-8245566/C12_E3_ROI.tif | 17701 KiB | scopeio | ok | 0.009 | 0.011 | 21.2 |
| tests/reference/zenodo-8245566/C12_E3_ROI.tif | 17701 KiB | bioformats | ok | 1.800 | 1.083 | 279.7 |
| tests/reference/zenodo-8245566/C12_E3_ROI.tif | 17701 KiB | bioformats-rs | ok | 0.071 | 0.072 | 43.9 |
| tests/data/nikon-tiff/gray.tif | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.2 |
| tests/data/nikon-tiff/gray.tif | 1 KiB | bioformats | ok | 1.203 | 0.615 | 104.5 |
| tests/data/nikon-tiff/gray.tif | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.9 |

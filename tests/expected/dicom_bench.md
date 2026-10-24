# dicom: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 0) crop 20000,15000,2048,2048 | 119021 KiB | scopeio | ok | 0.202 | 0.204 | 18.2 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 0) crop 20000,15000,2048,2048 | 119021 KiB | bioformats | ok | 6.074 | 1.927 | 551.6 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 0) crop 20000,15000,2048,2048 | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 1) crop 4000,3000,1024,1024 | 119021 KiB | scopeio | ok | 0.067 | 0.069 | 9.2 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 1) crop 4000,3000,1024,1024 | 119021 KiB | bioformats | ok | 5.103 | 1.588 | 446.5 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 1) crop 4000,3000,1024,1024 | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 2) | 119021 KiB | scopeio | ok | 0.295 | 0.297 | 22.8 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 2) | 119021 KiB | bioformats | ok | 6.576 | 2.123 | 586.5 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 2) | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 0) crop 10000,10000,2048,2048 | 125 KiB | scopeio | ok | 0.032 | 0.033 | 28.1 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 0) crop 10000,10000,2048,2048 | 125 KiB | bioformats | ok | 4.731 | 1.312 | 519.3 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 0) crop 10000,10000,2048,2048 | 125 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 1) crop 2000,2000,1024,1024 | 125 KiB | scopeio | ok | 0.021 | 0.022 | 16.3 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 1) crop 2000,2000,1024,1024 | 125 KiB | bioformats | ok | 4.401 | 1.282 | 451.1 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 1) crop 2000,2000,1024,1024 | 125 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 2) | 125 KiB | scopeio | ok | 0.025 | 0.026 | 16.4 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 2) | 125 KiB | bioformats | ok | 4.415 | 1.291 | 504.5 |
| tests/reference/openslide/DICOM/Leica-4/1.3.6.1.4.1.36533.116129230228107214763613716719238114924751.dcm (series 2) | 125 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 0) crop 15000,20000,2048,2048 | 156209 KiB | scopeio | ok | 0.054 | 0.056 | 17.0 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 0) crop 15000,20000,2048,2048 | 156209 KiB | bioformats | ok | 3.498 | 1.472 | 447.3 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 0) crop 15000,20000,2048,2048 | 156209 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 3) | 156209 KiB | scopeio | ok | 0.095 | 0.096 | 58.7 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 3) | 156209 KiB | bioformats | ok | 3.799 | 1.637 | 519.8 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 3) | 156209 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 7) | 156209 KiB | scopeio | ok | 0.016 | 0.017 | 5.1 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 7) | 156209 KiB | bioformats | ok | 3.043 | 1.304 | 344.5 |
| tests/reference/openslide/DICOM/Hamamatsu-2/A1 - 2026-05-29 10.08.06.1.dcm (series 7) | 156209 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 0) crop 28000,30000,2048,2048 | 1056 KiB | scopeio | ok | 0.055 | 0.056 | 22.3 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 0) crop 28000,30000,2048,2048 | 1056 KiB | bioformats | ok | 5.792 | 1.812 | 673.8 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 0) crop 28000,30000,2048,2048 | 1056 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 4) | 1056 KiB | scopeio | ok | 0.083 | 0.084 | 48.6 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 4) | 1056 KiB | bioformats | ok | 5.786 | 1.932 | 702.1 |
| tests/reference/openslide/DICOM/3DHISTECH-1/000001.dcm (series 4) | 1056 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 3) | 119021 KiB | scopeio | ok | 48.408 | 48.422 | 4334.1 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 3) | 119021 KiB | bioformats | ok | 3.386 | 1.212 | 416.2 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 3) | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 4) | 119021 KiB | scopeio | ok | 49.469 | 49.491 | 4334.7 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 4) | 119021 KiB | bioformats | ok | 3.585 | 1.262 | 415.2 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 4) | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 5) | 119021 KiB | scopeio | ok | 47.984 | 48.001 | 4334.9 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 5) | 119021 KiB | bioformats | ok | 3.561 | 1.223 | 409.8 |
| tests/reference/openslide/DICOM/CMU-1-JP2K-33005/DCM_0.dcm (series 5) | 119021 KiB | bioformats-rs | skipped: no series or crop option | 0.000 | 0.000 | 0.0 |
| tests/data/dicom/wsi/level0.dcm | 5 KiB | scopeio | ok | 0.001 | 0.002 | 4.0 |
| tests/data/dicom/wsi/level0.dcm | 5 KiB | bioformats | ok | 1.301 | 0.538 | 126.3 |
| tests/data/dicom/wsi/level0.dcm | 5 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.8 |
| tests/data/dicom/fluo/level0.dcm | 3 KiB | scopeio | ok | 0.001 | 0.002 | 3.5 |
| tests/data/dicom/fluo/level0.dcm | 3 KiB | bioformats | ok | 1.323 | 0.533 | 129.7 |
| tests/data/dicom/fluo/level0.dcm | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 9.6 |
| tests/data/dicom/plain.dcm | 1 KiB | scopeio | ok | 0.001 | 0.002 | 3.8 |
| tests/data/dicom/plain.dcm | 1 KiB | bioformats | ok | 1.276 | 0.529 | 126.1 |
| tests/data/dicom/plain.dcm | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 9.3 |

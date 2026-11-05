# amira: CPU and RAM compared with the reference readers

Generated 2026-11-05 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/AmiraMesh/ignacio/test.am | 18 KiB | scopeio | ok | 0.002 | 0.004 | 7.4 |
| tests/reference/ome/AmiraMesh/ignacio/test.am | 18 KiB | bioformats | ok | 1.181 | 0.618 | 260.6 |
| tests/reference/ome/AmiraMesh/ignacio/test.am | 18 KiB | bioformats-rs | ok | 0.007 | 0.008 | 19.7 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckAmira.am | 14 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckAmira.am | 14 KiB | bioformats | ok | 0.822 | 0.446 | 104.6 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckAmira.am | 14 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.0 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckMIB.am | 11 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckMIB.am | 11 KiB | bioformats | ok | 0.820 | 0.454 | 105.2 |
| tests/reference/ome/AmiraMesh/imagesc-50585/BF_CheckMIB.am | 11 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.8 |
| tests/reference/ome/AmiraMesh/zenodo-18337391/Electron_tomogram.am | 232681 KiB | scopeio | ok | 0.071 | 0.072 | 6.8 |
| tests/reference/ome/AmiraMesh/zenodo-18337391/Electron_tomogram.am | 232681 KiB | bioformats | ok | 2.144 | 1.515 | 332.8 |
| tests/reference/ome/AmiraMesh/zenodo-18337391/Electron_tomogram.am | 232681 KiB | bioformats-rs | error: Error opening <file>: IO error: stream did not contai | 0.090 | 0.091 | 237.7 |
| tests/data/amira/ascii_float.am | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.2 |
| tests/data/amira/ascii_float.am | 1 KiB | bioformats | ok | 0.808 | 0.444 | 104.7 |
| tests/data/amira/ascii_float.am | 1 KiB | bioformats-rs | error: Error reading plane 0: Invalid data: Amira ASCII plan | 0.002 | 0.003 | 11.7 |
| tests/data/amira/be_short.am | 0 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/data/amira/be_short.am | 0 KiB | bioformats | error: java.lang.IllegalArgumentException: 0 must be non-nul | 0.706 | 0.399 | 102.5 |
| tests/data/amira/be_short.am | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.9 |
| tests/data/amira/labels.am | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.2 |
| tests/data/amira/labels.am | 1 KiB | bioformats | ok | 0.829 | 0.445 | 106.4 |
| tests/data/amira/labels.am | 1 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.8 |
| tests/data/amira/le_ushort.am | 0 KiB | scopeio | ok | 0.001 | 0.002 | 5.0 |
| tests/data/amira/le_ushort.am | 0 KiB | bioformats | ok | 0.826 | 0.456 | 106.0 |
| tests/data/amira/le_ushort.am | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.0 |
| tests/data/amira/rle_byte.am | 0 KiB | scopeio | ok | 0.001 | 0.002 | 5.2 |
| tests/data/amira/rle_byte.am | 0 KiB | bioformats | error: loci.formats.FormatException: Slice 1 is unaligned! | 0.815 | 0.455 | 105.2 |
| tests/data/amira/rle_byte.am | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.8 |
| tests/data/amira/rle_slice.am | 0 KiB | scopeio | ok | 0.001 | 0.001 | 5.2 |
| tests/data/amira/rle_slice.am | 0 KiB | bioformats | ok | 0.805 | 0.450 | 104.5 |
| tests/data/amira/rle_slice.am | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 12.0 |
| tests/data/amira/stacked_double.am | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.0 |
| tests/data/amira/stacked_double.am | 1 KiB | bioformats | ok | 0.816 | 0.454 | 105.4 |
| tests/data/amira/stacked_double.am | 1 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.9 |
| tests/data/amira/vector3.am | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/data/amira/vector3.am | 1 KiB | bioformats | ok | 0.826 | 0.464 | 104.8 |
| tests/data/amira/vector3.am | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.8 |
| tests/data/amira/zip_int.am | 0 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/data/amira/zip_int.am | 0 KiB | bioformats | ok | 0.805 | 0.448 | 105.7 |
| tests/data/amira/zip_int.am | 0 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.9 |
| tests/data/amira/zip_nosize.am | 0 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/data/amira/zip_nosize.am | 0 KiB | bioformats | ok | 0.819 | 0.457 | 105.5 |
| tests/data/amira/zip_nosize.am | 0 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.0 |

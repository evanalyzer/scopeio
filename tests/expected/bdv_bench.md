# bdv: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/BDV/samples/drosophila.xml | 3 KiB | scopeio | ok | 61.988 | 62.409 | 55.5 |
| tests/reference/ome/BDV/samples/drosophila.xml | 3 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 126.105 | 124.319 | 452.0 |
| tests/reference/ome/BDV/samples/drosophila.xml | 3 KiB | bioformats-rs | ok | 112.491 | 112.838 | 3039.6 |
| tests/reference/ome/BDV/samples/HisYFP-SPIM.xml | 10 KiB | scopeio | ok | 92.266 | 92.305 | 31.7 |
| tests/reference/ome/BDV/samples/HisYFP-SPIM.xml | 10 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 34.366 | 33.469 | 434.0 |
| tests/reference/ome/BDV/samples/HisYFP-SPIM.xml | 10 KiB | bioformats-rs | ok | 35.232 | 35.303 | 459.2 |
| tests/data/bdv/synthetic.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/bdv/synthetic.xml | 2 KiB | bioformats | error: hdf.hdf5lib.exceptions.HDF5HeapException: Bad value [ | 0.889 | 0.497 | 107.9 |
| tests/data/bdv/synthetic.xml | 2 KiB | bioformats-rs | error: Error reading plane 0: Format error: HDF5 read: Inval | 0.005 | 0.006 | 13.9 |

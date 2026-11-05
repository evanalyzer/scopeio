# dcimg: CPU and RAM compared with the reference readers

Generated 2026-11-05 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/DCIMG/single/cell07/cell07.dcimg | 6721 KiB | scopeio | ok | 0.002 | 0.003 | 6.1 |
| tests/reference/ome/DCIMG/single/cell07/cell07.dcimg | 6721 KiB | bioformats | ok | 0.931 | 0.495 | 181.6 |
| tests/reference/ome/DCIMG/single/cell07/cell07.dcimg | 6721 KiB | bioformats-rs | ok | 0.012 | 0.013 | 24.8 |
| tests/reference/ome/DCIMG/single/cell09-a/cell09-a.dcimg | 673 KiB | scopeio | ok | 0.001 | 0.002 | 6.3 |
| tests/reference/ome/DCIMG/single/cell09-a/cell09-a.dcimg | 673 KiB | bioformats | ok | 0.804 | 0.443 | 121.7 |
| tests/reference/ome/DCIMG/single/cell09-a/cell09-a.dcimg | 673 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.1 |
| tests/reference/ome/DCIMG/single/cell09-b/cell09-b.dcimg | 673 KiB | scopeio | ok | 0.002 | 0.002 | 6.2 |
| tests/reference/ome/DCIMG/single/cell09-b/cell09-b.dcimg | 673 KiB | bioformats | ok | 0.822 | 0.453 | 121.7 |
| tests/reference/ome/DCIMG/single/cell09-b/cell09-b.dcimg | 673 KiB | bioformats-rs | ok | 0.004 | 0.004 | 13.0 |
| tests/reference/ome/DCIMG/single/bead-a/bead-a.dcimg | 801 KiB | scopeio | ok | 0.001 | 0.002 | 6.4 |
| tests/reference/ome/DCIMG/single/bead-a/bead-a.dcimg | 801 KiB | bioformats | ok | 0.810 | 0.457 | 125.4 |
| tests/reference/ome/DCIMG/single/bead-a/bead-a.dcimg | 801 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.4 |
| tests/reference/ome/DCIMG/single/bead-b/bead-b.dcimg | 801 KiB | scopeio | ok | 0.002 | 0.002 | 6.2 |
| tests/reference/ome/DCIMG/single/bead-b/bead-b.dcimg | 801 KiB | bioformats | ok | 0.823 | 0.460 | 124.5 |
| tests/reference/ome/DCIMG/single/bead-b/bead-b.dcimg | 801 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.4 |
| tests/reference/ome/DCIMG/single/beads1000/beads1000.dcimg | 800048 KiB | scopeio | ok | 0.101 | 0.103 | 6.3 |
| tests/reference/ome/DCIMG/single/beads1000/beads1000.dcimg | 800048 KiB | bioformats | ok | 3.325 | 2.165 | 348.9 |
| tests/reference/ome/DCIMG/single/beads1000/beads1000.dcimg | 800048 KiB | bioformats-rs | ok | 1.178 | 1.180 | 1574.9 |
| tests/reference/ome/DCIMG/single/dense2000/dense2000.dcimg | 1600095 KiB | scopeio | ok | 0.195 | 0.198 | 6.2 |
| tests/reference/ome/DCIMG/single/dense2000/dense2000.dcimg | 1600095 KiB | bioformats | ok | 7.849 | 6.253 | 356.3 |
| tests/reference/ome/DCIMG/single/dense2000/dense2000.dcimg | 1600095 KiB | bioformats-rs | ok | 2.299 | 2.302 | 3137.4 |
| tests/data/dcimg/framed_plain/framed_plain.dcimg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/dcimg/framed_plain/framed_plain.dcimg | 1 KiB | bioformats | ok | 0.805 | 0.443 | 104.0 |
| tests/data/dcimg/framed_plain/framed_plain.dcimg | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.9 |
| tests/data/dcimg/framed_stored/framed_stored.dcimg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/dcimg/framed_stored/framed_stored.dcimg | 1 KiB | bioformats | ok | 0.786 | 0.438 | 105.1 |
| tests/data/dcimg/framed_stored/framed_stored.dcimg | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 12.0 |
| tests/data/dcimg/packed_stored/packed_stored.dcimg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/dcimg/packed_stored/packed_stored.dcimg | 1 KiB | bioformats | ok | 0.801 | 0.445 | 104.5 |
| tests/data/dcimg/packed_stored/packed_stored.dcimg | 1 KiB | bioformats-rs | ok | 0.003 | 0.003 | 11.9 |
| tests/data/dcimg/packed_u8/packed_u8.dcimg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/data/dcimg/packed_u8/packed_u8.dcimg | 1 KiB | bioformats | ok | 0.783 | 0.446 | 104.3 |
| tests/data/dcimg/packed_u8/packed_u8.dcimg | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.9 |
| tests/data/dcimg/truncated/truncated.dcimg | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/dcimg/truncated/truncated.dcimg | 1 KiB | bioformats | ok | 0.790 | 0.446 | 104.8 |
| tests/data/dcimg/truncated/truncated.dcimg | 1 KiB | bioformats-rs | error: Error opening <file>: Unsupported format: DCIMG pixel | 0.003 | 0.003 | 12.0 |

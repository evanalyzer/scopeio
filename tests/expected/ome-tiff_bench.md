# ome-tiff: CPU and RAM compared with the reference readers

Generated 2026-10-24 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/data/ome-tiff/pyramid_u16.ome.tif | 125 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/data/ome-tiff/pyramid_u16.ome.tif | 125 KiB | bioformats | ok | 1.170 | 0.619 | 107.9 |
| tests/data/ome-tiff/pyramid_u16.ome.tif | 125 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.1 |
| tests/reference/ome/OME-TIFF/2007-06/single-channel.ome.tif | 41 KiB | scopeio | ok | 0.001 | 0.001 | 2.7 |
| tests/reference/ome/OME-TIFF/2007-06/single-channel.ome.tif | 41 KiB | bioformats | ok | 1.539 | 0.736 | 125.6 |
| tests/reference/ome/OME-TIFF/2007-06/single-channel.ome.tif | 41 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.8 |
| tests/reference/ome/OME-TIFF/2008-02/single-channel.ome.tif | 41 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2008-02/single-channel.ome.tif | 41 KiB | bioformats | ok | 1.561 | 0.747 | 125.9 |
| tests/reference/ome/OME-TIFF/2008-02/single-channel.ome.tif | 41 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/reference/ome/OME-TIFF/2008-09/instrument.ome.tiff | 4 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2008-09/instrument.ome.tiff | 4 KiB | bioformats | ok | 1.666 | 0.758 | 119.8 |
| tests/reference/ome/OME-TIFF/2008-09/instrument.ome.tiff | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.7 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-time-series.ome.tiff | 43 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-time-series.ome.tiff | 43 KiB | bioformats | ok | 1.770 | 0.802 | 124.9 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-time-series.ome.tiff | 43 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series-time-series.ome.tiff | 617 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series-time-series.ome.tiff | 617 KiB | bioformats | ok | 2.450 | 0.879 | 198.8 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series-time-series.ome.tiff | 617 KiB | bioformats-rs | ok | 0.012 | 0.013 | 13.9 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series.ome.tiff | 48 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series.ome.tiff | 48 KiB | bioformats | ok | 1.680 | 0.762 | 125.2 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel-z-series.ome.tiff | 48 KiB | bioformats-rs | ok | 0.005 | 0.005 | 12.6 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel.ome.tiff | 9 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel.ome.tiff | 9 KiB | bioformats | ok | 1.544 | 0.748 | 123.1 |
| tests/reference/ome/OME-TIFF/2008-09/multi-channel.ome.tiff | 9 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-aquired.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-aquired.ome.tiff | 3 KiB | bioformats | ok | 1.514 | 0.730 | 116.9 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-aquired.ome.tiff | 3 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.7 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-default.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-default.ome.tiff | 3 KiB | bioformats | ok | 1.452 | 0.702 | 117.4 |
| tests/reference/ome/OME-TIFF/2008-09/multi-pixel-default.ome.tiff | 3 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.7 |
| tests/reference/ome/OME-TIFF/2008-09/single-image.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/single-image.ome.tiff | 2 KiB | bioformats | ok | 1.471 | 0.717 | 116.8 |
| tests/reference/ome/OME-TIFF/2008-09/single-image.ome.tiff | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.5 |
| tests/reference/ome/OME-TIFF/2008-09/time-series.ome.tiff | 16 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/time-series.ome.tiff | 16 KiB | bioformats | ok | 1.529 | 0.720 | 120.1 |
| tests/reference/ome/OME-TIFF/2008-09/time-series.ome.tiff | 16 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.2 |
| tests/reference/ome/OME-TIFF/2008-09/z-series-time-series.ome.tiff | 181 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2008-09/z-series-time-series.ome.tiff | 181 KiB | bioformats | ok | 1.807 | 0.787 | 145.9 |
| tests/reference/ome/OME-TIFF/2008-09/z-series-time-series.ome.tiff | 181 KiB | bioformats-rs | ok | 0.006 | 0.007 | 13.2 |
| tests/reference/ome/OME-TIFF/2008-09/z-series.ome.tiff | 19 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2008-09/z-series.ome.tiff | 19 KiB | bioformats | ok | 1.532 | 0.751 | 120.4 |
| tests/reference/ome/OME-TIFF/2008-09/z-series.ome.tiff | 19 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2009-09/instrument.ome.tiff | 4 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/instrument.ome.tiff | 4 KiB | bioformats | ok | 1.490 | 0.694 | 117.3 |
| tests/reference/ome/OME-TIFF/2009-09/instrument.ome.tiff | 4 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-time-series.ome.tiff | 45 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-time-series.ome.tiff | 45 KiB | bioformats | ok | 1.521 | 0.710 | 120.7 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-time-series.ome.tiff | 45 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.7 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series-time-series.ome.tiff | 627 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series-time-series.ome.tiff | 627 KiB | bioformats | ok | 2.104 | 0.818 | 194.4 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series-time-series.ome.tiff | 627 KiB | bioformats-rs | ok | 0.011 | 0.012 | 14.0 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series.ome.tiff | 45 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series.ome.tiff | 45 KiB | bioformats | ok | 1.510 | 0.707 | 121.0 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel-z-series.ome.tiff | 45 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel.ome.tiff | 9 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel.ome.tiff | 9 KiB | bioformats | ok | 1.400 | 0.700 | 114.3 |
| tests/reference/ome/OME-TIFF/2009-09/multi-channel.ome.tiff | 9 KiB | bioformats-rs | ok | 0.004 | 0.004 | 13.0 |
| tests/reference/ome/OME-TIFF/2009-09/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats | ok | 1.876 | 0.751 | 129.2 |
| tests/reference/ome/OME-TIFF/2009-09/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2009-09/single-image.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/single-image.ome.tiff | 2 KiB | bioformats | ok | 1.320 | 0.664 | 110.2 |
| tests/reference/ome/OME-TIFF/2009-09/single-image.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.9 |
| tests/reference/ome/OME-TIFF/2009-09/time-series.ome.tiff | 17 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/time-series.ome.tiff | 17 KiB | bioformats | ok | 1.395 | 0.682 | 114.2 |
| tests/reference/ome/OME-TIFF/2009-09/time-series.ome.tiff | 17 KiB | bioformats-rs | ok | 0.005 | 0.006 | 12.8 |
| tests/reference/ome/OME-TIFF/2009-09/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/OME-TIFF/2009-09/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | bioformats | ok | 2.737 | 0.904 | 165.4 |
| tests/reference/ome/OME-TIFF/2009-09/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | bioformats-rs | ok | 0.007 | 0.008 | 13.2 |
| tests/reference/ome/OME-TIFF/2009-09/z-series-time-series.ome.tiff | 187 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2009-09/z-series-time-series.ome.tiff | 187 KiB | bioformats | ok | 1.694 | 0.748 | 136.3 |
| tests/reference/ome/OME-TIFF/2009-09/z-series-time-series.ome.tiff | 187 KiB | bioformats-rs | ok | 0.007 | 0.008 | 13.1 |
| tests/reference/ome/OME-TIFF/2009-09/z-series.ome.tiff | 17 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2009-09/z-series.ome.tiff | 17 KiB | bioformats | ok | 1.446 | 0.693 | 115.4 |
| tests/reference/ome/OME-TIFF/2009-09/z-series.ome.tiff | 17 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/reference/ome/OME-TIFF/2010-04/instrument.ome.tiff | 4 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/instrument.ome.tiff | 4 KiB | bioformats | ok | 1.399 | 0.673 | 114.8 |
| tests/reference/ome/OME-TIFF/2010-04/instrument.ome.tiff | 4 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-time-series.ome.tiff | 42 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-time-series.ome.tiff | 42 KiB | bioformats | ok | 1.426 | 0.684 | 116.6 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-time-series.ome.tiff | 42 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series-time-series.ome.tiff | 614 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series-time-series.ome.tiff | 614 KiB | bioformats | ok | 2.295 | 0.850 | 193.1 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series-time-series.ome.tiff | 614 KiB | bioformats-rs | ok | 0.014 | 0.015 | 13.8 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series.ome.tiff | 42 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series.ome.tiff | 42 KiB | bioformats | ok | 1.503 | 0.738 | 116.8 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel-z-series.ome.tiff | 42 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel.ome.tiff | 8 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel.ome.tiff | 8 KiB | bioformats | ok | 1.295 | 0.666 | 110.9 |
| tests/reference/ome/OME-TIFF/2010-04/multi-channel.ome.tiff | 8 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/OME-TIFF/2010-04/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats | ok | 1.905 | 0.815 | 123.5 |
| tests/reference/ome/OME-TIFF/2010-04/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2010-04/single-image.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/single-image.ome.tiff | 2 KiB | bioformats | ok | 1.363 | 0.706 | 110.5 |
| tests/reference/ome/OME-TIFF/2010-04/single-image.ome.tiff | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-04/time-series.ome.tiff | 16 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2010-04/time-series.ome.tiff | 16 KiB | bioformats | ok | 1.421 | 0.736 | 112.5 |
| tests/reference/ome/OME-TIFF/2010-04/time-series.ome.tiff | 16 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-04/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/OME-TIFF/2010-04/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | bioformats | ok | 2.537 | 0.866 | 148.9 |
| tests/reference/ome/OME-TIFF/2010-04/two-screens-two-plates-four-wells.ome.tiff | 19 KiB | bioformats-rs | ok | 0.007 | 0.008 | 13.2 |
| tests/reference/ome/OME-TIFF/2010-04/z-series-time-series.ome.tiff | 179 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/z-series-time-series.ome.tiff | 179 KiB | bioformats | ok | 1.731 | 0.755 | 137.9 |
| tests/reference/ome/OME-TIFF/2010-04/z-series-time-series.ome.tiff | 179 KiB | bioformats-rs | ok | 0.006 | 0.007 | 13.3 |
| tests/reference/ome/OME-TIFF/2010-04/z-series.ome.tiff | 16 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-04/z-series.ome.tiff | 16 KiB | bioformats | ok | 1.377 | 0.691 | 112.2 |
| tests/reference/ome/OME-TIFF/2010-04/z-series.ome.tiff | 16 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-06/instrument.ome.tiff | 12 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/instrument.ome.tiff | 12 KiB | bioformats | ok | 1.443 | 0.728 | 112.8 |
| tests/reference/ome/OME-TIFF/2010-06/instrument.ome.tiff | 12 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-time-series.ome.tiff | 12 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-time-series.ome.tiff | 12 KiB | bioformats | ok | 1.488 | 0.729 | 115.4 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-time-series.ome.tiff | 12 KiB | bioformats-rs | ok | 0.005 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series-time-series.ome.tiff | 51 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series-time-series.ome.tiff | 51 KiB | bioformats | ok | 2.196 | 0.863 | 161.4 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series-time-series.ome.tiff | 51 KiB | bioformats-rs | ok | 0.009 | 0.010 | 13.3 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series.ome.tiff | 12 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series.ome.tiff | 12 KiB | bioformats | ok | 1.521 | 0.748 | 115.1 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel-z-series.ome.tiff | 12 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel.ome.tiff | 3 KiB | bioformats | ok | 1.311 | 0.682 | 112.2 |
| tests/reference/ome/OME-TIFF/2010-06/multi-channel.ome.tiff | 3 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.8 |
| tests/reference/ome/OME-TIFF/2010-06/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats | ok | 1.720 | 0.714 | 120.0 |
| tests/reference/ome/OME-TIFF/2010-06/one-screen-one-plate-four-wells.ome.tiff | 10 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.7 |
| tests/reference/ome/OME-TIFF/2010-06/single-image.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.9 |
| tests/reference/ome/OME-TIFF/2010-06/single-image.ome.tiff | 2 KiB | bioformats | ok | 1.301 | 0.705 | 110.1 |
| tests/reference/ome/OME-TIFF/2010-06/single-image.ome.tiff | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.8 |
| tests/reference/ome/OME-TIFF/2010-06/time-series.ome.tiff | 6 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/time-series.ome.tiff | 6 KiB | bioformats | ok | 1.350 | 0.692 | 112.8 |
| tests/reference/ome/OME-TIFF/2010-06/time-series.ome.tiff | 6 KiB | bioformats-rs | ok | 0.004 | 0.004 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-06/two-screens-two-plates-four-wells.ome.tiff | 18 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/OME-TIFF/2010-06/two-screens-two-plates-four-wells.ome.tiff | 18 KiB | bioformats | ok | 2.242 | 0.797 | 138.0 |
| tests/reference/ome/OME-TIFF/2010-06/two-screens-two-plates-four-wells.ome.tiff | 18 KiB | bioformats-rs | ok | 0.007 | 0.008 | 12.9 |
| tests/reference/ome/OME-TIFF/2010-06/z-series-time-series.ome.tiff | 26 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2010-06/z-series-time-series.ome.tiff | 26 KiB | bioformats | ok | 1.632 | 0.732 | 134.8 |
| tests/reference/ome/OME-TIFF/2010-06/z-series-time-series.ome.tiff | 26 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2010-06/z-series.ome.tiff | 6 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2010-06/z-series.ome.tiff | 6 KiB | bioformats | ok | 1.375 | 0.680 | 112.5 |
| tests/reference/ome/OME-TIFF/2010-06/z-series.ome.tiff | 6 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.6 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z2.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Missing file multifile- | 1.184 | 0.627 | 108.1 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z3.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Missing file multifile- | 1.197 | 0.622 | 108.0 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z4.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Missing file multifile- | 1.147 | 0.625 | 109.1 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z5.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Missing file multifile- | 1.159 | 0.631 | 108.9 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.004 | 0.004 | 11.9 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | scopeio | ok | 0.004 | 0.005 | 3.0 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | bioformats | ok | 2.135 | 0.885 | 263.3 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | bioformats-rs | ok | 0.011 | 0.012 | 16.6 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | scopeio | ok | 0.013 | 0.014 | 3.4 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | bioformats | ok | 4.149 | 1.206 | 383.8 |
| tests/reference/ome/OME-TIFF/2012-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | bioformats-rs | ok | 0.032 | 0.033 | 26.3 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | scopeio | ok | 0.001 | 0.002 | 3.0 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.123 | 0.635 | 111.3 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.004 | 0.005 | 12.1 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | scopeio | ok | 0.004 | 0.005 | 3.0 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.294 | 0.688 | 150.6 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.006 | 0.007 | 12.1 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z2.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.098 | 0.609 | 109.5 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.6 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z3.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.124 | 0.626 | 108.4 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.9 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z4.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.093 | 0.609 | 108.8 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.003 | 11.6 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z5.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.129 | 0.622 | 109.2 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 12.0 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1198 KiB | scopeio | ok | 0.003 | 0.004 | 2.8 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1198 KiB | bioformats | ok | 1.631 | 0.762 | 222.7 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1198 KiB | bioformats-rs | ok | 0.007 | 0.008 | 15.0 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | scopeio | ok | 0.005 | 0.006 | 3.0 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | bioformats | ok | 1.999 | 0.833 | 264.9 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2059 KiB | bioformats-rs | ok | 0.010 | 0.011 | 16.3 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | scopeio | ok | 0.015 | 0.016 | 3.5 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | bioformats | ok | 3.589 | 1.180 | 371.2 |
| tests/reference/ome/OME-TIFF/2013-06/modulo/SPIM-ModuloAlongZ.ome.tiff | 7045 KiB | bioformats-rs | ok | 0.029 | 0.030 | 26.4 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | scopeio | ok | 0.001 | 0.002 | 3.1 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.072 | 0.608 | 110.4 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.004 | 0.004 | 11.9 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | scopeio | ok | 0.005 | 0.006 | 3.2 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.257 | 0.656 | 152.1 |
| tests/reference/ome/OME-TIFF/2013-06/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.007 | 0.007 | 11.9 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/multi-channel.ome.tif | 221 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/multi-channel.ome.tif | 221 KiB | bioformats | ok | 1.122 | 0.621 | 112.2 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/multi-channel.ome.tif | 221 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.2 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/single-channel.ome.tif | 74 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/single-channel.ome.tif | 74 KiB | bioformats | ok | 1.084 | 0.626 | 107.7 |
| tests/reference/ome/OME-TIFF/2015-01/bioformats-artificial/single-channel.ome.tif | 74 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z2.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.080 | 0.622 | 108.2 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z2.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.005 | 12.1 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z3.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.024 | 0.590 | 107.6 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z3.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.9 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z4.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.062 | 0.579 | 107.1 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z4.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.004 | 0.005 | 12.0 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z5.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats | error: loci.formats.FormatException: Invalid file (may be co | 1.045 | 0.605 | 108.0 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile-Z5.ome.tiff | 3 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.004 | 0.005 | 12.1 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1189 KiB | scopeio | ok | 0.003 | 0.004 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1189 KiB | bioformats | ok | 1.450 | 0.752 | 214.1 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/FLIM-ModuloAlongT-TSCPC.ome.tiff | 1189 KiB | bioformats-rs | ok | 0.006 | 0.008 | 14.7 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2058 KiB | scopeio | ok | 0.004 | 0.005 | 2.9 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2058 KiB | bioformats | ok | 1.718 | 0.804 | 255.0 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/LAMBDA-ModuloAlongZ-ModuloAlongT.ome.tiff | 2058 KiB | bioformats-rs | ok | 0.012 | 0.013 | 16.6 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/SPIM-ModuloAlongZ.ome.tiff | 7057 KiB | scopeio | ok | 0.014 | 0.014 | 3.3 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/SPIM-ModuloAlongZ.ome.tiff | 7057 KiB | bioformats | ok | 3.032 | 1.125 | 343.7 |
| tests/reference/ome/OME-TIFF/2015-01/modulo/SPIM-ModuloAlongZ.ome.tiff | 7057 KiB | bioformats-rs | ok | 0.032 | 0.033 | 26.4 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | scopeio | ok | 0.001 | 0.002 | 3.1 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.010 | 0.579 | 109.1 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-2D/tubhiswt_C1.ome.tif | 263 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.003 | 0.004 | 11.9 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | scopeio | ok | 0.005 | 0.006 | 3.1 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 1.120 | 0.642 | 150.9 |
| tests/reference/ome/OME-TIFF/2015-01/tubhiswt-3D/tubhiswt_C1.ome.tif | 5214 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.006 | 0.007 | 12.2 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z1.ome.tiff | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z1.ome.tiff | 3 KiB | bioformats | ok | 0.927 | 0.526 | 107.4 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z1.ome.tiff | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.7 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z2.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z2.ome.tiff | 2 KiB | bioformats | ok | 0.984 | 0.544 | 108.0 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z2.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z3.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z3.ome.tiff | 2 KiB | bioformats | ok | 0.983 | 0.539 | 107.2 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z3.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z4.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z4.ome.tiff | 2 KiB | bioformats | ok | 1.057 | 0.585 | 107.9 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z4.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z5.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z5.ome.tiff | 2 KiB | bioformats | ok | 1.047 | 0.585 | 106.8 |
| tests/reference/ome/OME-TIFF/2016-06/binaryonly/multifile-Z5.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z1.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z1.ome.tiff | 2 KiB | bioformats | ok | 1.053 | 0.582 | 107.4 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z1.ome.tiff | 2 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z2.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z2.ome.tiff | 2 KiB | bioformats | ok | 1.098 | 0.621 | 109.1 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z2.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z3.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z3.ome.tiff | 2 KiB | bioformats | ok | 1.106 | 0.612 | 107.8 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z3.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z4.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.6 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z4.ome.tiff | 2 KiB | bioformats | ok | 1.091 | 0.610 | 107.6 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z4.ome.tiff | 2 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z5.ome.tiff | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z5.ome.tiff | 2 KiB | bioformats | ok | 1.138 | 0.618 | 107.2 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile-Z5.ome.tiff | 2 KiB | bioformats-rs | ok | 0.005 | 0.006 | 13.0 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-A2.ome.tiff | 11 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-A2.ome.tiff | 11 KiB | bioformats | ok | 1.189 | 0.642 | 105.2 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-A2.ome.tiff | 11 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.6 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B1.ome.tiff | 11 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B1.ome.tiff | 11 KiB | bioformats | ok | 1.054 | 0.563 | 106.0 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B1.ome.tiff | 11 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B3.ome.tiff | 11 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B3.ome.tiff | 11 KiB | bioformats | ok | 1.013 | 0.551 | 106.6 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-B3.ome.tiff | 11 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.8 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2-2.ome.tiff | 11 KiB | scopeio | ok | 0.001 | 0.002 | 2.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2-2.ome.tiff | 11 KiB | bioformats | ok | 1.026 | 0.533 | 105.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2-2.ome.tiff | 11 KiB | bioformats-rs | ok | 0.004 | 0.005 | 12.9 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2.ome.tiff | 11 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2.ome.tiff | 11 KiB | bioformats | ok | 1.051 | 0.569 | 105.7 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/well-C2.ome.tiff | 11 KiB | bioformats-rs | ok | 0.004 | 0.005 | 13.0 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-4D/tubhiswt_C1_TP42.ome.tif | 2749 KiB | scopeio | ok | 0.130 | 0.132 | 10.2 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-4D/tubhiswt_C1_TP42.ome.tif | 2749 KiB | bioformats | error: loci.formats.FormatException: Missing file tubhiswt_C | 3.686 | 0.937 | 264.6 |
| tests/reference/ome/OME-TIFF/2012-06/tubhiswt-4D/tubhiswt_C1_TP42.ome.tif | 2749 KiB | bioformats-rs | error: Error reading plane 0: Plane index 0 out of range | 0.044 | 0.045 | 13.0 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile.companion.ome | 2 KiB | scopeio | ok | 0.001 | 0.002 | 2.7 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile.companion.ome | 2 KiB | bioformats | error: java.io.FileNotFoundException: /workspaces/scopeio/te | 1.004 | 0.568 | 106.6 |
| tests/reference/ome/OME-TIFF/2012-06/companion/multifile.companion.ome | 2 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML companion | 0.002 | 0.003 | 14.7 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile.companion.ome | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile.companion.ome | 2 KiB | bioformats | error: java.io.FileNotFoundException: /workspaces/scopeio/te | 0.985 | 0.567 | 106.6 |
| tests/reference/ome/OME-TIFF/2013-06/companion/multifile.companion.ome | 2 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML companion | 0.002 | 0.003 | 14.7 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile.companion.ome | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile.companion.ome | 2 KiB | bioformats | error: java.io.FileNotFoundException: /workspaces/scopeio/te | 0.944 | 0.563 | 106.3 |
| tests/reference/ome/OME-TIFF/2015-01/companion/multifile.companion.ome | 2 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML companion | 0.002 | 0.003 | 14.7 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile.companion.ome | 2 KiB | scopeio | ok | 0.001 | 0.001 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile.companion.ome | 2 KiB | bioformats | ok | 0.968 | 0.544 | 107.5 |
| tests/reference/ome/OME-TIFF/2016-06/companion/multifile.companion.ome | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 14.1 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/hcs.companion.ome | 3 KiB | scopeio | ok | 0.001 | 0.002 | 2.8 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/hcs.companion.ome | 3 KiB | bioformats | ok | 0.971 | 0.532 | 105.4 |
| tests/reference/ome/OME-TIFF/2016-06/plate-companion/hcs.companion.ome | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 14.3 |

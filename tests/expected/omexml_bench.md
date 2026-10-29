# omexml: CPU and RAM compared with the reference readers

Generated 2026-10-29 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-38-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 1 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/ome/OME-XML/2016-06/ROI.ome.xml | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/ROI.ome.xml | 3 KiB | bioformats | ok | 0.873 | 0.476 | 116.9 |
| tests/reference/ome/OME-XML/2016-06/ROI.ome.xml | 3 KiB | bioformats-rs | ok | 0.003 | 0.005 | 11.1 |
| tests/reference/ome/OME-XML/2016-06/commentannotation.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/commentannotation.ome.xml | 1 KiB | bioformats | ok | 0.862 | 0.479 | 110.4 |
| tests/reference/ome/OME-XML/2016-06/commentannotation.ome.xml | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/reference/ome/OME-XML/2016-06/filter.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/filter.ome.xml | 4 KiB | bioformats | ok | 0.820 | 0.450 | 104.4 |
| tests/reference/ome/OME-XML/2016-06/filter.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2016-06/folders-larger-taxonomy.ome.xml | 261 KiB | scopeio | ok | 0.002 | 0.003 | 5.2 |
| tests/reference/ome/OME-XML/2016-06/folders-larger-taxonomy.ome.xml | 261 KiB | bioformats | ok | 1.127 | 0.577 | 206.6 |
| tests/reference/ome/OME-XML/2016-06/folders-larger-taxonomy.ome.xml | 261 KiB | bioformats-rs | ok | 0.005 | 0.006 | 11.7 |
| tests/reference/ome/OME-XML/2016-06/folders-simple-taxonomy.ome.xml | 258 KiB | scopeio | ok | 0.003 | 0.004 | 5.3 |
| tests/reference/ome/OME-XML/2016-06/folders-simple-taxonomy.ome.xml | 258 KiB | bioformats | ok | 1.047 | 0.542 | 153.4 |
| tests/reference/ome/OME-XML/2016-06/folders-simple-taxonomy.ome.xml | 258 KiB | bioformats-rs | ok | 0.005 | 0.006 | 11.5 |
| tests/reference/ome/OME-XML/2016-06/hcs.ome.xml | 2 KiB | scopeio | ok | 0.005 | 0.006 | 6.5 |
| tests/reference/ome/OME-XML/2016-06/hcs.ome.xml | 2 KiB | bioformats | ok | 1.083 | 0.595 | 275.7 |
| tests/reference/ome/OME-XML/2016-06/hcs.ome.xml | 2 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML BinData p | 0.003 | 0.004 | 11.8 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-alternate.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-alternate.ome.xml | 4 KiB | bioformats | ok | 0.890 | 0.472 | 139.6 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-alternate.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-default.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-default.ome.xml | 4 KiB | bioformats | ok | 0.903 | 0.488 | 140.2 |
| tests/reference/ome/OME-XML/2016-06/instrument-units-default.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2016-06/instrument.ome.xml | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/instrument.ome.xml | 3 KiB | bioformats | ok | 0.906 | 0.503 | 136.5 |
| tests/reference/ome/OME-XML/2016-06/instrument.ome.xml | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2016-06/mapannotation.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/mapannotation.ome.xml | 1 KiB | bioformats | ok | 0.839 | 0.461 | 108.1 |
| tests/reference/ome/OME-XML/2016-06/mapannotation.ome.xml | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.7 |
| tests/reference/ome/OME-XML/2016-06/metadata-only.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/metadata-only.ome.xml | 1 KiB | bioformats | ok | 0.878 | 0.470 | 117.2 |
| tests/reference/ome/OME-XML/2016-06/metadata-only.ome.xml | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/minimum-specification.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/minimum-specification.ome.xml | 1 KiB | bioformats | ok | 0.824 | 0.466 | 120.9 |
| tests/reference/ome/OME-XML/2016-06/minimum-specification.ome.xml | 1 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML BinData p | 0.003 | 0.004 | 11.9 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-time-series.ome.xml | 7 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-time-series.ome.xml | 7 KiB | bioformats | ok | 0.901 | 0.475 | 129.0 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-time-series.ome.xml | 7 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.9 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series-time-series.ome.xml | 33 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series-time-series.ome.xml | 33 KiB | bioformats | ok | 1.116 | 0.611 | 211.1 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series-time-series.ome.xml | 33 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.3 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series.ome.xml | 7 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series.ome.xml | 7 KiB | bioformats | ok | 0.981 | 0.516 | 131.2 |
| tests/reference/ome/OME-XML/2016-06/multi-channel-z-series.ome.xml | 7 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/multi-channel.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/multi-channel.ome.xml | 1 KiB | bioformats | ok | 0.942 | 0.517 | 116.3 |
| tests/reference/ome/OME-XML/2016-06/multi-channel.ome.xml | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/no-date.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/no-date.ome.xml | 1 KiB | bioformats | ok | 0.829 | 0.464 | 108.6 |
| tests/reference/ome/OME-XML/2016-06/no-date.ome.xml | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2016-06/one-screen-one-plate-four-wells.ome.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/reference/ome/OME-XML/2016-06/one-screen-one-plate-four-wells.ome.xml | 8 KiB | bioformats | ok | 1.116 | 0.596 | 197.6 |
| tests/reference/ome/OME-XML/2016-06/one-screen-one-plate-four-wells.ome.xml | 8 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2016-06/single-image.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/single-image.ome.xml | 1 KiB | bioformats | ok | 0.815 | 0.459 | 109.9 |
| tests/reference/ome/OME-XML/2016-06/single-image.ome.xml | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2016-06/spim.ome.xml | 23 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/reference/ome/OME-XML/2016-06/spim.ome.xml | 23 KiB | bioformats | ok | 1.360 | 0.646 | 222.1 |
| tests/reference/ome/OME-XML/2016-06/spim.ome.xml | 23 KiB | bioformats-rs | ok | 0.002 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2016-06/tagannotation.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/tagannotation.ome.xml | 2 KiB | bioformats | ok | 0.855 | 0.492 | 110.0 |
| tests/reference/ome/OME-XML/2016-06/tagannotation.ome.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/time-series.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2016-06/time-series.ome.xml | 4 KiB | bioformats | ok | 0.847 | 0.458 | 118.9 |
| tests/reference/ome/OME-XML/2016-06/time-series.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation-posix-only.ome.xml | 9 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation-posix-only.ome.xml | 9 KiB | bioformats | ok | 0.998 | 0.567 | 170.3 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation-posix-only.ome.xml | 9 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation.ome.xml | 10 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation.ome.xml | 10 KiB | bioformats | ok | 1.042 | 0.549 | 172.3 |
| tests/reference/ome/OME-XML/2016-06/timestampannotation.ome.xml | 10 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2016-06/transformations-downgrade.ome.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/reference/ome/OME-XML/2016-06/transformations-downgrade.ome.xml | 8 KiB | bioformats | ok | 0.984 | 0.514 | 149.2 |
| tests/reference/ome/OME-XML/2016-06/transformations-downgrade.ome.xml | 8 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.6 |
| tests/reference/ome/OME-XML/2016-06/transformations-upgrade.ome.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/transformations-upgrade.ome.xml | 8 KiB | bioformats | ok | 1.000 | 0.522 | 171.8 |
| tests/reference/ome/OME-XML/2016-06/transformations-upgrade.ome.xml | 8 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/reference/ome/OME-XML/2016-06/two-screens-two-plates-four-wells.ome.xml | 15 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/two-screens-two-plates-four-wells.ome.xml | 15 KiB | bioformats | ok | 1.174 | 0.575 | 221.2 |
| tests/reference/ome/OME-XML/2016-06/two-screens-two-plates-four-wells.ome.xml | 15 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-body-space.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-body-space.ome.xml | 1 KiB | bioformats | ok | 0.924 | 0.496 | 109.1 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-body-space.ome.xml | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.9 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-multi-value.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-multi-value.ome.xml | 1 KiB | bioformats | ok | 0.928 | 0.511 | 109.6 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-multi-value.ome.xml | 1 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.7 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-svg.ome.xml | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.8 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-svg.ome.xml | 3 KiB | bioformats | ok | 0.917 | 0.482 | 109.5 |
| tests/reference/ome/OME-XML/2016-06/xmlannotation-svg.ome.xml | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2016-06/z-series-time-series.ome.xml | 17 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2016-06/z-series-time-series.ome.xml | 17 KiB | bioformats | ok | 1.061 | 0.575 | 160.7 |
| tests/reference/ome/OME-XML/2016-06/z-series-time-series.ome.xml | 17 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2016-06/z-series.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2016-06/z-series.ome.xml | 4 KiB | bioformats | ok | 0.913 | 0.498 | 117.4 |
| tests/reference/ome/OME-XML/2016-06/z-series.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2008-09/dataset-group-project.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/OME-XML/2008-09/dataset-group-project.ome.xml | 2 KiB | bioformats | ok | 1.371 | 0.658 | 135.2 |
| tests/reference/ome/OME-XML/2008-09/dataset-group-project.ome.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2008-09/instrument.ome.xml | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2008-09/instrument.ome.xml | 3 KiB | bioformats | ok | 1.464 | 0.670 | 140.9 |
| tests/reference/ome/OME-XML/2008-09/instrument.ome.xml | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-time-series.ome.xml | 7 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-time-series.ome.xml | 7 KiB | bioformats | ok | 1.438 | 0.663 | 138.5 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-time-series.ome.xml | 7 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.8 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series-time-series.ome.xml | 32 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series-time-series.ome.xml | 32 KiB | bioformats | ok | 1.814 | 0.803 | 210.6 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series-time-series.ome.xml | 32 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.7 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series.ome.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series.ome.xml | 8 KiB | bioformats | ok | 1.453 | 0.677 | 137.2 |
| tests/reference/ome/OME-XML/2008-09/multi-channel-z-series.ome.xml | 8 KiB | bioformats-rs | ok | 0.002 | 0.003 | 10.6 |
| tests/reference/ome/OME-XML/2008-09/multi-channel.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2008-09/multi-channel.ome.xml | 2 KiB | bioformats | ok | 1.406 | 0.646 | 127.2 |
| tests/reference/ome/OME-XML/2008-09/multi-channel.ome.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.8 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-aquired.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-aquired.ome.xml | 2 KiB | bioformats | ok | 1.427 | 0.674 | 124.2 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-aquired.ome.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.9 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-default.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-default.ome.xml | 2 KiB | bioformats | ok | 1.374 | 0.668 | 122.9 |
| tests/reference/ome/OME-XML/2008-09/multi-pixel-default.ome.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2008-09/single-image.ome.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2008-09/single-image.ome.xml | 1 KiB | bioformats | ok | 1.475 | 0.726 | 120.2 |
| tests/reference/ome/OME-XML/2008-09/single-image.ome.xml | 1 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.2 |
| tests/reference/ome/OME-XML/2008-09/time-series.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2008-09/time-series.ome.xml | 4 KiB | bioformats | ok | 1.430 | 0.696 | 126.7 |
| tests/reference/ome/OME-XML/2008-09/time-series.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.1 |
| tests/reference/ome/OME-XML/2008-09/z-series-time-series.ome.xml | 16 KiB | scopeio | ok | 0.001 | 0.002 | 4.4 |
| tests/reference/ome/OME-XML/2008-09/z-series-time-series.ome.xml | 16 KiB | bioformats | ok | 1.645 | 0.719 | 166.7 |
| tests/reference/ome/OME-XML/2008-09/z-series-time-series.ome.xml | 16 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/reference/ome/OME-XML/2008-09/z-series.ome.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 4.5 |
| tests/reference/ome/OME-XML/2008-09/z-series.ome.xml | 4 KiB | bioformats | ok | 1.492 | 0.738 | 126.7 |
| tests/reference/ome/OME-XML/2008-09/z-series.ome.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 11.0 |
| tests/data/omexml/bit.ome.xml | 0 KiB | scopeio | ok | 0.001 | 0.002 | 4.6 |
| tests/data/omexml/bit.ome.xml | 0 KiB | bioformats | error: java.lang.ArrayIndexOutOfBoundsException: arraycopy:  | 0.908 | 0.505 | 106.6 |
| tests/data/omexml/bit.ome.xml | 0 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML unsupport | 0.003 | 0.004 | 12.3 |
| tests/data/omexml/zlib_be.ome.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 4.7 |
| tests/data/omexml/zlib_be.ome.xml | 2 KiB | bioformats | error: java.lang.IllegalArgumentException: Invalid plane ind | 0.959 | 0.517 | 118.6 |
| tests/data/omexml/zlib_be.ome.xml | 2 KiB | bioformats-rs | error: Error opening <file>: Format error: OME-XML BinData p | 0.003 | 0.005 | 12.4 |

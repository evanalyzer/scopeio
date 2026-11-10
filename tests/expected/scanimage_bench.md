# scanimage: CPU and RAM compared with the reference readers

Generated 2026-11-10 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-42-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/channel_two_only_loop_version_2017b/planar_00001.tif | 117 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/channel_two_only_loop_version_2017b/planar_00001.tif | 117 KiB | bioformats | ok | 1.837 | 0.692 | 148.0 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/channel_two_only_loop_version_2017b/planar_00001.tif | 117 KiB | bioformats-rs | ok | 0.007 | 0.008 | 10.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/small_frames_at_high_rate_resonant_version_2018b/planar_00001.tif | 397 KiB | scopeio | ok | 0.001 | 0.002 | 5.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/small_frames_at_high_rate_resonant_version_2018b/planar_00001.tif | 397 KiB | bioformats | ok | 2.177 | 0.870 | 220.6 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/small_frames_at_high_rate_resonant_version_2018b/planar_00001.tif | 397 KiB | bioformats-rs | ok | 0.014 | 0.015 | 12.0 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/planar/single_channel_resonant_version_2020_1/planar.tif | 49 KiB | scopeio | ok | 0.001 | 0.002 | 5.5 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/planar/single_channel_resonant_version_2020_1/planar.tif | 49 KiB | bioformats | ok | 1.695 | 0.683 | 135.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/planar/single_channel_resonant_version_2020_1/planar.tif | 49 KiB | bioformats-rs | ok | 0.005 | 0.006 | 10.1 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/planar/single_channel_int16_version_4_2/planar_001_005.tif | 145 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/planar/single_channel_int16_version_4_2/planar_001_005.tif | 145 KiB | bioformats | ok | 2.120 | 0.784 | 152.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/planar/single_channel_int16_version_4_2/planar_001_005.tif | 145 KiB | bioformats-rs | ok | 0.005 | 0.006 | 10.2 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_1/planar/single_channel_resonant_version_5_1/planar.tif | 183 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_1/planar/single_channel_resonant_version_5_1/planar.tif | 183 KiB | bioformats | ok | 2.564 | 0.845 | 166.8 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_1/planar/single_channel_resonant_version_5_1/planar.tif | 183 KiB | bioformats-rs | ok | 0.006 | 0.007 | 10.5 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/two_channels_resonant_version_2016b/planar.tif | 217 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/two_channels_resonant_version_2016b/planar.tif | 217 KiB | bioformats | ok | 1.924 | 0.760 | 153.7 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_3/planar/two_channels_resonant_version_2016b/planar.tif | 217 KiB | bioformats-rs | ok | 0.006 | 0.007 | 10.6 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/volumetric/flyback_frames_two_channels_version_2021_0/volume_00001_00001.tif | 320 KiB | scopeio | ok | 0.001 | 0.002 | 5.4 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/volumetric/flyback_frames_two_channels_version_2021_0/volume_00001_00001.tif | 320 KiB | bioformats | ok | 2.184 | 0.850 | 198.5 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/volumetric/flyback_frames_two_channels_version_2021_0/volume_00001_00001.tif | 320 KiB | bioformats-rs | ok | 0.020 | 0.021 | 12.6 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multiple_frames_per_slice/frames_saved_channels_one_and_four_version_2023_0/stack.tif | 267 KiB | scopeio | ok | 0.001 | 0.002 | 5.4 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multiple_frames_per_slice/frames_saved_channels_one_and_four_version_2023_0/stack.tif | 267 KiB | bioformats | ok | 1.863 | 0.765 | 158.9 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multiple_frames_per_slice/frames_saved_channels_one_and_four_version_2023_0/stack.tif | 267 KiB | bioformats-rs | ok | 0.008 | 0.009 | 11.0 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multi_roi/planar_eight_rois_version_2023_1/planar_00000_00001.tif | 238 KiB | scopeio | ok | 0.001 | 0.002 | 5.4 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multi_roi/planar_eight_rois_version_2023_1/planar_00000_00001.tif | 238 KiB | bioformats | ok | 1.796 | 0.732 | 141.3 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/tiff_format_version_4/multi_roi/planar_eight_rois_version_2023_1/planar_00000_00001.tif | 238 KiB | bioformats-rs | ok | 0.007 | 0.008 | 10.4 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/multiple_frames_per_slice/frames_averaged_two_channels_version_3_6/stack_001.tif | 825 KiB | scopeio | ok | 0.001 | 0.003 | 5.2 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/multiple_frames_per_slice/frames_averaged_two_channels_version_3_6/stack_001.tif | 825 KiB | bioformats | ok | 2.293 | 0.906 | 261.6 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/legacy/multiple_frames_per_slice/frames_averaged_two_channels_version_3_6/stack_001.tif | 825 KiB | bioformats-rs | ok | 0.007 | 0.008 | 11.5 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/volumetric_two_channels_single_file/vol_two_ch_single_file_00001_00001.tif | 9971 KiB | scopeio | ok | 0.007 | 0.008 | 6.6 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/volumetric_two_channels_single_file/vol_two_ch_single_file_00001_00001.tif | 9971 KiB | bioformats | ok | 19.246 | 16.589 | 458.2 |
| tests/reference/gin-ophys/imaging_datasets/ScanImage/volumetric_two_channels_single_file/vol_two_ch_single_file_00001_00001.tif | 9971 KiB | bioformats-rs | ok | 0.443 | 0.444 | 97.3 |
| tests/data/scanimage/f1_slow.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 4.9 |
| tests/data/scanimage/f1_slow.tif | 3 KiB | bioformats | ok | 1.493 | 0.659 | 123.3 |
| tests/data/scanimage/f1_slow.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.3 |
| tests/data/scanimage/f2_json.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 5.2 |
| tests/data/scanimage/f2_json.tif | 2 KiB | bioformats | ok | 1.437 | 0.644 | 118.0 |
| tests/data/scanimage/f2_json.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.1 |
| tests/data/scanimage/si4_planar.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/data/scanimage/si4_planar.tif | 3 KiB | bioformats | ok | 1.492 | 0.641 | 122.3 |
| tests/data/scanimage/si4_planar.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.3 |
| tests/data/scanimage/state_stack.tif | 2 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/data/scanimage/state_stack.tif | 2 KiB | bioformats | ok | 1.478 | 0.673 | 121.3 |
| tests/data/scanimage/state_stack.tif | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.2 |
| tests/data/scanimage/v3_mroi.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 5.2 |
| tests/data/scanimage/v3_mroi.tif | 3 KiB | bioformats | ok | 1.457 | 0.645 | 118.4 |
| tests/data/scanimage/v3_mroi.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.3 |
| tests/data/scanimage/v4_fast_flyback.tif | 9 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/data/scanimage/v4_fast_flyback.tif | 9 KiB | bioformats | ok | 1.518 | 0.663 | 128.2 |
| tests/data/scanimage/v4_fast_flyback.tif | 9 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.2 |
| tests/data/scanimage/v4_stopped.tif | 3 KiB | scopeio | ok | 0.001 | 0.002 | 5.1 |
| tests/data/scanimage/v4_stopped.tif | 3 KiB | bioformats | ok | 1.467 | 0.662 | 119.0 |
| tests/data/scanimage/v4_stopped.tif | 3 KiB | bioformats-rs | ok | 0.003 | 0.004 | 10.0 |

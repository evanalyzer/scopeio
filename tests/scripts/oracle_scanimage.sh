#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Regenerates the ScanImage references: CatalystNeuro test files (GIN
# ophys_testing_data, fetched with tests/scripts/fetch_gin.py) and the
# synthetic files. Bio-Formats and bioformats-rs read ScanImage files as
# plain TIFF (every page a time point).
#
# Usage: REP=3 sh tests/scripts/oracle_scanimage.sh
set -e
cd "$(dirname "$0")/../.."
S=tests/reference/gin-ophys/imaging_datasets/ScanImage
REP=${REP:-1}
python3 -I tests/scripts/oracle.py scanimage --repeat $REP \
    $S/tiff_format_version_3/planar/channel_two_only_loop_version_2017b/planar_00001.tif \
    $S/tiff_format_version_3/planar/small_frames_at_high_rate_resonant_version_2018b/planar_00001.tif \
    $S/tiff_format_version_4/planar/single_channel_resonant_version_2020_1/planar.tif \
    $S/legacy/planar/single_channel_int16_version_4_2/planar_001_005.tif \
    $S/tiff_format_version_1/planar/single_channel_resonant_version_5_1/planar.tif \
    $S/tiff_format_version_3/planar/two_channels_resonant_version_2016b/planar.tif \
    $S/tiff_format_version_4/volumetric/flyback_frames_two_channels_version_2021_0/volume_00001_00001.tif \
    $S/tiff_format_version_4/multiple_frames_per_slice/frames_saved_channels_one_and_four_version_2023_0/stack.tif \
    $S/tiff_format_version_4/multi_roi/planar_eight_rois_version_2023_1/planar_00000_00001.tif \
    $S/legacy/multiple_frames_per_slice/frames_averaged_two_channels_version_3_6/stack_001.tif \
    $S/volumetric_two_channels_single_file/vol_two_ch_single_file_00001_00001.tif \
    tests/data/scanimage/*.tif

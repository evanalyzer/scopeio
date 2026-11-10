#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Regenerates the Philips TIFF references (OpenSlide test data, CC0-1.0).
# Bio-Formats reads Philips TIFF as plain TIFF and returns only the
# full-resolution image, so the references are full-resolution crops; the
# second Philips-4 crop covers tiles that are not stored.
#
# Usage: REP=3 sh tests/scripts/oracle_philips.sh
set -e
cd "$(dirname "$0")/../.."
P=tests/reference/openslide/Philips-TIFF
REP=${REP:-1}
python3 -I tests/scripts/oracle.py philips --repeat $REP --series 0 --ours-series 0 --crop 20000,15000,2048,2048 $P/Philips-1.tiff
python3 -I tests/scripts/oracle.py philips --repeat 1 --series 0 --ours-series 0 --crop 52736,146944,2048,2048 --append $P/Philips-2.tiff
python3 -I tests/scripts/oracle.py philips --repeat $REP --series 0 --ours-series 0 --crop 76800,41472,2048,2048 --append $P/Philips-4.tiff
python3 -I tests/scripts/oracle.py philips --repeat 1 --series 0 --ours-series 0 --crop 15360,15360,2048,2048 --append $P/Philips-4.tiff

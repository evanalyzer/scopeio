#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Regenerates the Leica SCN references (OpenSlide test data, "distributable").
# Bio-Formats lists the macro images as series before the main images.
#
# Usage: REP=3 sh tests/scripts/oracle_scn.sh
set -e
cd "$(dirname "$0")/../.."
L=tests/reference/openslide/Leica
REP=${REP:-1}
python3 -I tests/scripts/oracle.py scn --repeat $REP --series 3 --ours-series 0 --crop 15000,15000,2048,2048 $L/Leica-1.scn
python3 -I tests/scripts/oracle.py scn --repeat 1 --series 5 --ours-series 0/2 --append $L/Leica-1.scn
python3 -I tests/scripts/oracle.py scn --repeat 1 --series 0 --ours-series assoc:macro --append $L/Leica-1.scn
python3 -I tests/scripts/oracle.py scn --repeat $REP --series 6 --ours-series 0 --append $L/Leica-Fluorescence-1.scn
python3 -I tests/scripts/oracle.py scn --repeat 1 --series 7 --ours-series 0/1 --append $L/Leica-Fluorescence-1.scn
python3 -I tests/scripts/oracle.py scn --repeat 1 --series 0 --ours-series assoc:macro --append $L/Leica-Fluorescence-1.scn
python3 -I tests/scripts/oracle.py scn --repeat 1 --series 3 --ours-series "assoc:macro 2" --append $L/Leica-Fluorescence-1.scn

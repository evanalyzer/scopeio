#!/bin/sh
# Regenerates the Ventana BIF references (OpenSlide test data, CC0).
#
# Usage: REP=3 sh tests/scripts/oracle_bif.sh
set -e
cd "$(dirname "$0")/../.."
V=tests/reference/openslide/Ventana/Ventana-1.bif
REP=${REP:-1}
python3 -I tests/scripts/oracle.py bif --repeat $REP --series 0 --crop 4000,8000,4096,2048 $V
python3 -I tests/scripts/oracle.py bif --repeat 1 --series 0 --crop 0,0,2048,1024 --append $V
python3 -I tests/scripts/oracle.py bif --repeat 1 --series 3 --ours-series 0/3 --append $V
python3 -I tests/scripts/oracle.py bif --repeat 1 --series 8 --ours-series assoc:macro --append $V
python3 -I tests/scripts/oracle.py bif --repeat 1 --series 9 --ours-series assoc:probability --append $V

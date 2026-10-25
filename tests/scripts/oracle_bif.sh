#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Regenerates the Ventana BIF references (OpenSlide test data, CC0 and
# distributable); OS-2 (iScan) needs fetch_openslide.py --large and is
# compared with OpenSlide crops.
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
O=tests/reference/openslide/Ventana/OS-2.bif
if [ -f $O ]; then
  for c in "0 69000,37000,2048,2048" "0 2000,34000,1024,1024" "2 14000,9000,1024,1024" "5 1000,600,512,512" "8 0,0,449,299"; do
    set -- $c
    python3 -I tests/scripts/oracle.py bif --repeat 1 --tools openslide --ours-series 0/$1 --crop $2 --append $O
  done
fi

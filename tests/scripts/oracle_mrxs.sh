#!/bin/sh
# Regenerates the MIRAX references: crops of several levels of each slide,
# rendered by OpenSlide (black box) and read by scopeio.
#
# Usage: REP=3 bash tests/scripts/oracle_mrxs.sh
set -e
cd "$(dirname "$0")/../.."
M=tests/reference/openslide/Mirax
REP=${REP:-1}
O="--tools openslide --openslide-bounds"
first=1
crop() {  # file level x y w h
  local app="--append"; [ $first = 1 ] && app="" && first=0
  python3 -I tests/scripts/oracle.py mrxs --repeat $REP $O --ours-series 0/$2 --crop $3,$4,$5,$6 $app $1
}
crop $M/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs 0 1600 2400 1024 1024
crop $M/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs 1 800 1200 512 512
crop $M/CMU-1-Saved-1_16/CMU-1-Saved-1_16.mrxs 3 100 250 256 256
crop $M/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs 0 14000 20000 1024 1024
crop $M/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs 2 3500 5000 512 512
crop $M/CMU-1-Saved-1_2/CMU-1-Saved-1_2.mrxs 5 400 600 256 256
crop $M/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs 0 5000 3500 1024 1024
crop $M/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs 1 2500 1700 512 512
crop $M/Mirax2-Fluorescence-2/Mirax2-Fluorescence-2.mrxs 3 600 400 256 256
# Slide version 2.2: compressed position buffer
crop $M/Mirax2.2-1/Mirax2.2-1.mrxs 0 44000 61000 1024 1024
crop $M/Mirax2.2-1/Mirax2.2-1.mrxs 2 11000 15250 512 512
crop $M/Mirax2.2-1/Mirax2.2-1.mrxs 5 1300 1800 256 256
crop $M/Mirax2.2-1/Mirax2.2-1.mrxs 9 0 0 185 349

#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Writes larger PNG, JPEG and JPEG 2000 images for the plain-image benchmark
# into tests/reference/plain/ (not committed) and records the reference
# results. Needs ImageMagick (`magick`).
#
# Usage: REP=3 sh tests/scripts/make_plain_bench.sh
set -e
cd "$(dirname "$0")/../.."
B=tests/reference/plain
REP=${REP:-1}
mkdir -p $B
[ -f $B/photo_rgb8.png ] || magick -size 6000x4000 -seed 7 plasma:fractal -blur 0x1 -depth 8 $B/photo_rgb8.png
[ -f $B/photo_rgb8.jpg ] || magick $B/photo_rgb8.png -quality 90 $B/photo_rgb8.jpg
[ -f $B/photo_rgb8.jp2 ] || magick $B/photo_rgb8.png -define jp2:quality=0 $B/photo_rgb8.jp2
[ -f $B/gray16.png ] || magick -size 4096x4096 -seed 11 plasma:fractal -colorspace Gray -depth 16 $B/gray16.png
for f in $B/photo_rgb8.png $B/photo_rgb8.jpg $B/photo_rgb8.jp2 $B/gray16.png; do
  python3 -I tests/scripts/oracle.py plain --repeat $REP --keep-series 0 --append $f
done

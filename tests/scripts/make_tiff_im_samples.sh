#!/bin/sh
# Writes TIFF samples with ImageMagick (libtiff) as an independent writer,
# mainly for codecs not produced by make_tiff_samples.py (JPEG, Zstandard).
# There is no spec-derived truth for these; tests compare them with the
# reference readers. Output in tests/data/tiff_im/ is committed.
#
# Usage: sh tests/scripts/make_tiff_im_samples.sh
set -e
out="$(dirname "$0")/../data/tiff_im"
mkdir -p "$out"
rgb="-size 96x64 gradient:#ff0000-#0000ff ( -size 96x64 gradient:#00ff00-#000000 -rotate 90 -resize 96x64! ) -compose blend -define compose:args=50 -composite"
gray16="-size 80x50 gradient:black-white -depth 16 -colorspace gray"
w() { magick $1 $2 "$out/$3"; }
w "$rgb" "-depth 8 -compress JPEG -quality 85" rgb8_jpeg.tif
w "$rgb" "-depth 8 -compress JPEG -quality 85 -define tiff:tile-geometry=32x32" rgb8_jpeg_tiles.tif
w "$rgb" "-depth 8 -colorspace gray -compress JPEG -quality 90" gray8_jpeg.tif
w "$rgb" "-depth 8 -compress JPEG -sampling-factor 1x1 -define tiff:photometric=rgb" rgb8_jpeg_rgb.tif
w "$gray16" "-compress Zstd" gray16_zstd.tif
w "$gray16" "-compress Zip -define tiff:predictor=2" gray16_zip_pred.tif
w "$gray16" "-compress LZW -define tiff:tile-geometry=16x16 -endian MSB" gray16_lzw_tiles_be.tif
w "$rgb" "-depth 8 -compress RLE" rgb8_packbits.tif
w "$gray16" "-define quantum:format=floating-point -depth 32 -compress Zip -define tiff:predictor=3" gray32f_zip_fpred.tif
w "$rgb" "-depth 16 -compress Zip -interlace Plane" rgb16_zip_planar.tif
magick $gray16 -compress None "TIFF64:$out/gray16_bigtiff.tif"
magick $gray16 \( +clone -negate \) \( +clone -flip \) -adjoin -compress LZW "$out/gray16_3pages.tif"
magick $rgb -depth 8 -alpha set -channel A -evaluate set 60% +channel -compress LZW "$out/rgba8_lzw.tif"

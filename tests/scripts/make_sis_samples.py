#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Olympus SIS TIFF file.

Layout from tifffile (BSD-3-Clause, Christoph Gohlke) read as documentation
and the analySIS 5.0 files of Figshare 19467233 (CC BY 4.0): an 8-bit
palette image whose colour map is a gray ramp with overlay colours in
entries 1-6 and 249-254; OlympusSIS (tag 33560, LONG offset) points to a
60-byte `SIS0` record (minute, hour, day, month from 0, year from 1900 at
10..20, name at 26, sub-record count at 58) followed by (type i16, count
i16, offset u32) sub-records; type 1 holds the length exponent at 10, x and
y pixel size at 12 / 20 (times 10^exponent metres), magnification at 36,
camera at 46 and picture type at 80.

- palette.tif: 9 x 7, 250 nm pixels, taken 2021-03-04 05:06, name
  "synthetic sis".

Also writes truth.tsv with the CRC-32 of the stored indices. Output in
tests/data/sis/ is committed.

Usage: python3 -I tests/scripts/make_sis_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "sis"
W, H = 9, 7


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    pixels = bytes((x * 28 + y * 3) % 256 for y in range(H) for x in range(W))
    out = bytearray(b"II*\0" + bytes(4))
    data_at = len(out)
    out += pixels
    # Gray ramp with overlay colours, as analySIS writes it.
    ramp = [i * 257 for i in range(256)]
    r, g, b = ramp[:], ramp[:], ramp[:]
    for i, (cr, cg, cb) in zip(range(1, 7), [(1, 0, 0), (0, 1, 0), (1, 1, 0), (0, 0, 1), (1, 0, 1), (0, 1, 1)]):
        r[i], g[i], b[i] = cr * 32768, cg * 32768, cb * 32768
    if len(out) % 2:
        out += b"\0"
    cmap_at = len(out)
    out += struct.pack("<768H", *(r + g + b))
    general = bytearray(112)
    struct.pack_into("<h", general, 10, -9)
    struct.pack_into("<dd", general, 12, 250.0, 250.0)
    struct.pack_into("<d", general, 36, 40.0)
    general[46:46 + 11] = b"Test Camera"
    general[80:80 + 10] = b"Gray Value"
    sis_at = len(out)
    rec = bytearray(60)
    rec[0:4] = b"SIS0"
    struct.pack_into("<5h", rec, 10, 6, 5, 4, 2, 121)
    rec[26:26 + 13] = b"synthetic sis"
    struct.pack_into("<H", rec, 58, 1)
    general_at = sis_at + 60 + 8
    out += rec + struct.pack("<hhI", 1, 1, general_at) + general
    if len(out) % 2:
        out += b"\0"
    make_at = len(out)
    make = b"Olympus Soft Imaging Solutions\0"
    out += make
    if len(out) % 2:
        out += b"\0"
    ifd_at = len(out)
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 8), (259, 3, 1, 1), (262, 3, 1, 3),
               (271, 2, len(make), make_at), (273, 4, 1, data_at), (277, 3, 1, 1), (278, 3, 1, H),
               (279, 4, 1, len(pixels)), (320, 3, 768, cmap_at), (33560, 4, 1, sis_at)]
    out += struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    struct.pack_into("<I", out, 4, ifd_at)
    (OUT / "palette.tif").write_bytes(bytes(out))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        f.write(f"palette.tif\t{zlib.crc32(pixels):08x}\n")


if __name__ == "__main__":
    main()

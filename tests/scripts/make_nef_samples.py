#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic uncompressed Nikon NEF file.

Layout from TIFF/EP and the public D300 samples of raw.pixls.us (CC0):
big-endian TIFF; IFD0 is an 8-bit RGB thumbnail with Make "NIKON
CORPORATION" and SubIFDs (330) pointing to the raw image: photometric CFA
(32803), 12 bits per sample packed MSB first (rows padded to whole bytes),
CFARepeatPatternDim 2 x 2 (33421) and CFAPattern R G G B (33422).

- raw12.nef: 10 x 6 CFA samples, 4 x 3 thumbnail.

Also writes truth.tsv with the CRC-32 of the raw samples (uint16, native
byte order). Output in tests/data/nef/ is committed.

Usage: python3 -I tests/scripts/make_nef_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "nef"
W, H = 10, 6


def pack12(values):
    bits = 0
    n = 0
    out = bytearray()
    for v in values:
        bits = bits << 12 | v
        n += 12
        while n >= 8:
            n -= 8
            out.append(bits >> n & 0xFF)
    if n:
        out.append(bits << (8 - n) & 0xFF)
    return bytes(out)


def ifd(entries, next_ifd=0):
    return struct.pack(">H", len(entries)) + b"".join(struct.pack(">HHI", t, ty, c) + v for t, ty, c, v in entries) + struct.pack(">I", next_ifd)


def short(v):
    return struct.pack(">HH", v, 0)


def long(v):
    return struct.pack(">I", v)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    samples = [(x * 397 + y * 211 + (x % 2) * 1000 + (y % 2) * 500) % 4096 for y in range(H) for x in range(W)]
    raw = b"".join(pack12(samples[y * W:(y + 1) * W]) for y in range(H))
    thumb = bytes((x * 60 + c * 30 + y * 10) % 256 for y in range(3) for x in range(4) for c in range(3))
    make = b"NIKON CORPORATION\0"
    out = bytearray(b"MM\0*" + bytes(4))
    raw_at = len(out)
    out += raw
    thumb_at = len(out)
    out += thumb
    make_at = len(out)
    out += make
    bits_at = len(out)
    out += struct.pack(">3H", 8, 8, 8)
    if len(out) % 2:
        out += b"\0"
    raw_ifd_at = len(out)
    raw_entries = [(254, 4, 1, long(0)), (256, 4, 1, long(W)), (257, 4, 1, long(H)), (258, 3, 1, short(12)),
                   (259, 3, 1, short(1)), (262, 3, 1, short(32803)), (273, 4, 1, long(raw_at)),
                   (277, 3, 1, short(1)), (278, 4, 1, long(H)), (279, 4, 1, long(len(raw))),
                   (33421, 3, 2, struct.pack(">HH", 2, 2)), (33422, 1, 4, bytes([0, 1, 1, 2]))]
    out += ifd(raw_entries)
    ifd0_at = len(out)
    entries = [(254, 4, 1, long(1)), (256, 4, 1, long(4)), (257, 4, 1, long(3)), (258, 3, 3, long(bits_at)),
               (259, 3, 1, short(1)), (262, 3, 1, short(2)), (271, 2, len(make), long(make_at)),
               (273, 4, 1, long(thumb_at)), (277, 3, 1, short(3)), (278, 4, 1, long(3)),
               (279, 4, 1, long(len(thumb))), (330, 4, 1, long(raw_ifd_at))]
    out += ifd(entries)
    struct.pack_into(">I", out, 4, ifd0_at)
    (OUT / "raw12.nef").write_bytes(bytes(out))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        f.write(f"raw12.nef\t{zlib.crc32(struct.pack(f'<{len(samples)}H', *samples)):08x}\n")


if __name__ == "__main__":
    main()

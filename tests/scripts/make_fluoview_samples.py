#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Olympus FluoView TIFF files.

Layout from tifffile (BSD-3-Clause, Christoph Gohlke) read as
documentation: one IFD per page; the first IFD carries MM_Header (tag
34361, a packed 1454-byte record: HeaderFlag i2, ImageType u1, ImageName
257 bytes, OffsetData u4, PaletteSize i4, OffsetPalette0/1 u4, CommentSize
i4, OffsetComment u4, then ten dimensions at byte 284 of 100 bytes each:
Name 16 bytes, Size i4, Origin f8, Resolution f8, Unit 64 bytes); every IFD
carries MM_Stamp (tag 34362; tifffile reads 8 doubles, Bio-Formats needs an
integer type and more than 8, so 16 doubles declared as 64 SHORTs are
written). The dimensions after X and Y are listed fastest first.

- zc.tif: uint16, 7 x 5, Z 3 (0.5 um) fastest, then Ch 2; 0.2 um pixels.
- ct.tif: uint8, 7 x 5, Ch 2 fastest, then T 3 (1.5 s); 0.25 um pixels.

Also writes truth.tsv with the CRC-32 of the pixels in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z, t).
Output in tests/data/fluoview/ is committed.

Usage: python3 -I tests/scripts/make_fluoview_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "fluoview"
W, H = 7, 5


def mm_header(name, dims):
    h = bytearray(1454)
    struct.pack_into("<hB", h, 0, 0, 1)
    h[3:3 + len(name)] = name.encode()
    for i, (dname, size, res, unit) in enumerate(dims):
        struct.pack_into("<16sidd64s", h, 284 + 100 * i, dname.encode(), size, 0.0, res, unit.encode())
    return bytes(h)


def tiff(pages, fmt, bits, header):
    """`pages`: list of row-major sample lists; one IFD per page."""
    out = bytearray(b"II*\0" + bytes(4))
    data = []
    for p in pages:
        data.append((len(out), struct.calcsize(f"<{len(p)}{fmt}")))
        out += struct.pack(f"<{len(p)}{fmt}", *p)
    hdr_at = len(out)
    out += header
    stamp_at = len(out)
    out += struct.pack("<16d", *range(16))
    prev = 4
    for k, (at, size) in enumerate(data):
        if len(out) % 2:
            out += b"\0"
        entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, bits), (259, 3, 1, 1), (262, 3, 1, 1),
                   (273, 4, 1, at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, size)]
        if k == 0:
            entries.append((34361, 1, len(header), hdr_at))
        entries.append((34362, 3, 64, stamp_at))
        ifd_at = len(out)
        struct.pack_into("<I", out, prev, ifd_at)
        out += struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries)
        prev = len(out)
        out += struct.pack("<I", 0)
    return bytes(out)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # Z fastest, then channel.
    v = lambda x, y, c, z, t: x * 401 + y * 37 + z * 5000 + c * 20000
    pages = [[v(x, y, c, z, 0) for y in range(H) for x in range(W)] for c in range(2) for z in range(3)]
    dims = [("X", W, 0.2, "um"), ("Y", H, 0.2, "um"), ("Z", 3, 0.5, "um"), ("Ch", 2, 0.0, "")]
    (OUT / "zc.tif").write_bytes(tiff(pages, "H", 16, mm_header("synthetic zc", dims)))
    px = b"".join(struct.pack("<H", v(x, y, c, z, 0)) for z in range(3) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("zc.tif", zlib.crc32(px)))

    # Channel fastest, then time.
    v = lambda x, y, c, z, t: (x * 20 + y * 3 + c * 7 + t * 40) % 256
    pages = [[v(x, y, c, 0, t) for y in range(H) for x in range(W)] for t in range(3) for c in range(2)]
    dims = [("X", W, 0.25, "um"), ("Y", H, 0.25, "um"), ("Ch", 2, 0.0, ""), ("T", 3, 1.5, "s")]
    (OUT / "ct.tif").write_bytes(tiff(pages, "B", 8, mm_header("synthetic ct", dims)))
    px = bytes(v(x, y, c, 0, t) for t in range(3) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("ct.tif", zlib.crc32(px)))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, crc in truth:
            f.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

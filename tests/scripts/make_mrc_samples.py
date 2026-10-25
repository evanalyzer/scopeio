#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic MRC files.

Layout from the MRC2014 specification: a 1024-byte header (nx, ny, nz,
mode, sampling mx/my/mz at 28, cell lengths in Angstrom at 40, axis order at
64, space group at 88, extended header length at 92, extended header type at
104, version at 108, `MAP ` at 208, machine stamp at 212, label count at 220
and ten 80-byte labels at 224), the extended header, then the sections,
rows stored bottom-up. Mode 101 (IMOD documentation): two 4-bit samples per
byte, low nibble first, each row padded to whole bytes.

- i16_be.mrc: big endian, int16 (mode 1), 7 x 5 x 3, 1.5 A pixels, 2 A
  sections, 512-byte extended header, two labels.
- u8_old.st: little endian, uint8 (mode 0), no `MAP ` marker and no machine
  stamp (pre-2000 header; byte order from the sizes), 7 x 5 x 2.
- i8.mrc: MRC2014 (version 20140), int8 (mode 0), 7 x 5 x 1.
- u16_stack.mrcs: uint16 (mode 6), volume stack (space group 401) of two
  volumes of two sections each.
- f16.mrc: half float (mode 12), 7 x 5 x 2, returned as float32.
- nibble.mrc: 4-bit (mode 101), 7 x 5 x 2, returned as uint8.

Also writes truth.tsv with the CRC-32 of the pixels (top row first, native
byte order) in the layout of tests/common/mod.rs `decode_level()`. Output in
tests/data/mrc/ is committed.

Usage: python3 -I tests/scripts/make_mrc_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "mrc"
W, H = 7, 5


def header(e, nz, mode, mz=None, cell=None, ispg=0, ext=0, version=0, marker=True, labels=()):
    h = bytearray(1024)
    mz = nz if mz is None else mz
    struct.pack_into(f"{e}4i", h, 0, W, H, nz, mode)
    struct.pack_into(f"{e}3i", h, 28, W, H, mz)
    if cell:
        struct.pack_into(f"{e}3f", h, 40, *cell)
    struct.pack_into(f"{e}3f", h, 52, 90.0, 90.0, 90.0)
    struct.pack_into(f"{e}3i", h, 64, 1, 2, 3)
    struct.pack_into(f"{e}2i", h, 88, ispg, ext)
    struct.pack_into(f"{e}i", h, 108, version)
    if marker:
        h[208:212] = b"MAP "
        h[212:214] = b"\x44\x44" if e == "<" else b"\x11\x11"
    struct.pack_into(f"{e}i", h, 220, len(labels))
    for i, label in enumerate(labels):
        h[224 + 80 * i:224 + 80 * i + len(label)] = label.encode()
    return bytes(h) + bytes(ext)


def sections(e, fmt, value, nz):
    """Stored sections (bottom row first) and the expected pixels (top row
    first, native order)."""
    data, truth = bytearray(), bytearray()
    for z in range(nz):
        rows = [[value(x, y, z) for x in range(W)] for y in range(H)]
        for row in reversed(rows):
            data += struct.pack(f"{e}{W}{fmt}", *row)
        for row in rows:
            truth += struct.pack(f"={W}{'f' if fmt == 'e' else fmt}", *row)
    return bytes(data), bytes(truth)


def nibble_sections(value, nz):
    data, truth = bytearray(), bytearray()
    for z in range(nz):
        rows = [[value(x, y, z) for x in range(W)] for y in range(H)]
        for row in reversed(rows):
            padded = row + [0] * (len(row) % 2)
            data += bytes(padded[i] | padded[i + 1] << 4 for i in range(0, len(padded), 2))
        for row in rows:
            truth += bytes(row)
    return bytes(data), bytes(truth)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    files = []

    d, t = sections(">", "h", lambda x, y, z: x * 1000 - y * 3000 + z * 7 - 5000, 3)
    files.append(("i16_be.mrc", header(">", 3, 1, cell=(W * 1.5, H * 1.5, 3 * 2.0), ext=512,
                                       labels=("synthetic", "second label")) + d, t))

    d, t = sections("<", "B", lambda x, y, z: x * 30 + y * 5 + z * 50, 2)
    files.append(("u8_old.st", header("<", 2, 0, marker=False) + d, t))

    d, t = sections("<", "b", lambda x, y, z: x * 20 - y * 25, 1)
    files.append(("i8.mrc", header("<", 1, 0, version=20140) + d, t))

    d, t = sections("<", "H", lambda x, y, z: x * 401 + y * 77 + z * 9000, 4)
    files.append(("u16_stack.mrcs", header("<", 4, 6, mz=2, cell=(W * 2.0, H * 2.0, 2 * 4.0), ispg=401) + d, t))

    d, t = sections("<", "e", lambda x, y, z: x * 0.5 - y * 1.25 + z * 100.0, 2)
    files.append(("f16.mrc", header("<", 2, 12) + d, t))

    d, t = nibble_sections(lambda x, y, z: (x + 2 * y + 3 * z) % 16, 2)
    files.append(("nibble.mrc", header("<", 2, 101) + d, t))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, data, truth in files:
            (OUT / name).write_bytes(data)
            f.write(f"{name}\t{zlib.crc32(truth):08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Keller Lab Block files.

Layout from the KLB README (doc/klb/) and the public sample files: header
(u8 version 2, u32 xyzct[5], f32 pixelSize[5], u8 dataType, u8
compressionType, char metadata[256], u32 blockSize[5]), one u64 per block
(end of its data after the header), blocks x fastest then y, z, c, t,
each with its clipped samples x fastest, little endian.

- stack.klb: uint16 7 x 5 x 3, 2 channels, 2 time points, bzip2 blocks of
  4 x 3 x 2 x 1 x 1 (edge blocks clipped), pixel size 0.5 / 0.5 / 2 um,
  2.5 s.
- plain.klb: float32 6 x 4, uncompressed, one block of 4 x 4.
- int8.klb: int8 5 x 3 x 1 x 3 channels, bzip2, block 5 x 3 x 1 x 2.

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/klb/ is
committed.

Usage: python3 -I tests/scripts/make_klb_samples.py
"""

import bz2
import itertools
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "klb"
TYPES = {"u1": (0, "B"), "u2": (1, "H"), "f4": (8, "f"), "i1": (4, "b")}


def klb(dims, pixel, dtype, block, value, compress, meta=b""):
    code, fmt = TYPES[dtype]
    grid = [-(-d // b) for d, b in zip(dims, block)]
    blobs = []
    for bt, bc, bz, by, bx in itertools.product(*[range(g) for g in reversed(grid)]):
        idx = [bx, by, bz, bc, bt]
        lo = [i * b for i, b in zip(idx, block)]
        hi = [min(l + b, d) for l, b, d in zip(lo, block, dims)]
        vals = [value(x, y, z, c, t) for t in range(lo[4], hi[4]) for c in range(lo[3], hi[3])
                for z in range(lo[2], hi[2]) for y in range(lo[1], hi[1]) for x in range(lo[0], hi[0])]
        raw = struct.pack(f"<{len(vals)}{fmt}", *vals)
        blobs.append(bz2.compress(raw) if compress else raw)
    ends, at = [], 0
    for b in blobs:
        at += len(b)
        ends.append(at)
    head = bytes([2]) + struct.pack("<5I", *dims) + struct.pack("<5f", *pixel) + bytes([code, 1 if compress else 0])
    head += meta.ljust(256, b"\0") + struct.pack("<5I", *block) + struct.pack(f"<{len(ends)}Q", *ends)
    return head + b"".join(blobs)


def truth(dims, dtype, value):
    fmt = TYPES[dtype][1]
    x, y, z, c, t = dims
    vals = [value(i, j, k, ch, tt) for tt in range(t) for k in range(z) for j in range(y) for i in range(x) for ch in range(c)]
    return zlib.crc32(struct.pack(f"<{len(vals)}{fmt}", *vals))


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    rows = []
    v = lambda x, y, z, c, t: (x * 1000 + y * 77 + z * 3000 + c * 20000 + t * 11) % 65536
    dims = (7, 5, 3, 2, 2)
    (OUT / "stack.klb").write_bytes(klb(dims, (0.5, 0.5, 2.0, 1.0, 2.5), "u2", (4, 3, 2, 1, 1), v, True, b"synthetic stack"))
    rows.append(("stack.klb", truth(dims, "u2", v)))
    f = lambda x, y, z, c, t: x * 0.5 - y * 1.25
    (OUT / "plain.klb").write_bytes(klb((6, 4, 1, 1, 1), (1, 1, 1, 1, 1), "f4", (4, 4, 1, 1, 1), f, False))
    rows.append(("plain.klb", truth((6, 4, 1, 1, 1), "f4", f)))
    s = lambda x, y, z, c, t: (x * 20 - y * 30 + c * 7) % 256 - 128
    (OUT / "int8.klb").write_bytes(klb((5, 3, 1, 3, 1), (1, 1, 1, 1, 1), "i1", (5, 3, 1, 2, 1), s, True))
    rows.append(("int8.klb", truth((5, 3, 1, 3, 1), "i1", s)))
    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in rows:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

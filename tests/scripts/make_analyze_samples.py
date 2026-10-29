#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Analyze 7.5 header/image pairs.

Header layout from the Mayo Analyze 7.5 format document and G. Wideman's
field compilation (doc/analyze/): sizeof_hdr 348, extents 16384, regular
"r", dim[0..7] at 40, vox_units at 56, datatype and bitpix at 70,
pixdim[0..7] at 76, vox_offset at 108, funused1 (SPM scale) at 112,
descrip at 148, orient at 252. No NIfTI magic.

- be_i16.hdr/.img: big-endian int16, 7 x 5 x 3, 0.5 x 0.5 x 2 mm.
- le_u8_4d.hdr/.img: little-endian uint8, 7 x 5 x 2 x 3 (time), voxels at
  vox_offset 16 in the .img, units "um", SPM scale 2.5.
- f32.hdr/.img: little-endian float32, 7 x 5, no unit.
- rgb.hdr/.img: RGB (datatype 128), 7 x 5 x 2, interleaved per voxel.

Also writes truth.tsv with the CRC-32 of the voxels in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z,
t). Output in tests/data/analyze/ is committed.

Usage: python3 -I tests/scripts/make_analyze_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "analyze"
W, H = 7, 5


def header(e, dims, datatype, bitpix, pixdim, units, offset=0.0, scale=0.0):
    h = bytearray(348)
    struct.pack_into(f"{e}i", h, 0, 348)
    struct.pack_into(f"{e}i", h, 32, 16384)
    h[38] = ord("r")
    struct.pack_into(f"{e}8h", h, 40, len(dims), *dims, *([0] * (7 - len(dims))))
    h[56:56 + len(units)] = units
    struct.pack_into(f"{e}2h", h, 70, datatype, bitpix)
    struct.pack_into(f"{e}8f", h, 76, 0.0, *pixdim, *([0.0] * (7 - len(pixdim))))
    struct.pack_into(f"{e}2f", h, 108, offset, scale)
    h[148:148 + 9] = b"synthetic"
    return bytes(h)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    v = lambda x, y, z: x * 401 - y * 37 + z * 3000 - 9000
    (OUT / "be_i16.hdr").write_bytes(header(">", [W, H, 3], 4, 16, [0.5, 0.5, 2.0], b"mm"))
    (OUT / "be_i16.img").write_bytes(b"".join(struct.pack(">h", v(x, y, z)) for z in range(3) for y in range(H) for x in range(W)))
    truth.append(("be_i16.hdr", zlib.crc32(b"".join(struct.pack("<h", v(x, y, z)) for z in range(3) for y in range(H) for x in range(W)))))

    u = lambda x, y, z, t: (x * 30 + y * 7 + z * 50 + t * 90) % 256
    data = bytes(u(x, y, z, t) for t in range(3) for z in range(2) for y in range(H) for x in range(W))
    (OUT / "le_u8_4d.hdr").write_bytes(header("<", [W, H, 2, 3], 2, 8, [0.25, 0.25, 1.0, 1.5], b"um", 16.0, 2.5))
    (OUT / "le_u8_4d.img").write_bytes(bytes(range(16)) + data)
    truth.append(("le_u8_4d.hdr", zlib.crc32(data)))

    f = lambda x, y: x * 0.5 - y * 1.25
    data = b"".join(struct.pack("<f", f(x, y)) for y in range(H) for x in range(W))
    (OUT / "f32.hdr").write_bytes(header("<", [W, H], 16, 32, [1.0, 1.0], b""))
    (OUT / "f32.img").write_bytes(data)
    truth.append(("f32.hdr", zlib.crc32(data)))

    rgb = lambda x, y, z: bytes([(x * 30) % 256, (y * 50) % 256, (z * 100 + x) % 256])
    data = b"".join(rgb(x, y, z) for z in range(2) for y in range(H) for x in range(W))
    (OUT / "rgb.hdr").write_bytes(header("<", [W, H, 2], 128, 24, [0.1, 0.1, 0.2], b"mm"))
    (OUT / "rgb.img").write_bytes(data)
    truth.append(("rgb.hdr", zlib.crc32(data)))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Writes small synthetic DeltaVision files.

Layout from the Priism IVE header description (doc/deltavision-dv/): a
1024-byte header (width, height, sections, mode, spacing at 40/44/48, extended
header length at 92, identifier -16224 at 96, integers / floats per section
at 128/130, time points at 180, section order at 182, wavelengths at 196 and
198..), the extended header (per section: 8 int32, 32 float32 whose first
ones are photosensor, time stamp, stage x/y/z, min, max, mean, exposure,
neutral density, excitation, emission), then the sections, rows stored
bottom-up.

- wzt_be.r3d: big endian, uint16 (mode 6), 2 wavelengths x 3 z x 2 time
  points of 7 x 5, section order wavelength-Z-T, time stamps 0 / 2.5 s.
- ztw_le.dv: little endian, float32 (mode 2), 1 wavelength x 2 z, order
  Z-T-wavelength.

Also writes truth.tsv with the CRC-32 of the pixels (top row first) in the
layout of tests/common/mod.rs `decode_level()`. Output in tests/data/dv/ is
committed.

Usage: python3 -I tests/scripts/make_dv_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "dv"
W, H = 7, 5


def dv(e, mode, fmt, c, z, t, order, sections, waves, spacing, times):
    """`sections`: list of (c, z, t) per stored section; returns bytes."""
    h = bytearray(1024)
    ext_ints, ext_floats = 8, 32
    ext = bytearray()
    for (ci, zi, ti) in sections:
        floats = [0.0] * ext_floats
        floats[1] = times[ti]
        floats[8] = 0.1 * (ci + 1)
        floats[10], floats[11] = waves[ci] - 50, waves[ci]
        ext += struct.pack(f"{e}{ext_ints}i", *([0] * ext_ints)) + struct.pack(f"{e}{ext_floats}f", *floats)
    ext += bytes(-len(ext) % 1024)
    struct.pack_into(f"{e}4i", h, 0, W, H, len(sections), mode)
    struct.pack_into(f"{e}3f", h, 40, *spacing)
    struct.pack_into(f"{e}i", h, 92, len(ext))
    struct.pack_into(f"{e}h", h, 96, -16224)
    struct.pack_into(f"{e}2h", h, 128, ext_ints, ext_floats)
    struct.pack_into(f"{e}2h", h, 180, t, order)
    struct.pack_into(f"{e}h", h, 196, c)
    for i, wl in enumerate(waves):
        struct.pack_into(f"{e}h", h, 198 + 2 * i, wl)
    h[224:224 + 9] = b"synthetic"
    data = bytearray()
    for (ci, zi, ti) in sections:
        rows = [[fmt[1](x, y, ci, zi, ti) for x in range(W)] for y in range(H)]
        for row in reversed(rows):  # bottom row first
            data += struct.pack(f"{e}{W}{fmt[0]}", *row)
    return bytes(h) + bytes(ext) + bytes(data), sections


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    v16 = lambda x, y, c, z, t: (x * 401 + y * 77 + c * 9000 + z * 1300 + t * 20000) % 65536
    secs = [(c, z, t) for t in range(2) for z in range(3) for c in range(2)]  # wavelength fastest
    data, _ = dv(">", 6, ("H", v16), 2, 3, 2, 1, secs, [525, 610], (0.1, 0.1, 0.3), [0.0, 2.5])
    (OUT / "wzt_be.r3d").write_bytes(data)
    px = b"".join(struct.pack("=H", v16(x, y, c, z, t)) for t in range(2) for z in range(3) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("wzt_be.r3d", zlib.crc32(px)))

    vf = lambda x, y, c, z, t: x * 0.5 - y * 1.25 + z * 100.0
    secs = [(0, z, 0) for z in range(2)]
    data, _ = dv("<", 2, ("f", vf), 1, 2, 1, 0, secs, [460], (0.2, 0.2, 0.5), [0.0])
    (OUT / "ztw_le.dv").write_bytes(data)
    px = b"".join(struct.pack("=f", vf(x, y, 0, z, 0)) for z in range(2) for y in range(H) for x in range(W))
    truth.append(("ztw_le.dv", zlib.crc32(px)))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, crc in truth:
            f.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

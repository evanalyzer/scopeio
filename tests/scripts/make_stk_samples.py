#!/usr/bin/env python3
"""Writes small synthetic MetaMorph STK files.

Layout from the MetaMorph STK description (doc/metamorph-stk/) and
tifffile (BSD-3-Clause): a little-endian TIFF whose IFD describes the first
plane (one uncompressed strip), the further planes stored right after it;
UIC2 (33629, type RATIONAL, count = planes) points to 6 u32 per plane
(z distance numerator/denominator, creation Julian day and milliseconds,
modification day and milliseconds); UIC1 (33628, LONG) to (id, value) pairs:
4 / 5 x / y calibration (offset to a fraction), 6 calibration units
(offset to u32 length + text).

- zstack.stk: 4 planes of 9 x 7 uint16, z distance 3/2, 0.25 um pixels.
- timeseries.stk: 3 planes, z distance 0, planes 1.5 s apart.

Also writes truth.tsv with the CRC-32 of the pixels in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/stk/ is
committed.

Usage: python3 -I tests/scripts/make_stk_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "stk"
W, H = 9, 7


def stk(planes, zdist, millis, cal, unit):
    n = len(planes)
    data = b"".join(planes)
    out = bytearray(b"II*\0" + bytes(4))
    data_at = len(out)
    out += data
    # UIC2 records, calibration fractions, unit text.
    uic2_at = len(out)
    for k in range(n):
        out += struct.pack("<6I", zdist[0], zdist[1], 2460000, millis[k], 2460000, millis[k])
    frac_at = len(out)
    out += struct.pack("<II", *cal)
    unit_at = len(out)
    u = unit.encode() + b"\0"
    out += struct.pack("<I", len(u)) + u
    uic1_at = len(out)
    pairs = [(3, 1), (4, frac_at), (5, frac_at), (6, unit_at)]
    out += b"".join(struct.pack("<II", *p) for p in pairs)
    if len(out) % 2:
        out += b"\0"
    ifd_at = len(out)
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, data_at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(planes[0])),
               (33628, 4, len(pairs), uic1_at), (33629, 5, n, uic2_at)]
    out += struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    struct.pack_into("<I", out, 4, ifd_at)
    return bytes(out)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    for name, n, zdist, step in (("zstack.stk", 4, (3, 2), 0), ("timeseries.stk", 3, (0, 1), 1500)):
        planes = [b"".join(struct.pack("<H", (x * 401 + y * 37 + k * 5000 + n) % 65536) for y in range(H) for x in range(W)) for k in range(n)]
        (OUT / name).write_bytes(stk(planes, zdist, [1000 + k * step for k in range(n)], (25, 100), "um"))
        truth.append((name, zlib.crc32(b"".join(planes))))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, crc in truth:
            f.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

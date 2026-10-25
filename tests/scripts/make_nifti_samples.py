#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic NIfTI files.

Layout from nifti1.h and nifti2.h (public domain, doc/nifti/): NIfTI-1
header of 348 bytes (dim i16[8] at 40, datatype at 70, bitpix at 72,
pixdim f32[8] at 76, vox_offset f32 at 108, xyzt_units at 123, descrip at
148, magic at 344), NIfTI-2 header of 540 bytes (magic at 4, datatype at
12, bitpix at 14, dim i64[8] at 16, pixdim f64[8] at 104, vox_offset i64 at
168, descrip at 240, xyzt_units i32 at 500); voxels x fastest, then y, z,
t, u.

- le_u16_5d.nii: NIfTI-1 little endian, uint16 (512), 7 x 5 x 3 x 2 x 2
  (x, y, z, t, u), 0.5 x 0.5 x 1.5 um, 2 s per time point.
- rgb.nii: NIfTI-1 big endian, RGB24 (128), 7 x 5 x 2, mm.
- v2_f64.nii.gz: NIfTI-2 little endian, float64, 7 x 5 x 2, gzip.
- v2_pair.hdr / v2_pair.img: NIfTI-2 header and image pair, int8.

Also writes truth.tsv with the CRC-32 of the voxels in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z,
t). Output in tests/data/nifti/ is committed.

Usage: python3 -I tests/scripts/make_nifti_samples.py
"""

import gzip
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "nifti"
W, H = 7, 5


def nifti1(e, dims, datatype, bitpix, pixdim, units, magic, offset):
    h = bytearray(348)
    struct.pack_into(f"{e}i", h, 0, 348)
    struct.pack_into(f"{e}8h", h, 40, len(dims), *dims, *([1] * (7 - len(dims))))
    struct.pack_into(f"{e}2h", h, 70, datatype, bitpix)
    struct.pack_into(f"{e}8f", h, 76, 1.0, *pixdim, *([0.0] * (7 - len(pixdim))))
    struct.pack_into(f"{e}f", h, 108, offset)
    h[123] = units
    h[148:148 + 9] = b"synthetic"
    h[344:348] = magic
    return bytes(h) + bytes(offset - 348) if offset else bytes(h)


def nifti2(e, dims, datatype, bitpix, pixdim, units, magic, offset):
    h = bytearray(540)
    struct.pack_into(f"{e}i", h, 0, 540)
    h[4:12] = magic + b"\r\n\x1a\n"
    struct.pack_into(f"{e}2h", h, 12, datatype, bitpix)
    struct.pack_into(f"{e}8q", h, 16, len(dims), *dims, *([1] * (7 - len(dims))))
    struct.pack_into(f"{e}8d", h, 104, 1.0, *pixdim, *([0.0] * (7 - len(pixdim))))
    struct.pack_into(f"{e}q", h, 168, offset)
    h[240:240 + 9] = b"synthetic"
    struct.pack_into(f"{e}i", h, 500, units)
    return bytes(h) + bytes(offset - 540) if offset else bytes(h)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # 5-D uint16: channels are the fifth dimension (u).
    nz, nt, nu = 3, 2, 2
    v = lambda x, y, z, t, u: x * 401 + y * 37 + z * 3000 + t * 9000 + u * 30000
    data = b"".join(struct.pack("<H", v(x, y, z, t, u)) for u in range(nu) for t in range(nt) for z in range(nz) for y in range(H) for x in range(W))
    hdr = nifti1("<", [W, H, nz, nt, nu], 512, 16, [0.5, 0.5, 1.5, 2.0], 3 | 8, b"n+1\0", 352)
    (OUT / "le_u16_5d.nii").write_bytes(hdr + data)
    px = b"".join(struct.pack("<H", v(x, y, z, t, u)) for t in range(nt) for z in range(nz) for y in range(H) for x in range(W) for u in range(nu))
    truth.append(("le_u16_5d.nii", zlib.crc32(px)))

    # RGB24, big endian header.
    rgb = lambda x, y, z: bytes([(x * 30) % 256, (y * 50) % 256, (z * 100 + x) % 256])
    data = b"".join(rgb(x, y, z) for z in range(2) for y in range(H) for x in range(W))
    (OUT / "rgb.nii").write_bytes(nifti1(">", [W, H, 2], 128, 24, [0.1, 0.1, 0.2], 2, b"n+1\0", 352) + data)
    truth.append(("rgb.nii", zlib.crc32(data)))

    # NIfTI-2 float64, gzip-compressed.
    f = lambda x, y, z: x * 0.25 - y * 1.5 + z * 10.0
    data = b"".join(struct.pack("<d", f(x, y, z)) for z in range(2) for y in range(H) for x in range(W))
    raw = nifti2("<", [W, H, 2], 64, 64, [2.0, 2.0, 3.0], 2, b"n+2\0", 544) + data
    (OUT / "v2_f64.nii.gz").write_bytes(gzip.compress(raw, mtime=0))
    truth.append(("v2_f64.nii.gz", zlib.crc32(data)))

    # NIfTI-2 header + image pair, int8.
    i8 = lambda x, y, z: x * 10 - y * 20 + z
    data = b"".join(struct.pack("<b", i8(x, y, z)) for z in range(2) for y in range(H) for x in range(W))
    (OUT / "v2_pair.hdr").write_bytes(nifti2("<", [W, H, 2], 256, 8, [1.0, 1.0, 1.0], 3, b"ni2\0", 0))
    (OUT / "v2_pair.img").write_bytes(data)
    truth.append(("v2_pair.hdr", zlib.crc32(data)))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

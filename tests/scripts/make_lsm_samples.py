#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Zeiss LSM file.

Layout from "File Format Description - LSM 5xx Release 2.0"
(doc/zeiss-lsm/Lsmfile.doc): little-endian TIFF, one image directory per
plane (z fastest, then time) followed by a thumbnail directory
(NewSubfileType 1), channels as planar samples with one strip each, and the
CZ_LSMINFO tag (34412) with dimensions, voxel sizes, the channel colours and
names and the time stamps. Two 12-bit channels in uint16, 3 z, 2 t; a
palette photometric as LSM writes for display. Also writes truth.tsv with
the CRC-32 in the layout of tests/common/mod.rs `decode()`. Output in
tests/data/lsm/ is committed.

Usage: python3 -I tests/scripts/make_lsm_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "lsm"
W, H, Z, C, T = 6, 4, 3, 2, 2


def value(x, y, c, z, t):
    return (x * 37 + y * 101 + c * 1009 + z * 211 + t * 503) % 4096


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    data = bytearray(b"II*\0\0\0\0\0")
    # Extra structures first: channel colours/names and time stamps.
    names = b"Ch1-T1\0Ch2-T2\0"
    colors = struct.pack("<2I", 0x0000FF00, 0x000000FF)  # green, red (0x00BBGGRR)
    cc_head = 24
    block = struct.pack("<6i", cc_head + len(colors) + len(names), 2, 2, cc_head, cc_head + len(colors), 0) + colors + names
    colors_at = len(data)
    data += block
    stamps_at = len(data)
    data += struct.pack("<ii", 8 + 16, T) + struct.pack(f"<{T}d", 100.0, 104.5)
    info = bytearray(464)
    struct.pack_into("<Iiiiiiii", info, 0, 0x0400494C, 464, W, H, Z, C, T, 2)
    struct.pack_into("<ii", info, 32, 2, 2)
    struct.pack_into("<ddd", info, 40, 0.2e-6, 0.2e-6, 1.5e-6)
    struct.pack_into("<I", info, 64, 0)
    struct.pack_into("<I", info, 84, colors_at)
    struct.pack_into("<d", info, 88, 4.5)
    struct.pack_into("<I", info, 108, stamps_at)
    info_at = len(data)
    data += bytes(info)
    cmap_at = len(data)
    data += struct.pack("<768H", *([v * 257 for v in range(256)] * 3))
    prev = 4
    truth = bytearray()
    for t in range(T):
        for z in range(Z):
            for y in range(H):
                for x in range(W):
                    for c in range(C):
                        truth += struct.pack("<H", value(x, y, c, z, t))
            for thumb in (False, True):
                strips = []
                spp = 3 if thumb else C
                for c in range(spp):
                    strips.append(len(data))
                    if thumb:
                        data += bytes([(c * 40 + i) % 256 for i in range(2 * 2)])
                    else:
                        data += b"".join(struct.pack("<H", value(x, y, c, z, t)) for y in range(H) for x in range(W))
                counts = [4] * 3 if thumb else [W * H * 2] * C
                extra = bytearray()
                base = len(data)

                def ext(b):
                    pos = base + len(extra)
                    extra.extend(b)
                    return pos

                w, h = (2, 2) if thumb else (W, H)
                bits = 8 if thumb else 16
                entries = [(254, 4, 1, 1 if thumb else 0), (256, 4, 1, w), (257, 4, 1, h),
                           (258, 3, spp, ext(struct.pack(f"<{spp}H", *[bits] * spp)) if spp > 2 else bits | (bits << 16)),
                           (259, 3, 1, 1), (262, 3, 1, 2 if thumb else 3),
                           (273, 4, spp, ext(struct.pack(f"<{spp}I", *strips))),
                           (277, 3, 1, spp), (278, 4, 1, h),
                           (279, 4, spp, ext(struct.pack(f"<{spp}I", *counts))), (284, 3, 1, 2)]
                if not thumb:
                    entries.append((320, 3, 768, cmap_at))
                if prev == 4:
                    entries.append((34412, 1, 464, info_at))
                data += extra
                at = len(data)
                struct.pack_into("<I", data, prev, at)
                data += struct.pack("<H", len(entries))
                for tag, typ, count, val in sorted(entries):
                    v = struct.pack("<HH", val & 0xFFFF, val >> 16) if typ == 3 and count <= 2 else struct.pack("<I", val)
                    data += struct.pack("<HHI", tag, typ, count) + v
                prev = len(data)
                data += b"\0\0\0\0"
    (OUT / "synthetic.lsm").write_bytes(bytes(data))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        f.write(f"synthetic.lsm\t0\t{zlib.crc32(bytes(truth)):08x}\n")


if __name__ == "__main__":
    main()

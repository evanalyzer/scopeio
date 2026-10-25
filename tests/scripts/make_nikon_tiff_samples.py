#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Nikon NIS-Elements TIFF file.

Layout from public NIS-Elements TIFF exports (Zenodo 7677868, CC BY 4.0):
a gray TIFF whose IFD carries NIS private tags; tag 65331 (BYTE) is a
directory (u32 entry count, then per entry u32 name length in UTF-16
units, the name, u32 flags 1 and 0, u32 size, u32 offset of the entry's
data in the tag) with the entry `MetadataTiffV1_0`, the picture metadata in
the ND2 lite-variant encoding (item: type byte, name length incl. the
terminator, UTF-16 name, value; type 11 levels hold a u32 child count, a u64
length, the children and one u64 offset per child). Tag 65330 is present
as in the exports (an empty directory here).

- gray.tif: uint16 9 x 7, 0.325 um pixels, objective "Plan Apo 20x" NA
  0.75, stage at (100, -200, 3000) um, plane "DAPI".

Also writes truth.tsv with the CRC-32 of the pixels. Output in
tests/data/nikon-tiff/ is committed.

Usage: python3 -I tests/scripts/make_nikon_tiff_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "nikon-tiff"
W, H = 9, 7


def name(n):
    return bytes([len(n) + 1]) + (n + "\0").encode("utf-16le")


def item(typ, n, payload):
    return bytes([typ]) + name(n) + payload


def f64(n, v):
    return item(6, n, struct.pack("<d", v))


def text(n, v):
    return item(8, n, (v + "\0").encode("utf-16le"))


def flag(n, v):
    return item(1, n, bytes([v]))


def level(n, children):
    body = b"".join(children)
    return item(11, n, struct.pack("<IQ", len(children), len(body)) + body + bytes(8 * len(children)))


def metadata():
    plane = level("a0", [text("sDescription", "DAPI")])
    pic = level("SLxPictureMetadata", [
        f64("dTimeAbsolute", 2459888.5), f64("dXPos", 100.0), f64("dYPos", -200.0), f64("dZPos", 3000.0),
        level("sPicturePlanes", [level("sPlaneNew", [plane])]),
        f64("dCalibration", 0.325), f64("dAspect", 1.0), flag("bCalibrated", 1),
        text("wsObjectiveName", "Plan Apo 20x"), f64("dObjectiveMag", -1.0), f64("dObjectiveNA", 0.75),
    ])
    entry = "MetadataTiffV1_0"
    head_len = 4 + 4 + 2 * len(entry) + 16
    return (struct.pack("<II", 1, len(entry)) + entry.encode("utf-16le")
            + struct.pack("<4I", 1, 0, len(pic), head_len) + pic)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    pixels = struct.pack(f"<{W * H}H", *((x * 701 + y * 97) % 65536 for y in range(H) for x in range(W)))
    meta = metadata()
    empty = struct.pack("<I", 0)
    out = bytearray(b"II*\0" + bytes(4))
    data_at = len(out)
    out += pixels
    meta_at = len(out)
    out += meta
    empty_at = len(out)
    out += empty
    if len(out) % 2:
        out += b"\0"
    ifd_at = len(out)
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, data_at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(pixels)),
               (65330, 1, len(empty), empty_at), (65331, 1, len(meta), meta_at)]
    out += struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    struct.pack_into("<I", out, 4, ifd_at)
    (OUT / "gray.tif").write_bytes(bytes(out))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        f.write(f"gray.tif\t{zlib.crc32(pixels):08x}\n")


if __name__ == "__main__":
    main()

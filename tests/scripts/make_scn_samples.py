#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Leica SCN file.

Layout from the openslide.org Leica page (doc/leica-scn/): a BigTIFF whose
first ImageDescription is the SCN XML (namespace 2010/10/01) with a
collection of images; each image lists dimensions (level r, channel c)
pointing at TIFF directories. A macro image covering the collection, a
brightfield image with two levels (RGB, tiled) and a fluorescence image with
two gray channels. Uncompressed. Also writes truth.tsv with the CRC-32 of
each series level and the macro. Output in tests/data/scn/ is committed.

Usage: python3 -I tests/scripts/make_scn_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "scn"


def rgb(w, h, seed):
    return bytes(((x * 3 + y * 5 + k * 61 + seed * 17) % 256) for y in range(h) for x in range(w) for k in range(3))


def gray(w, h, seed):
    return bytes(((x * 7 + y * 11 + seed * 53) % 256) for y in range(h) for x in range(w))


def tiles(px, w, h, t, bpp):
    out = []
    for ty in range(0, h, t):
        for tx in range(0, w, t):
            b = bytearray()
            for y in range(ty, ty + t):
                for x in range(tx, tx + t):
                    b += px[(y * w + x) * bpp:(y * w + x + 1) * bpp] if x < w and y < h else bytes(bpp)
            out.append(bytes(b))
    return out


class BigTiff:
    def __init__(self):
        self.data = bytearray(b"II+\0\x08\0\0\0" + bytes(8))
        self.prev = 8
        self.count = 0

    def ifd(self, w, h, px, spp, description=None, tile=16):
        chunks = tiles(px, w, h, tile, spp)
        offsets = []
        for c in chunks:
            offsets.append(len(self.data))
            self.data += c
        extra = bytearray()
        base = len(self.data)

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            return pos

        e = [(256, 4, 1, w), (257, 4, 1, h), (258, 3, spp, ext(struct.pack(f"<{spp}H", *[8] * spp)) if spp > 4 else
              struct.unpack("<Q", struct.pack(f"<{spp}H", *[8] * spp).ljust(8, b"\0"))[0]),
             (259, 3, 1, 1), (262, 3, 1, 2 if spp == 3 else 1), (277, 3, 1, spp), (284, 3, 1, 1),
             (322, 3, 1, tile), (323, 3, 1, tile),
             # One LONG8 fits inline in BigTIFF.
             (324, 16, len(offsets), offsets[0] if len(offsets) == 1 else ext(struct.pack(f"<{len(offsets)}Q", *offsets))),
             (325, 16, len(chunks), len(chunks[0]) if len(chunks) == 1 else ext(struct.pack(f"<{len(chunks)}Q", *map(len, chunks))))]
        if description:
            d = description.encode() + b"\0"
            e.append((270, 2, len(d), ext(d)))
        self.data += extra
        at = len(self.data)
        struct.pack_into("<Q", self.data, self.prev, at)
        self.data += struct.pack("<Q", len(e))
        for tag, typ, count, value in sorted(e):
            self.data += struct.pack("<HHQQ", tag, typ, count, value)
        self.prev = len(self.data)
        self.data += bytes(8)
        self.count += 1
        return self.count - 1


XML = """<?xml version="1.0"?>
<scn xmlns="http://www.leica-microsystems.com/scn/2010/10/01">
  <collection name="synthetic" sizeX="1000000" sizeY="2000000">
    <image name="macro">
      <pixels sizeX="8" sizeY="16"><dimension sizeX="8" sizeY="16" r="0" ifd="0"/></pixels>
      <view sizeX="1000000" sizeY="2000000" offsetX="0" offsetY="0" spacingZ="0"/>
    </image>
    <image name="bright">
      <creationDate>2021-04-05T06:07:08Z</creationDate>
      <pixels sizeX="40" sizeY="24">
        <dimension sizeX="40" sizeY="24" r="0" ifd="1"/>
        <dimension sizeX="10" sizeY="6" r="1" ifd="2"/>
      </pixels>
      <view sizeX="20000" sizeY="12000" offsetX="300000" offsetY="400000" spacingZ="0"/>
      <scanSettings><objectiveSettings><objective>20</objective></objectiveSettings>
        <illuminationSettings><numericalAperture>0.7</numericalAperture><illuminationSource>brightfield</illuminationSource></illuminationSettings>
      </scanSettings>
    </image>
    <image name="fluo">
      <pixels sizeX="20" sizeY="16">
        <dimension sizeX="20" sizeY="16" r="0" c="0" ifd="3"/>
        <dimension sizeX="20" sizeY="16" r="0" c="1" ifd="4"/>
      </pixels>
      <view sizeX="10000" sizeY="8000" offsetX="500000" offsetY="900000" spacingZ="0"/>
      <scanSettings><illuminationSettings><illuminationSource>fluorescence</illuminationSource></illuminationSettings>
        <channelSettings><channel index="0" name="DAPI" rgb="#0000ff"><exposureTime>20000</exposureTime></channel>
          <channel index="1" name="FITC" rgb="#00ff00"><exposureTime>50000</exposureTime></channel></channelSettings>
      </scanSettings>
    </image>
  </collection>
</scn>"""


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    t = BigTiff()
    macro = rgb(8, 16, 1)
    t.ifd(8, 16, macro, 3, XML)
    b0, b1 = rgb(40, 24, 2), rgb(10, 6, 3)
    t.ifd(40, 24, b0, 3)
    t.ifd(10, 6, b1, 3)
    f0, f1 = gray(20, 16, 4), gray(20, 16, 5)
    t.ifd(20, 16, f0, 1)
    t.ifd(20, 16, f1, 1)
    (OUT / "synthetic.scn").write_bytes(bytes(t.data))
    fluo = bytes(v for pair in zip(f0, f1) for v in pair)
    truth = [("0/0", b0), ("0/1", b1), ("1/0", fluo), ("macro", macro)]
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

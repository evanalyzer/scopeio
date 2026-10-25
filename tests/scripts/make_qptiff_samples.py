#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Akoya QPTIFF file.

Layout from the Akoya QPTIFF specification (doc/perkinelmer-qptiff/,
appendix A): Software "PerkinElmer-QPI", an XML ImageDescription per
directory with ImageType, Name and Color; two 16-bit fluorescence bands at
full resolution (tiled), an RGB thumbnail, the two bands at half resolution,
an RGB overview and label. Uncompressed. Also writes truth.tsv with the
CRC-32 of each level and associated image. Output in tests/data/qptiff/ is
committed.

Usage: python3 -I tests/scripts/make_qptiff_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "qptiff"


def gray16(w, h, seed):
    return b"".join(struct.pack("<H", (x * 97 + y * 31 + seed * 4001) % 65536) for y in range(h) for x in range(w))


def rgb(w, h, seed):
    return bytes(((x * 3 + y * 5 + k * 61 + seed * 17) % 256) for y in range(h) for x in range(w) for k in range(3))


def tiles(px, w, h, tw, th, bpp):
    out = []
    for ty in range(0, h, th):
        for tx in range(0, w, tw):
            t = bytearray()
            for y in range(ty, ty + th):
                for x in range(tx, tx + tw):
                    t += px[(y * w + x) * bpp:(y * w + x + 1) * bpp] if x < w and y < h else bytes(bpp)
            out.append(bytes(t))
    return out


def desc(image_type, name=None, color=None):
    extra = (f"<Name>{name}</Name>" if name else "") + (f"<Color>{color}</Color>" if color else "")
    return (f'<?xml version="1.0" encoding="utf-8"?><PerkinElmer-QPI-ImageDescription><DescriptionVersion>5</DescriptionVersion>'
            f"<AcquisitionSoftware>Synthetic</AcquisitionSoftware><ImageType>{image_type}</ImageType>"
            f"<ExposureTime>2500</ExposureTime>{extra}<Objective>20x</Objective></PerkinElmer-QPI-ImageDescription>")


class Tiff:
    def __init__(self):
        self.data = bytearray(b"II*\0\0\0\0\0")
        self.prev = 4

    def ifd(self, w, h, chunks, spp, bits, description, subfile=0, tile=None):
        offsets = []
        for c in chunks:
            offsets.append(len(self.data))
            self.data += c
        extra = bytearray()
        base = len(self.data)

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            if len(extra) % 2:
                extra.append(0)
            return pos

        def arr(vals):
            return ext(struct.pack(f"<{len(vals)}I", *vals)) if len(vals) > 1 else vals[0]

        d = description.encode() + b"\0"
        sw = b"PerkinElmer-QPI\0"
        e = [(254, 4, 1, subfile), (256, 4, 1, w), (257, 4, 1, h),
             (258, 3, spp, ext(struct.pack(f"<{spp}H", *[bits] * spp)) if spp > 2 else bits), (259, 3, 1, 1),
             (262, 3, 1, 2 if spp == 3 else 1), (270, 2, len(d), ext(d)), (277, 3, 1, spp),
             (282, 5, 1, ext(struct.pack("<II", 20000, 1))), (283, 5, 1, ext(struct.pack("<II", 20000, 1))),
             (284, 3, 1, 1), (296, 3, 1, 3), (305, 2, len(sw), ext(sw))]
        if tile:
            e += [(322, 3, 1, tile), (323, 3, 1, tile), (324, 4, len(offsets), arr(offsets)), (325, 4, len(chunks), arr([len(c) for c in chunks]))]
        else:
            e += [(273, 4, 1, offsets[0]), (278, 4, 1, h), (279, 4, 1, len(chunks[0]))]
        self.data += extra
        if len(self.data) % 2:
            self.data.append(0)
        at = len(self.data)
        struct.pack_into("<I", self.data, self.prev, at)
        self.data += struct.pack("<H", len(e))
        for tag, typ, count, value in sorted(e):
            v = struct.pack("<HH", value, 0) if typ == 3 and count == 1 else struct.pack("<I", value)
            self.data += struct.pack("<HHI", tag, typ, count) + v
        self.prev = len(self.data)
        self.data += b"\0\0\0\0"


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    t = Tiff()
    truth = []
    W, H = 40, 24
    bands = [("DAPI", "0,0,255"), ("FITC", "0,255,0")]
    full = [gray16(W, H, i) for i in range(2)]
    for (name, color), px in zip(bands, full):
        t.ifd(W, H, tiles(px, W, H, 16, 16, 2), 1, 16, desc("FullResolution", name, color), tile=16)
    thumb = rgb(10, 6, 1)
    t.ifd(10, 6, [thumb], 3, 8, desc("Thumbnail"), subfile=1)
    half = [gray16(20, 12, 10 + i) for i in range(2)]
    for (name, color), px in zip(bands, half):
        t.ifd(20, 12, tiles(px, 20, 12, 16, 16, 2), 1, 16, desc("ReducedResolution", name, color), subfile=1, tile=16)
    macro = rgb(16, 6, 4)
    t.ifd(16, 6, [macro], 3, 8, desc("Overview"), subfile=1)
    label = rgb(8, 8, 3)
    t.ifd(8, 8, [label], 3, 8, desc("Label"), subfile=1)
    (OUT / "synthetic.qptiff").write_bytes(bytes(t.data))
    # decode() layout: channels interleaved per pixel.
    def interleave(planes, w, h):
        return b"".join(planes[c][2 * i:2 * i + 2] for i in range(w * h) for c in range(len(planes)))
    truth = [("level0", interleave(full, W, H)), ("level1", interleave(half, 20, 12)), ("thumbnail", thumb), ("macro", macro), ("label", label)]
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

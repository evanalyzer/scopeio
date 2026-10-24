#!/usr/bin/env python3
"""Writes a small synthetic Aperio SVS file.

Layout from the Aperio "Digital Slides and Third-Party Data Interchange"
document and the openslide.org Aperio page (doc/aperio-svs/): tiled
full-resolution IFD with an `Aperio` ImageDescription, a stripped
thumbnail, a tiled pyramid level, a label (NewSubfileType 1) and a macro
(NewSubfileType 9). Uncompressed RGB. Also writes truth.tsv with the
CRC-32 of each level and associated image. Output in tests/data/svs/ is
committed.

Usage: python3 -I tests/scripts/make_svs_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "svs"


def rgb(w, h, seed):
    return bytes(((x * 3 + y * 5 + k * 61 + seed * 17) % 256) for y in range(h) for x in range(w) for k in range(3))


def tiles(pixels, w, h, tw, th):
    out = []
    for ty in range(0, h, th):
        for tx in range(0, w, tw):
            t = bytearray()
            for y in range(ty, ty + th):
                for x in range(tx, tx + tw):
                    t += pixels[(y * w + x) * 3:(y * w + x) * 3 + 3] if x < w and y < h else bytes(3)
            out.append(bytes(t))
    return out


class Tiff:
    def __init__(self):
        self.data = bytearray(b"II*\0\0\0\0\0")
        self.prev = 4

    def ifd(self, w, h, chunks, tiled, desc=None, subfile=0, tile=(16, 16)):
        offsets = []
        for c in chunks:
            offsets.append(len(self.data))
            self.data += c
        extra = bytearray()
        base = len(self.data)
        entries = []

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            if len(extra) % 2:
                extra.append(0)
            return pos

        def add(tag, typ, count, value):
            entries.append((tag, typ, count, value))

        add(254, 4, 1, subfile)
        add(256, 4, 1, w)
        add(257, 4, 1, h)
        add(258, 3, 3, ext(struct.pack("<3H", 8, 8, 8)))
        add(259, 3, 1, 1)
        add(262, 3, 1, 2)
        if desc:
            d = desc.encode() + b"\0"
            add(270, 2, len(d), ext(d))
        add(277, 3, 1, 3)
        add(284, 3, 1, 1)
        if tiled:
            add(322, 3, 1, tile[0])
            add(323, 3, 1, tile[1])
            add(324, 4, len(offsets), ext(struct.pack(f"<{len(offsets)}I", *offsets)) if len(offsets) > 1 else offsets[0])
            add(325, 4, len(chunks), ext(struct.pack(f"<{len(chunks)}I", *map(len, chunks))) if len(chunks) > 1 else len(chunks[0]))
        else:
            add(273, 4, 1, offsets[0])
            add(278, 4, 1, h)
            add(279, 4, 1, len(chunks[0]))
        self.data += extra
        if len(self.data) % 2:
            self.data.append(0)
        at = len(self.data)
        struct.pack_into("<I", self.data, self.prev, at)
        self.data += struct.pack("<H", len(entries))
        for tag, typ, count, value in sorted(entries):
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
    desc = ("Aperio Image Library vSynthetic\r\n40x24 [0,0 40x24] (16x16) RAW|AppMag = 40|Date = 03/14/21|"
            "Time = 08:30:00|MPP = 0.2500|Filename = synthetic")
    full = rgb(W, H, 0)
    t.ifd(W, H, tiles(full, W, H, 16, 16), True, desc)
    truth.append(("level0", full))
    thumb = rgb(10, 6, 1)
    t.ifd(10, 6, [thumb], False, "Aperio Image Library vSynthetic\r\n40x24 -> 10x6")
    truth.append(("thumbnail", thumb))
    half = rgb(20, 12, 2)
    t.ifd(20, 12, tiles(half, 20, 12, 16, 16), True, "Aperio Image Library vSynthetic\r\n40x24 -> 20x12")
    truth.append(("level1", half))
    label = rgb(8, 8, 3)
    t.ifd(8, 8, [label], False, "label 8x8", subfile=1)
    truth.append(("label", label))
    macro = rgb(16, 6, 4)
    t.ifd(16, 6, [macro], False, "macro 16x6", subfile=9)
    truth.append(("macro", macro))
    (OUT / "synthetic.svs").write_bytes(bytes(t.data))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

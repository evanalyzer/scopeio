#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Philips TIFF file.

Layout from the openslide.org Philips page (doc/philips-tiff/) and own
analysis of the OpenSlide Philips test slides: a BigTIFF with Software
"Philips DP v1.1" whose first ImageDescription is a DPUfsImport XML
document. Three tiled RGB levels (16 x 16 tiles, uncompressed): 64 x 48,
then 32 x 24 and 16 x 12, stored padded to whole tiles (32 x 32 and
16 x 16) as Philips does; the XML gives each level's pixel spacing (rows
0.0005 mm, columns 0.00025 mm at full resolution). Tile (1, 1) of level 0
is not stored (offset and byte count 0) and reads as white. The label is a
base64 JPEG in the XML (a solid colour, from ImageMagick) and also a
stripped TIFF directory "Label" with other pixels (the XML image wins); the
macro is only a stripped TIFF directory "Macro". Every level carries the
ImageDepth and TileDepth tags with the value 0 that Philips DP v1.1 writes.

Also writes truth.tsv with the CRC-32 of each level (unpadded, missing tile
white) and of the macro. Output in tests/data/philips/ is committed.

Usage: python3 -I tests/scripts/make_philips_samples.py
"""

import base64
import shutil
import struct
import subprocess
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "philips"
TILE = 16
MISSING = (0, 1, 1)  # level, tile x, tile y


def rgb(w, h, seed):
    return bytes(((x * 3 + y * 5 + k * 61 + seed * 17) % 256) for y in range(h) for x in range(w) for k in range(3))


def tiles(px, w, h, pw, ph):
    """Tiles of a w x h image padded to pw x ph (padding is black)."""
    out = []
    for ty in range(0, ph, TILE):
        for tx in range(0, pw, TILE):
            b = bytearray()
            for y in range(ty, ty + TILE):
                for x in range(tx, tx + TILE):
                    b += px[(y * w + x) * 3:(y * w + x + 1) * 3] if x < w and y < h else bytes(3)
            out.append(bytes(b))
    return out


class BigTiff:
    def __init__(self):
        self.data = bytearray(b"II+\0\x08\0\0\0" + bytes(8))
        self.prev = 8

    def _write(self, entries, extra):
        self.data += extra
        at = len(self.data)
        struct.pack_into("<Q", self.data, self.prev, at)
        self.data += struct.pack("<Q", len(entries))
        for tag, typ, count, value in sorted(entries):
            self.data += struct.pack("<HHQQ", tag, typ, count, value)
        self.prev = len(self.data)
        self.data += bytes(8)

    def _common(self, w, h, description, ext):
        e = [(256, 4, 1, w), (257, 4, 1, h), (258, 3, 3, struct.unpack("<Q", struct.pack("<3H", 8, 8, 8) + bytes(2))[0]),
             (259, 3, 1, 1), (262, 3, 1, 2), (277, 3, 1, 3), (284, 3, 1, 1)]
        sw = b"Philips DP v1.1\0"
        e.append((305, 2, len(sw), ext(sw)))
        if description:
            # Values of up to 8 bytes are stored in the entry itself.
            d = description.encode() + b"\0"
            e.append((270, 2, len(d), struct.unpack("<Q", d.ljust(8, b"\0"))[0] if len(d) <= 8 else ext(d)))
        return e

    def tiled(self, w, h, pw, ph, px, description=None, missing=()):
        chunks = tiles(px, w, h, pw, ph)
        offsets, counts = [], []
        for i, c in enumerate(chunks):
            if i in missing:
                offsets.append(0)
                counts.append(0)
                continue
            offsets.append(len(self.data))
            counts.append(len(c))
            self.data += c
        extra = bytearray()
        base = len(self.data)

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            return pos

        e = self._common(pw, ph, description, ext)
        e += [(322, 3, 1, TILE), (323, 3, 1, TILE),
              # One LONG8 fits in the entry itself.
              (324, 16, len(offsets), offsets[0] if len(offsets) == 1 else ext(struct.pack(f"<{len(offsets)}Q", *offsets))),
              (325, 16, len(counts), counts[0] if len(counts) == 1 else ext(struct.pack(f"<{len(counts)}Q", *counts))),
              (32997, 4, 1, 0), (32998, 4, 1, 0)]
        self._write(e, extra)

    def stripped(self, w, h, px, description):
        at = len(self.data)
        self.data += px
        extra = bytearray()
        base = len(self.data)

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            return pos

        e = self._common(w, h, description, ext)
        e += [(273, 16, 1, at), (278, 4, 1, h), (279, 16, 1, len(px))]
        self._write(e, extra)


def attr(name, value, kind="IString"):
    return f'<Attribute Name="{name}" Group="0x0000" Element="0x0000" PMSVR="{kind}">{value}</Attribute>'


def representation(n, row, col, w, h):
    return ('<DataObject ObjectType="PixelDataRepresentation">'
            + attr("DICOM_PIXEL_SPACING", f"&quot;{row}&quot; &quot;{col}&quot;", "IDoubleArray")
            + attr("PIIM_PIXEL_DATA_REPRESENTATION_COLUMNS", w, "IUInt32")
            + attr("PIIM_PIXEL_DATA_REPRESENTATION_NUMBER", n, "IUInt16")
            + attr("PIIM_PIXEL_DATA_REPRESENTATION_ROWS", h, "IUInt32") + "</DataObject>")


def xml(label_jpeg):
    reps = "".join(representation(n, 0.0005 * 2 ** n, 0.00025 * 2 ** n, pw, ph)
                   for n, (pw, ph) in enumerate([(64, 48), (32, 32), (16, 16)]))
    wsi = ('<DataObject ObjectType="DPScannedImage">' + attr("PIM_DP_IMAGE_TYPE", "WSI")
           + attr("DICOM_PIXEL_SPACING", "&quot;0.0005&quot; &quot;0.00025&quot;", "IDoubleArray")
           + attr("PIIM_PIXEL_DATA_REPRESENTATION_SEQUENCE", f"<Array>{reps}</Array>", "IDataObjectArray")
           + attr("PIM_DP_IMAGE_COLUMNS", 64, "IUInt32") + attr("PIM_DP_IMAGE_ROWS", 48, "IUInt32") + "</DataObject>")
    label = ('<DataObject ObjectType="DPScannedImage">' + attr("PIM_DP_IMAGE_TYPE", "LABELIMAGE")
             + attr("PIM_DP_IMAGE_DATA", base64.b64encode(label_jpeg).decode()) + "</DataObject>")
    return ('<?xml version="1.0" encoding="UTF-8" ?>\n<DataObject ObjectType="DPUfsImport">'
            + attr("DICOM_MANUFACTURER", "Synthetic")
            + attr("DICOM_ACQUISITION_DATETIME", "20240229131415.000000")
            + attr("PIM_DP_SCANNED_IMAGES", f"<Array>{wsi}{label}</Array>", "IDataObjectArray")
            + attr("PIM_DP_UFS_BARCODE", base64.b64encode(b"SYN-42").decode())
            + "</DataObject>\n")


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    label_jpeg = subprocess.run(["magick", "-size", "24x12", "xc:rgb(200,100,50)", "-quality", "100", "jpeg:-"],
                                capture_output=True, check=True).stdout
    t = BigTiff()
    sizes = [(64, 48, 64, 48), (32, 24, 32, 32), (16, 12, 16, 16)]
    levels = []
    for n, (w, h, pw, ph) in enumerate(sizes):
        px = rgb(w, h, n + 1)
        across = pw // TILE
        missing = {MISSING[2] * across + MISSING[1]} if n == MISSING[0] else set()
        t.tiled(w, h, pw, ph, px, xml(label_jpeg) if n == 0 else f"level={n} mag=1 quality=100", missing)
        if n == MISSING[0]:
            px = bytearray(px)
            for y in range(MISSING[2] * TILE, (MISSING[2] + 1) * TILE):
                for x in range(MISSING[1] * TILE, (MISSING[1] + 1) * TILE):
                    px[(y * w + x) * 3:(y * w + x + 1) * 3] = b"\xff\xff\xff"
        levels.append(bytes(px))
    t.stripped(8, 4, rgb(8, 4, 7), "Label")
    macro = rgb(12, 6, 8)
    t.stripped(12, 6, macro, "Macro")
    (OUT / "synthetic.tiff").write_bytes(bytes(t.data))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for n, px in enumerate(levels):
            f.write(f"0/{n}\t{zlib.crc32(px):08x}\n")
        f.write(f"macro\t{zlib.crc32(macro):08x}\n")


if __name__ == "__main__":
    main()

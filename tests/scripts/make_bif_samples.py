#!/usr/bin/env python3
"""Writes a small synthetic Ventana BIF file.

Layout from the Roche BIF white paper (doc/ventana-bif/): BigTIFF with
IFD 0 `Label_Image` (XMP with the iScan node), IFD 1 `Probability_Image`,
IFD 2 the scan (`level=0 ...`, XMP EncodeInfo Ver 2 with TileJointInfo
overlaps, tiles numbered along a serpentine path from the lower left; Tile2
covers Tile1) and IFD 3 the next level. As in the sample files, tiles are
stored on the TIFF grid of the padded image and the lower level was made
from the unstitched grid. Uncompressed RGB. Also writes truth.tsv with the
CRC-32 of both levels, composed here by that model, and of the label.
Output in tests/data/bif/ is committed.

Usage: python3 -I tests/scripts/make_bif_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "bif"
TW = TH = 16
ROWS, COLS = 2, 3
ACROSS = 4  # TIFF tile grid of the padded image
WHITE = 250


def tile_px(r, c, seed=0):
    return bytes(((x * 5 + y * 3 + k * 50 + (r * COLS + c) * 37 + seed) % 200 + 20) for y in range(TH) for x in range(TW) for k in range(3))


class BigTiff:
    def __init__(self):
        self.data = bytearray(b"II+\0\x08\0\0\0" + bytes(8))
        self.prev = 8

    def ifd(self, w, h, chunks, description, xmp=None, tiled=True, spp=3):
        offsets = []
        for c in chunks:
            offsets.append(len(self.data) if c else 0)
            self.data += c
        extra = bytearray()
        base = len(self.data)

        def ext(b):
            pos = base + len(extra)
            extra.extend(b)
            return pos

        def inline_or(vals, fmt, typ):
            return vals[0] if len(vals) == 1 else ext(struct.pack(f"<{len(vals)}{fmt}", *vals))

        d = description.encode() + b"\0"
        bits = struct.unpack("<Q", struct.pack(f"<{spp}H", *[8] * spp).ljust(8, b"\0"))[0]
        e = [(256, 4, 1, w), (257, 4, 1, h), (258, 3, spp, bits), (259, 3, 1, 1), (262, 3, 1, 2 if spp == 3 else 1),
             (270, 2, len(d), ext(d) if len(d) > 8 else struct.unpack("<Q", d.ljust(8, b"\0"))[0]), (277, 3, 1, spp), (284, 3, 1, 1)]
        if tiled:
            e += [(322, 3, 1, TW), (323, 3, 1, TH), (324, 16, len(offsets), inline_or(offsets, "Q", 16)),
                  (325, 16, len(chunks), inline_or([len(c) for c in chunks], "Q", 16))]
        else:
            e += [(273, 16, 1, offsets[0]), (278, 4, 1, h), (279, 16, 1, len(chunks[0]))]
        if xmp:
            x = xmp.encode()
            e.append((700, 1, len(x), ext(x)))
        self.data += extra
        at = len(self.data)
        struct.pack_into("<Q", self.data, self.prev, at)
        self.data += struct.pack("<Q", len(e))
        for tag, typ, count, value in sorted(e):
            self.data += struct.pack("<HHQQ", tag, typ, count, value)
        self.prev = len(self.data)
        self.data += bytes(8)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    # Overlaps by (image row, left column) and the covering side.
    overlap = {(0, 0): 4, (0, 1): 0, (1, 0): 2, (1, 1): 6}
    right_on_top = {(0, 0): True, (0, 1): True, (1, 0): False, (1, 1): True}
    # Serpentine numbering: physical row 0 = image row ROWS-1, left to right; next row right to left.
    def number(r, c):
        pr = ROWS - 1 - r
        k = c if pr % 2 == 0 else COLS - 1 - c
        return pr * COLS + k + 1
    joints = []
    for (r, c), o in overlap.items():
        left, right = number(r, c), number(r, c + 1)
        t1, t2 = (left, right) if right_on_top[(r, c)] else (right, left)
        joints.append(f'<TileJointInfo FlagJoined="1" Confidence="100" Direction="LEFT" Tile1="{t1}" Tile2="{t2}" OverlapX="{o}" OverlapY="0"/>')
    for r in range(ROWS - 1):
        for c in range(COLS):
            joints.append(f'<TileJointInfo FlagJoined="1" Confidence="100" Direction="UP" Tile1="{number(r + 1, c)}" Tile2="{number(r, c)}" OverlapX="0" OverlapY="0"/>')
    frames = "".join(f'<Frame XY="{c},{r}" Z="0"/>' for r in range(ROWS) for c in range(COLS))
    encode = (f'<?xml version="1.0" encoding="UTF-8"?><EncodeInfo Ver="2"><SlideInfo><SlideStitchInfo>'
              f'<ImageInfo AOIScanned="1" AOIIndex="0" NumRows="{ROWS}" NumCols="{COLS}" Width="{TW}" Height="{TH}">{"".join(joints)}</ImageInfo>'
              f'<FrameInfo AOIScanned="1" AOIIndex="0">{frames}</FrameInfo></SlideStitchInfo></SlideInfo>'
              f'<AoiOrigin><AOI0 OriginX="0" OriginY="0"/></AoiOrigin></EncodeInfo>')
    iscan = (f'<?xml version="1.0" encoding="UTF-8"?><Metadata><iScan Mode="brightfield" Magnification="40" ScanRes="0.25" '
             f'ScannerModel="VENTANA DP 200" Z-layers="1" ScanWhitePoint="{WHITE}"/></Metadata>')
    t = BigTiff()
    label = bytes([(i * 7) % 256 for i in range(8 * 12 * 3)])
    t.ifd(8, 12, [label], "Label_Image", iscan, tiled=False)
    prob = bytes([(i * 3) % 256 for i in range(8 * 12)])
    t.ifd(8, 12, [prob], "Probability_Image", "<Metadata/>", tiled=False, spp=1)
    # Level 0: TIFF grid ACROSS x ROWS; column 3 is padding, tile (1, 2) unscanned.
    tiles0 = {}
    chunks = []
    for r in range(ROWS):
        for c in range(ACROSS):
            if c < COLS and (r, c) != (1, 2):
                tiles0[(r, c)] = tile_px(r, c)
                chunks.append(tiles0[(r, c)])
            else:
                chunks.append(b"")
    t.ifd(ACROSS * TW, ROWS * TH, chunks, "level=0 mag=40 quality=95", encode)
    # Level 1 from the unstitched grid: halve each 16 x 16 tile of the grid.
    grid_w = ACROSS * TW
    unstitched = bytearray(bytes([WHITE]) * (grid_w * ROWS * TH * 3))
    for (r, c), px in tiles0.items():
        for y in range(TH):
            for x in range(TW):
                i = ((r * TH + y) * grid_w + c * TW + x) * 3
                unstitched[i:i + 3] = px[(y * TW + x) * 3:(y * TW + x) * 3 + 3]
    l1w, l1h = grid_w // 2, ROWS * TH // 2
    level1 = bytearray()
    for y in range(l1h):
        for x in range(l1w):
            i = ((2 * y) * grid_w + 2 * x) * 3
            level1 += unstitched[i:i + 3]
    # Stored as one 16 x 16 tile grid (2 x 1 tiles).
    l1_tiles = []
    for tx in range(0, l1w, TW):
        b = bytearray()
        for y in range(TH):
            for x in range(tx, tx + TW):
                b += level1[(y * l1w + x) * 3:(y * l1w + x) * 3 + 3] if y < l1h else bytes(3)
        l1_tiles.append(bytes(b))
    t.ifd(l1w, l1h, l1_tiles, "level=1 mag=20 quality=95")
    (OUT / "synthetic.bif").write_bytes(bytes(t.data))

    # Truth: stitched level 0 and the shifted level 1.
    spans = {}
    width = None
    for r in range(ROWS):
        x = 0
        for c in range(COLS):
            left_hidden = overlap[(r, c - 1)] if c > 0 and not right_on_top[(r, c - 1)] else 0
            right_hidden = overlap[(r, c)] if c + 1 < COLS and right_on_top[(r, c)] else 0
            spans[(r, c)] = (x + left_hidden, x + TW - right_hidden, left_hidden)
            x = x + TW - (overlap[(r, c)] if c + 1 < COLS else 0)
        width = x if width is None else min(width, x)
    h0 = ROWS * TH
    l0 = bytearray(width * h0 * 3)
    for (r, c), (x0, x1, sx) in spans.items():
        for y in range(TH):
            for x in range(x0, min(x1, width)):
                i = ((r * TH + y) * width + x) * 3
                px = tiles0.get((r, c))
                l0[i:i + 3] = px[(y * TW + sx + x - x0) * 3:(y * TW + sx + x - x0) * 3 + 3] if px else bytes([WHITE] * 3)
    w1, h1 = width // 2, h0 // 2
    l1 = bytearray(bytes([WHITE]) * (w1 * h1 * 3))
    for (r, c), (x0, x1, sx) in spans.items():
        ry0, ry1 = -(-(r * TH) // 2), -(-((r + 1) * TH) // 2)
        cx0, cx1 = -(-x0 // 2), min(-(-x1 // 2), w1)
        stored = (c * TW + sx) // 2
        for y in range(ry0, min(ry1, h1)):
            for x in range(cx0, cx1):
                s = stored + x - cx0
                l1[(y * w1 + x) * 3:(y * w1 + x) * 3 + 3] = level1[(y * l1w + s) * 3:(y * l1w + s) * 3 + 3]
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\twidth\tcrc32\n")
        f.write(f"level0\t{width}\t{zlib.crc32(bytes(l0)):08x}\n")
        f.write(f"level1\t{w1}\t{zlib.crc32(bytes(l1)):08x}\n")
        f.write(f"macro\t8\t{zlib.crc32(label):08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Writes a small synthetic Ventana BIF file.

Layout from the Roche BIF white paper (doc/ventana-bif/): BigTIFF with
IFD 0 `Label_Image` (XMP with the iScan node), IFD 1 `Probability_Image`,
IFD 2 the scan (`level=0 ...`, XMP EncodeInfo Ver 2 with TileJointInfo
overlaps, tiles numbered along a serpentine path from the lower left; Tile2
covers Tile1) and IFD 3 the next level. As in the sample files, tiles are
stored on the TIFF grid of the padded image and the lower level was made
from the unstitched grid. Uncompressed RGB. volumetric.bif adds three focal
planes (ImageDepth tag 32997): the tile tables hold the nominal plane, then
the near one, then the far one. iscan.bif has the layout of iScan scanners
(no Frame nodes): two scanned AOIs and an unscanned one, stage positions
(Pos-X/Pos-Y, y pointing up), AoiOrigin in the TIFF grid and tiles placed on
a lattice whose pitch is the tile size less the mean joined overlap, later
tiles on top. Also writes truth.tsv with the CRC-32 of
both levels (focal planes near to far), composed here by that model, and of
the label.
Output in tests/data/bif/ is committed.

Usage: python3 -I tests/scripts/make_bif_samples.py
"""

import math
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

    def ifd(self, w, h, chunks, description, xmp=None, tiled=True, spp=3, depth=1):
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
        if depth > 1:
            e.append((32997, 4, 1, depth))
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


def storage_block(z, depth):
    """Block of focal plane z (near to far): nominal first, then near, then far."""
    nominal = (depth - 1) // 2
    return nominal - z if z < nominal else 0 if z == nominal else z


def build(name, depth):
    """Writes `name`; returns its truth rows (image, width, crc32)."""
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
             f'ScannerModel="VENTANA DP 200" Z-layers="{depth}" Z-spacing="1.5" ScanWhitePoint="{WHITE}"/></Metadata>')
    t = BigTiff()
    label = bytes([(i * 7) % 256 for i in range(8 * 12 * 3)])
    t.ifd(8, 12, [label], "Label_Image", iscan, tiled=False)
    prob = bytes([(i * 3) % 256 for i in range(8 * 12)])
    t.ifd(8, 12, [prob], "Probability_Image", "<Metadata/>", tiled=False, spp=1)
    # Level 0: TIFF grid ACROSS x ROWS; column 3 is padding, tile (1, 2) unscanned.
    # One block of tiles per focal plane, in storage order.
    blocks0, blocks1 = [], []
    chunks, l1_tiles = [], []
    grid_w = ACROSS * TW
    l1w, l1h = grid_w // 2, ROWS * TH // 2
    for b in range(depth):
        tiles0 = {}
        for r in range(ROWS):
            for c in range(ACROSS):
                if c < COLS and (r, c) != (1, 2):
                    tiles0[(r, c)] = tile_px(r, c, seed=b * 61)
                    chunks.append(tiles0[(r, c)])
                else:
                    chunks.append(b"")
        # Level 1 from the unstitched grid: halve each 16 x 16 tile of the grid.
        unstitched = bytearray(bytes([WHITE]) * (grid_w * ROWS * TH * 3))
        for (r, c), px in tiles0.items():
            for y in range(TH):
                for x in range(TW):
                    i = ((r * TH + y) * grid_w + c * TW + x) * 3
                    unstitched[i:i + 3] = px[(y * TW + x) * 3:(y * TW + x) * 3 + 3]
        level1 = bytearray()
        for y in range(l1h):
            for x in range(l1w):
                i = ((2 * y) * grid_w + 2 * x) * 3
                level1 += unstitched[i:i + 3]
        # Stored as one 16 x 16 tile grid (2 x 1 tiles).
        for tx in range(0, l1w, TW):
            row = bytearray()
            for y in range(TH):
                for x in range(tx, tx + TW):
                    row += level1[(y * l1w + x) * 3:(y * l1w + x) * 3 + 3] if y < l1h else bytes(3)
            l1_tiles.append(bytes(row))
        blocks0.append(tiles0)
        blocks1.append(level1)
    t.ifd(ACROSS * TW, ROWS * TH, chunks, "level=0 mag=40 quality=95", encode, depth=depth)
    t.ifd(l1w, l1h, l1_tiles, "level=1 mag=20 quality=95", depth=depth)
    (OUT / name).write_bytes(bytes(t.data))

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
    w1, h1 = width // 2, h0 // 2
    all0, all1 = bytearray(), bytearray()
    for z in range(depth):
        tiles0, level1 = blocks0[storage_block(z, depth)], blocks1[storage_block(z, depth)]
        l0 = bytearray(width * h0 * 3)
        for (r, c), (x0, x1, sx) in spans.items():
            for y in range(TH):
                for x in range(x0, min(x1, width)):
                    i = ((r * TH + y) * width + x) * 3
                    px = tiles0.get((r, c))
                    l0[i:i + 3] = px[(y * TW + sx + x - x0) * 3:(y * TW + sx + x - x0) * 3 + 3] if px else bytes([WHITE] * 3)
        l1 = bytearray(bytes([WHITE]) * (w1 * h1 * 3))
        for (r, c), (x0, x1, sx) in spans.items():
            ry0, ry1 = -(-(r * TH) // 2), -(-((r + 1) * TH) // 2)
            cx0, cx1 = -(-x0 // 2), min(-(-x1 // 2), w1)
            stored = (c * TW + sx) // 2
            for y in range(ry0, min(ry1, h1)):
                for x in range(cx0, cx1):
                    s = stored + x - cx0
                    l1[(y * w1 + x) * 3:(y * w1 + x) * 3 + 3] = level1[(y * l1w + s) * 3:(y * l1w + s) * 3 + 3]
        all0 += l0
        all1 += l1
    return [(name, "level0", width, f"{zlib.crc32(bytes(all0)):08x}"), (name, "level1", w1, f"{zlib.crc32(bytes(all1)):08x}"),
            (name, "macro", 8, f"{zlib.crc32(label):08x}")]


def rnd(v):
    """Rounds half away from zero, as Rust's f64::round."""
    return math.floor(v + 0.5)


def build_iscan(name):
    """iScan layout: AOIs on a lattice; returns truth rows."""
    across, down = 4, 3  # TIFF grid of 16 x 16 tiles
    # AOI: (rows, cols, Pos-X, Pos-Y, grid origin in tiles, scanned)
    aois = [(2, 3, 0, 40, (0, 0), 1), (1, 1, 60, 70, (3, 0), 0), (1, 2, 30, 10, (0, 2), 1)]
    joints = {0: ([4, 3, None, 2], [2, 2, 3]), 2: ([5], [])}  # RIGHT OverlapX, UP OverlapY; None = not joined
    grid = {}
    for k, (rows, cols, _, _, (ox, oy), scanned) in enumerate(aois):
        for r in range(rows):
            for c in range(cols):
                grid[(oy + r, ox + c)] = tile_px(r, c, seed=k * 97) if scanned else bytes([WHITE] * (TW * TH * 3))
    chunks = [grid.get((r, c), b"") for r in range(down) for c in range(across)]
    infos, origins = "", ""
    for k, (rows, cols, x, y, (ox, oy), scanned) in enumerate(aois):
        right, up = joints.get(k, ([], []))
        js = "".join(f"<TileJointInfo FlagJoined='{0 if o is None else 1}' Confidence='90' Direction='RIGHT' Tile1='{i + 1}' Tile2='{i + 2}' "
                     f"OverlapX='{o or 0}' OverlapY='1'/>" for i, o in enumerate(right))
        js += "".join(f"<TileJointInfo FlagJoined='1' Confidence='90' Direction='UP' Tile1='{i + 1}' Tile2='{2 * cols - i}' "
                      f"OverlapX='-1' OverlapY='{o}'/>" for i, o in enumerate(up))
        infos += (f'<ImageInfo AOIScanned="{scanned}" Width="{TW}" Height="{TH}" NumRows="{rows}" NumCols="{cols}" '
                  f'Pos-X="{x}" Pos-Y="{y}">{js}</ImageInfo>')
        origins += f'<AOI{k} OriginX="{ox * TW}" OriginY="{oy * TH}"/>'
    encode = (f"<?xml version='1.0'?><EncodeInfo Ver='2'><SlideInfo><iScan Magnification='40' ScanRes='0.25' Z-layers='1'/>"
              f"<SlideStitchInfo>{infos}</SlideStitchInfo><AoiOrigin>{origins}</AoiOrigin></SlideInfo></EncodeInfo>")
    iscan = (f'<?xml version="1.0" encoding="UTF-8"?><Metadata><iScan Magnification="40" ScanRes="0.25" '
             f'Z-layers="1" ScanWhitePoint="{WHITE}"/></Metadata>')
    t = BigTiff()
    label = bytes([(i * 5) % 256 for i in range(8 * 12 * 3)])
    t.ifd(8, 12, [label], "Label Image", iscan, tiled=False)
    gw, gh = across * TW, down * TH
    t.ifd(gw, gh, chunks, "level=0 mag=40 quality=90", encode)
    canvas_grid = bytearray(bytes([WHITE]) * (gw * gh * 3))
    for (r, c), px in grid.items():
        for y in range(TH):
            i = ((r * TH + y) * gw + c * TW) * 3
            canvas_grid[i:i + TW * 3] = px[y * TW * 3:(y + 1) * TW * 3]
    # Level 1 from the unstitched grid, one 16 x 16 tile grid (2 x 2 tiles).
    l1w, l1h = gw // 2, gh // 2
    level1 = bytearray()
    for y in range(l1h):
        for x in range(l1w):
            i = ((2 * y) * gw + 2 * x) * 3
            level1 += canvas_grid[i:i + 3]
    l1_tiles = []
    for ty in range(0, 32, TH):
        for tx in range(0, l1w, TW):
            b = bytearray()
            for y in range(ty, ty + TH):
                for x in range(tx, tx + TW):
                    b += level1[(y * l1w + x) * 3:(y * l1w + x) * 3 + 3] if y < l1h else bytes(3)
            l1_tiles.append(bytes(b))
    t.ifd(l1w, l1h, l1_tiles, "level=1 mag=20 quality=90")
    (OUT / name).write_bytes(bytes(t.data))

    # Truth by the lattice model.
    ox_all = [o for k, (r, _) in joints.items() if aois[k][5] for o in r if o is not None]
    oy_all = [o for k, (_, u) in joints.items() if aois[k][5] for o in u]
    px, py = TW - sum(ox_all) / len(ox_all), TH - sum(oy_all) / len(oy_all)
    scanned = [a for a in aois if a[5]]
    top = lambda a: a[3] + (a[0] - 1) * py + TH
    x0, y_top = min(a[2] for a in scanned), max(top(a) for a in scanned)
    y_bottom = min(a[3] for a in scanned)
    w0 = math.ceil(max(a[2] + (a[1] - 1) * px + TW for a in scanned) - x0)
    h0 = math.ceil(y_top - y_bottom)
    tiles = []
    for a in scanned:
        rows, cols, x, y, (ox, oy), _ = a
        for r in range(rows):
            for c in range(cols):
                tiles.append(((ox + c) * TW, (oy + r) * TH, x - x0 + c * px, y_top - top(a) + r * py))
    out = []
    for f, src, sw, sh in ((1, canvas_grid, gw, gh), (2, level1, l1w, l1h)):
        w, h = -(-w0 // f), -(-h0 // f)
        img = bytearray(bytes([WHITE]) * (w * h * 3))
        for gx, gy, x, y in tiles:
            dx, dy = rnd(x / f), rnd(y / f)
            sx, sy = rnd(gx / f), rnd(gy / f)
            tw, th = min(rnd((gx + TW) / f) - sx, sw - sx), min(rnd((gy + TH) / f) - sy, sh - sy)
            for yy in range(th):
                for xx in range(tw):
                    X, Y = dx + xx, dy + yy
                    if 0 <= X < w and 0 <= Y < h:
                        img[(Y * w + X) * 3:(Y * w + X) * 3 + 3] = src[((sy + yy) * sw + sx + xx) * 3:((sy + yy) * sw + sx + xx) * 3 + 3]
        out.append((name, f"level{f - 1}", w, f"{zlib.crc32(bytes(img)):08x}"))
    return out + [(name, "macro", 8, f"{zlib.crc32(label):08x}")]


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    rows = build("synthetic.bif", 1) + build("volumetric.bif", 3) + build_iscan("iscan.bif")
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\timage\twidth\tcrc32\n")
        for row in rows:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

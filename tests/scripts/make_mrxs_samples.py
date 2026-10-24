#!/usr/bin/env python3
"""Writes a small synthetic 3DHistech MIRAX slide.

Layout from the openslide.org MIRAX page (doc/3dhistech-mrxs/): Slidedat.ini,
Index.dat (version string, slide id, hierarchical and non-hierarchical
roots, record tables pointing to linked data pages), a data file and the
slide position buffer (one 9-byte entry per camera position). Two photos of
2 x 2 images that overlap; JPEG images written by ImageMagick (run as a
program) with constant 8 x 8 gray blocks, which decode exactly. Also writes
truth.tsv with the CRC-32 of each level, composed here by the documented
placement. Output in tests/data/mrxs/ is committed.

Usage: python3 -I tests/scripts/make_mrxs_samples.py   (needs `magick`)
"""

import shutil
import struct
import subprocess
import tempfile
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "mrxs"
SLIDE_ID = "0123456789ABCDEF0123456789ABCDEF"
TW, TH = 16, 8  # image size
DIV = 2
PHOTOS = [(0, 0), (28, 4)]  # camera positions in level-0 pixels; photo width 32
FILL = (255, 255, 255)


def jpeg(pixels, w, h):
    with tempfile.TemporaryDirectory() as tmp:
        raw, out = Path(tmp) / "in.rgb", Path(tmp) / "out.jpg"
        raw.write_bytes(pixels)
        subprocess.run(["magick", "-size", f"{w}x{h}", "-depth", "8", f"rgb:{raw}", "-type", "TrueColor", "-quality", "100",
                        "-sampling-factor", "1x1", str(out)], check=True)
        data = out.read_bytes()
        back = subprocess.run(["magick", str(out), "-depth", "8", "rgb:-"], check=True, capture_output=True).stdout
        assert back == pixels
        return data


def blocks(seed):
    return bytes(((x // 8 * 41 + y // 8 * 23 + seed * 29) % 180 + 40) for y in range(TH) for x in range(TW) for _ in range(3))


def compose(w, h, pieces):
    """Canvas of `w` x `h`; pieces (pixels, src_w, x, y) drawn in order."""
    canvas = bytearray(bytes(FILL) * (w * h))
    for px, sw, sh, x0, y0 in pieces:
        for y in range(sh):
            for x in range(sw):
                if 0 <= x0 + x < w and 0 <= y0 + y < h:
                    canvas[((y0 + y) * w + x0 + x) * 3:((y0 + y) * w + x0 + x) * 3 + 3] = px[(y * sw + x) * 3:(y * sw + x) * 3 + 3]
    return bytes(canvas)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "synthetic").mkdir(parents=True)
    nx, ny = DIV * len(PHOTOS), DIV
    data = bytearray(b"01.02" + SLIDE_ID.encode() + b"000" + bytes(256))
    level_items = [[], []]
    truth = []
    # Level 0: every image; level 1: one image per photo (2 x 2 images, halved).
    level0 = []
    for iy in range(ny):
        for ix in range(nx):
            px = blocks(iy * nx + ix)
            off = len(data)
            data += jpeg(px, TW, TH)
            level_items[0].append((iy * nx + ix, off, len(data) - off))
            p = PHOTOS[ix // DIV]
            level0.append((px, TW, TH, p[0] + (ix % DIV) * TW, p[1] + (iy % DIV) * TH))
    level1 = []
    for ph, p in enumerate(PHOTOS):
        px = blocks(100 + ph)
        off = len(data)
        data += jpeg(px, TW, TH)
        level_items[1].append((ph * DIV, off, len(data) - off))
        level1.append((px, TW, TH, p[0] // 2, p[1] // 2))
    label = bytes([90, 90, 90] * (TW * TH))
    off = len(data)
    data += jpeg(label, TW, TH)
    label_item = (off, len(data) - off)
    positions = b"".join(struct.pack("<Bii", 1, x, y) for x, y in PHOTOS)
    off = len(data)
    data += positions
    pos_item = (off, len(positions))
    (OUT / "synthetic" / "Data0000.dat").write_bytes(bytes(data))

    # Canvas: bounding box of level 0.
    x0 = min(p[3] for p in level0)
    y0 = min(p[4] for p in level0)
    w0 = max(p[3] + TW for p in level0) - x0
    h0 = max(p[4] + TH for p in level0) - y0
    truth.append(("level0", compose(w0, h0, [(a, b, c, x - x0, y - y0) for a, b, c, x, y in level0])))
    truth.append(("level1", compose(w0 // 2, h0 // 2, [(a, b, c, x - x0 // 2, y - y0 // 2) for a, b, c, x, y in level1])))
    truth.append(("label", label))

    # Index.dat: header, two roots, record tables, pages.
    idx = bytearray(b"01.02" + SLIDE_ID.encode())
    roots_at = len(idx)
    idx += bytes(8)
    hier_table = len(idx)
    idx += bytes(4 * 2)  # two zoom levels
    nonhier_table = len(idx)
    idx += bytes(4 * 2)  # barcode, position buffer

    def page_list(entries, ints):
        # An empty first page, then one page with the items.
        first = len(idx)
        idx.extend(struct.pack("<II", 0, 0))
        second = len(idx)
        idx.extend(struct.pack("<II", len(entries), 0))
        for e in entries:
            idx.extend(struct.pack(f"<{ints}i", *e))
        struct.pack_into("<I", idx, first + 4, second)
        return first

    for level, items in enumerate(level_items):
        struct.pack_into("<I", idx, hier_table + 4 * level, page_list([(i, o, n, 0) for i, o, n in items], 4))
    struct.pack_into("<I", idx, nonhier_table, page_list([(0, 0, label_item[0], label_item[1], 0)], 5))
    struct.pack_into("<I", idx, nonhier_table + 4, page_list([(0, 0, pos_item[0], pos_item[1], 0)], 5))
    struct.pack_into("<II", idx, roots_at, hier_table, nonhier_table)
    (OUT / "synthetic" / "Index.dat").write_bytes(bytes(idx))

    ini = f"""﻿[GENERAL]
SLIDE_VERSION = 01.03
SLIDE_NAME = synthetic
SLIDE_ID = {SLIDE_ID}
IMAGENUMBER_X = {nx}
IMAGENUMBER_Y = {ny}
CURRENT_SLIDE_VERSION = 1.9
SLIDE_CREATIONDATETIME = 01/02/2021 03:04:05
OBJECTIVE_MAGNIFICATION = 40
CameraImageDivisionsPerSide = {DIV}
[HIERARCHICAL]
INDEXFILE = Index.dat
HIER_COUNT = 1
NONHIER_COUNT = 2
HIER_0_NAME = Slide zoom level
HIER_0_COUNT = 2
HIER_0_VAL_0 = ZoomLevel_0
HIER_0_VAL_0_SECTION = LAYER_0_LEVEL_0_SECTION
HIER_0_VAL_1 = ZoomLevel_1
HIER_0_VAL_1_SECTION = LAYER_0_LEVEL_1_SECTION
NONHIER_0_NAME = Scan data layer
NONHIER_0_COUNT = 1
NONHIER_0_VAL_0 = ScanDataLayer_SlideBarcode
NONHIER_1_NAME = VIMSLIDE_POSITION_BUFFER
NONHIER_1_COUNT = 1
NONHIER_1_VAL_0 = default
[DATAFILE]
FILE_COUNT = 1
FILE_0 = Data0000.dat
[LAYER_0_LEVEL_0_SECTION]
IMAGE_FILL_COLOR_BGR = 16777215
MICROMETER_PER_PIXEL_X = 0.25
MICROMETER_PER_PIXEL_Y = 0.25
DIGITIZER_WIDTH = {TW}
DIGITIZER_HEIGHT = {TH}
IMAGE_CONCAT_FACTOR = 0
IMAGE_FORMAT = JPEG
[LAYER_0_LEVEL_1_SECTION]
DIGITIZER_WIDTH = {TW}
DIGITIZER_HEIGHT = {TH}
IMAGE_CONCAT_FACTOR = 1
IMAGE_FORMAT = JPEG
"""
    (OUT / "synthetic" / "Slidedat.ini").write_text(ini.replace("\n", "\r\n"), encoding="utf-8")
    (OUT / "synthetic.mrxs").write_bytes(jpeg(blocks(7), TW, TH))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

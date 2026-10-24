#!/usr/bin/env python3
"""Writes a small synthetic Olympus VSI dataset (`.vsi` + ETS).

Written from the record-tree and ETS layouts described in
doc/olympus-vsi/ (OpenReadout and bioio-vsi notes). Raw 16-bit tiles, two
channels, two pyramid levels, a negative tile-grid origin and one missing
tile. Also writes truth.tsv with the CRC-32 of level 0 in the layout of
tests/common/mod.rs `decode()`. Output in tests/data/vsi/ is committed.

Usage: python3 -I tests/scripts/make_vsi_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "vsi"
W, H, C = 150, 100, 2
TILE = 64
ORIGIN = (-3, -1)
BACKGROUND = 0x1234
MISSING = (2, 1, 1)  # column, row, channel of the level-0 tile not stored


def value(x, y, c):
    return (x * 37 + y * 71 + c * 1009) * 2654435761 % 60000


# --- Record tree ------------------------------------------------------------

def field(tag, kind, payload=b"", index=None, inline=None, volume=False):
    """One field as (header word, tag, payload, index, inline value)."""
    typ = kind
    if volume:
        typ |= 0x9000_0000
    if inline is not None:
        typ |= 0x4000_0000
    if index is not None:
        typ |= 0x0800_0000
    return (typ, tag, payload, index, inline)


def volume(fields):
    """Serialises a record volume; links are relative to its start."""
    body = bytearray()
    offsets = []
    pos = 24
    for typ, tag, payload, index, inline in fields:
        header = 20 if index is not None else 16
        offsets.append(pos)
        pos += header + len(payload)
    for i, (typ, tag, payload, index, inline) in enumerate(fields):
        nxt = offsets[i + 1] if i + 1 < len(fields) else 0
        size = inline if inline is not None else len(payload)
        body += struct.pack("<IIII", typ, tag, nxt, size)
        if index is not None:
            body += struct.pack("<I", index)
        body += payload
    head = struct.pack("<HHIqII", 24, 21321, 1, 24 if fields else 0, len(fields), 0)
    return head + bytes(body)


def utf16(s):
    return s.encode("utf-16le")


def tree():
    channels = [volume([field(2023, 5, inline=4), field(2021, 8192, utf16("DAPI"))]),
                volume([field(2021, 8192, utf16("FITC"))])]
    dim0 = volume([field(2008, 0, ch, index=i, volume=True) for i, ch in enumerate(channels)])
    settings = volume([
        field(2030, 8192, utf16("Synthetic 2C")),
        field(2019, 260, struct.pack("<dd", 0.25, 0.25)),
        field(2020, 13, utf16("10^-6m^1")),
        field(2015, 7, struct.pack("<q", 1_700_000_000)),
    ])
    layout = volume([field(2053, 259, struct.pack("<4i", 0, 0, W, H)),
                     field(2410, 8199, struct.pack("<2i", *ORIGIN))])
    plane = volume([field(2018, 0, layout, volume=True)])
    stack = volume([
        field(2003, 2, struct.pack("<I", C)),
        field(2005, 0, settings, volume=True),
        field(2007, 0, dim0, index=0, volume=True),
        field(2007, 0, volume([]), index=1, volume=True),
        field(2002, 0, plane, index=0, volume=True),
    ])
    collection = volume([field(2001, 0, stack, index=1, volume=True)])
    return volume([field(2000, 0, collection, index=0, volume=True)])


def tiff_preview(start):
    """A 2 x 2 RGB uncompressed IFD and its strip, placed at `start`."""
    pixels = bytes([255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255])
    entries = [(256, 3, 1, 2), (257, 3, 1, 2), (258, 3, 1, 8), (259, 3, 1, 1), (262, 3, 1, 2),
               (273, 4, 1, 0), (277, 3, 1, 3), (278, 3, 1, 2), (279, 4, 1, len(pixels))]
    ifd_len = 2 + 12 * len(entries) + 4
    strip = start + ifd_len
    out = struct.pack("<H", len(entries))
    for tag, typ, count, val in entries:
        if tag == 273:
            val = strip
        out += struct.pack("<HHI", tag, typ, count)
        out += struct.pack("<HH", val, 0) if typ == 3 else struct.pack("<I", val)
    out += struct.pack("<I", 0)
    return out + pixels


# --- ETS --------------------------------------------------------------------

def tile_pixels(level, col, row, c):
    """Raw tile data; level 1 holds an independent pattern."""
    out = bytearray()
    ox, oy = (ORIGIN[0], ORIGIN[1]) if level == 0 else (-1, 0)
    w, h = (W, H) if level == 0 else (-(-W // 2), -(-H // 2))
    for ly in range(TILE):
        for lx in range(TILE):
            x, y = col * TILE + lx + ox, row * TILE + ly + oy
            v = value(x, y, c) if 0 <= x < w and 0 <= y < h else 0
            if level == 1:
                v = (x + y + c) % 7 if 0 <= x < w and 0 <= y < h else 0
            out += struct.pack("<H", v)
    return bytes(out)


def ets():
    tiles = []
    for level in (0, 1):
        ox, oy = (ORIGIN[0], ORIGIN[1]) if level == 0 else (-1, 0)
        w, h = (W, H) if level == 0 else (-(-W // 2), -(-H // 2))
        cols, rows = -(-(w - ox) // TILE), -(-(h - oy) // TILE)
        for c in range(C):
            for row in range(rows):
                for col in range(cols):
                    if level == 0 and (col, row, c) == MISSING:
                        continue
                    tiles.append(((col, row, c, level), tile_pixels(level, col, row, c)))
    ets_header = bytearray(228)
    ets_header[:4] = b"ETS\0"
    struct.pack_into("<9I", ets_header, 4, 0x30006, 4, 1, 1, 0, 100, TILE, TILE, 1)
    struct.pack_into("<I", ets_header, 108, BACKGROUND)
    struct.pack_into("<4I", ets_header, 184, 3, W, H, C)
    data = bytearray()
    entries = bytearray()
    base = 64 + 228
    for coords, raw in tiles:
        entries += struct.pack("<I4iQII", 4, *coords, base + len(data), len(raw), 0)
        data += raw
    table = base + len(data)
    sis = struct.pack("<4sIIIQIIQIIQII", b"SIS\0", 64, 3, 4, 64, 228, 0, table, len(tiles), 0, 0, 0, 0)
    return sis + bytes(ets_header) + bytes(data) + bytes(entries)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "_synthetic_" / "stack1").mkdir(parents=True)
    body = tree()
    preview_at = 8 + len(body)
    vsi = b"II*\0" + struct.pack("<I", preview_at) + body + tiff_preview(preview_at)
    (OUT / "synthetic.vsi").write_bytes(vsi)
    (OUT / "_synthetic_" / "stack1" / "frame_t.ets").write_bytes(ets())

    out = bytearray()
    for y in range(H):
        for x in range(W):
            for c in range(C):
                # Pixels of the missing tile read as the background value.
                col, row = (x - ORIGIN[0]) // TILE, (y - ORIGIN[1]) // TILE
                v = BACKGROUND if (col, row, c) == MISSING else value(x, y, c)
                out += struct.pack("<H", v)
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tchannels\tsize_z\tsize_t\tcrc32\n")
        f.write(f"synthetic.vsi\t0\t{C}\t1\t1\t{zlib.crc32(bytes(out)):08x}\n")


if __name__ == "__main__":
    main()

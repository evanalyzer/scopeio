#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Hamamatsu DCIMG files.

Layout from OpenReadout's DCIMG format description (doc/hamamatsu-dcimg/)
and own hex dumps of the OME / Zenodo samples:

- packed_stored.dcimg: version 7, 3 frames of 16 x 6 uint16, 40 bytes per
  row (8 padding bytes), stored pixels: row 2, columns 1-4 hold
  0, 65535, 0, 65535 and the footer table their real values per frame.
- framed_stored.dcimg: version 0x1000000, 4 frames of 12 x 5, 32-byte
  trailers with stored pixels (row 3, columns 0-3, announced by a kind-4
  block), camera block with model, serial and sub-array.
- framed_plain.dcimg: version 0x1000000, 2 frames of 12 x 5, 16-byte
  trailers, frame counters 0 and 2 (a dropped frame).
- packed_u8.dcimg: version 7, 2 frames of 9 x 4 uint8, no stored pixels.
- truncated.dcimg: framed_plain with 3 frames, cut inside the third.

Each file is written into a folder of its own (<name>/<name>.dcimg):
Bio-Formats groups all .dcimg files of a folder into one series.

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()`: frames in order, each bottom row
first (as Bio-Formats returns DCIMG planes), stored values put back.
Output in tests/data/dcimg/ is committed.

Usage: python3 -I tests/scripts/make_dcimg_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "dcimg"
JUNK = (0, 65535, 0, 65535)
T0 = 1_725_555_197  # 2024-09-05T16:53:17Z


def frame_values(w, h, t, bpp):
    m = 256 if bpp == 1 else 65536
    return [[(x * 37 + y * 101 + t * 1009 + 7) % m for x in range(w)] for y in range(h)]


def pack_frame(rows, row_bytes, bpp):
    out = bytearray()
    for r in rows:
        line = bytes(r) if bpp == 1 else struct.pack(f"<{len(r)}H", *r)
        out += line + b"\0" * (row_bytes - len(line))
    return bytes(out)


def truth_bytes(frames, bpp):
    """Frames bottom row first, little endian."""
    out = bytearray()
    for rows in frames:
        for r in reversed(rows):
            out += bytes(r) if bpp == 1 else struct.pack(f"<{len(r)}H", *r)
    return bytes(out)


def file_header(version, frames, hb, size):
    h = bytearray(hb)
    h[0:8] = b"DCIMG\0\0\0"
    struct.pack_into("<I", h, 8, version)
    struct.pack_into("<III", h, 0x20, 1, frames, hb)
    struct.pack_into("<Q", h, 0x30, size)
    struct.pack_into("<Q", h, 0x40, size)
    return h


def packed(w, h, n, bpp, row_bytes, stored):
    """Version 7: header, frames, footer with counters, stamps, stored pixels."""
    hb, data_off = 0x78, 112
    frame_bytes = row_bytes * h
    frames = [frame_values(w, h, t, bpp) for t in range(n)]
    real = []
    if stored:
        row, col = stored
        for rows in frames:
            real.append(rows[row][col:col + 4])
    stored_frames = []
    for rows in frames:
        rows = [list(r) for r in rows]
        if stored:
            rows[stored[0]][stored[1]:stored[1] + 4] = JUNK
        stored_frames.append(rows)
    data = b"".join(pack_frame(r, row_bytes, bpp) for r in stored_frames)
    data_size = data_off + len(data)
    # Footer: u32 7, u64 offset of the second structure, its tables.
    counters, stamps, values = 0x110, 0x110 + 4 * n, 0x110 + 12 * n
    footer = bytearray(values + (8 * n if stored else 0))
    struct.pack_into("<I", footer, 0, 7)
    struct.pack_into("<Q", footer, 8, 0xa0)
    struct.pack_into("<I", footer, 0x28, len(footer))
    struct.pack_into("<Q", footer, 0xa0 + 0x30, counters)
    struct.pack_into("<Q", footer, 0xa0 + 0x40, stamps)
    for t in range(n):
        struct.pack_into("<I", footer, counters + 4 * t, t)
        struct.pack_into("<II", footer, stamps + 8 * t, T0, 250_000 * t)
    if stored:
        struct.pack_into("<Q", footer, 0xa0 + 0x58, values)
        struct.pack_into("<I", footer, 0xa0 + 0x64, stored[0] * row_bytes + stored[1] * bpp)
        struct.pack_into("<Q", footer, 0xa0 + 0x68, 8)
        for t, v in enumerate(real):
            struct.pack_into("<4H", footer, values + 8 * t, *v)
    session = bytearray(data_off)
    size = hb + data_size + len(footer)
    struct.pack_into("<Q", session, 0, size - hb)
    struct.pack_into("<IIIIIII", session, 0x20, n, bpp, 1, w, row_bytes, h, frame_bytes)
    struct.pack_into("<IQ", session, 0x44, data_off, data_size)
    blob = bytes(file_header(7, n, hb, size)) + bytes(session) + data + bytes(footer)
    return blob, truth_bytes(frames, bpp)


def framed(w, h, n, trailer, stored, counters=None):
    """Version 0x1000000: header, block table, camera block, frames + trailers."""
    hb, bpp = 0x70, 2
    row_bytes, frame_bytes = w * bpp, w * bpp * h
    stride = frame_bytes + trailer
    counters = counters or list(range(n))
    # Block table at hb + 0xf0, blocks from hb + 0xa0 + 0x80 on.
    table = []
    camera_at = 0x80
    table.append((3, 248, camera_at))
    if stored:
        table.append((4, 56, camera_at + 248))
    blocks_end = camera_at + 248 + (56 if stored else 0)
    data_off = 0xa0 + blocks_end + 16
    session = bytearray(data_off)
    struct.pack_into("<I", session, 0x3c, n)
    struct.pack_into("<I", session, 0x40, bpp)
    struct.pack_into("<IIII", session, 0x48, w, h, row_bytes, frame_bytes)
    struct.pack_into("<Q", session, 0x60, data_off)
    struct.pack_into("<I", session, 0x74, stride)
    struct.pack_into("<I", session, 0x7c, trailer)
    for i, (kind, size, off) in enumerate(table):
        struct.pack_into("<IIQ", session, 0xf0 + 16 * i, kind, size, off)
    cam = bytearray(248)
    for at, text in ((0x00, b"19.12.642.5901"), (0x20, b"7.5.140.5901"), (0x40, b"C11440-22C"), (0x80, b"2.50.A"), (0x90, b"S/N: 001634")):
        cam[at:at + len(text)] = text
    struct.pack_into("<4H", cam, 0xc8, 0, w * 2, 940, h)
    session[0xa0 + camera_at:0xa0 + camera_at + 248] = cam
    if stored:
        k4 = bytearray(56)
        struct.pack_into("<IIIII", k4, 0, 1, 0, 16, 8, stored[0] * row_bytes + stored[1] * bpp)
        session[0xa0 + camera_at + 248:0xa0 + camera_at + 248 + 56] = k4
    frames = [frame_values(w, h, t, bpp) for t in range(n)]
    body = bytearray()
    for t, rows in enumerate(frames):
        out = [list(r) for r in rows]
        if stored:
            out[stored[0]][stored[1]:stored[1] + 4] = JUNK
        body += pack_frame(out, row_bytes, bpp)
        tr = bytearray(trailer)
        struct.pack_into("<III", tr, 0, counters[t], T0 + t, 100_000 * t)
        if stored:
            struct.pack_into("<4H", tr, 12, *rows[stored[0]][stored[1]:stored[1] + 4])
        body += tr
    size = hb + len(session) + len(body)
    struct.pack_into("<Q", session, 0, size - hb)
    blob = bytes(file_header(0x01000000, n, hb, size)) + bytes(session) + bytes(body)
    return blob, truth_bytes(frames, bpp)


def write(name, blob):
    folder = OUT / name[:-len(".dcimg")]
    folder.mkdir()
    (folder / name).write_bytes(blob)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    for name, (blob, pixels) in [
        ("packed_stored.dcimg", packed(16, 6, 3, 2, 40, (2, 1))),
        ("framed_stored.dcimg", framed(12, 5, 4, 32, (3, 0))),
        ("framed_plain.dcimg", framed(12, 5, 2, 16, None, counters=[0, 2])),
        ("packed_u8.dcimg", packed(9, 4, 2, 1, 12, None)),
    ]:
        write(name, blob)
        truth.append((f"{name[:-6]}/{name}", zlib.crc32(pixels)))
    blob, pixels = framed(12, 5, 3, 16, None)
    stride = 12 * 2 * 5 + 16
    write("truncated.dcimg", blob[:len(blob) - stride // 2])
    truth.append(("truncated/truncated.dcimg", zlib.crc32(pixels[:2 * 12 * 2 * 5])))
    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

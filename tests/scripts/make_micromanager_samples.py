#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Micro-Manager datasets.

Layouts from the Micro-Manager file format page and the NDTiff
specification (doc/micro-manager/):

- stack/: image file stack, 2 positions, C 2, Z 2, T 2, uint16, 7 x 5.
  Position 0 is split over syn_MMStack_Pos0.ome.tif (t 0) and its
  continuation syn_MMStack_Pos0_1.ome.tif (t 1); position 1 is in the
  big-endian syn_MMStack_Pos1.ome.tif, whose image c 1, z 1, t 1 is missing
  (reads as zeros). Each file: TIFF header, index map offset header, display
  settings and comments headers, summary (JSON), IFDs with the image
  metadata (tag 51123, JSON), index map.
- sep/Pos0, sep/Pos1: separate image files (Micro-Manager 1.4,
  FrameKey-t-c-z entries), uint8, C 2, T 3.
- sep2/Pos0: separate image files (Micro-Manager 2.0, Coords-<file>),
  uint16, C 1, Z 2.
- nd/: NDTiff v3, 12-bit (pixel type 4), axes channel ("DAPI", "GFP"),
  z (-1, 0) and position (0, 1); the image channel GFP, z 0, position 1 is
  written twice, the index keeps the second.

Also writes truth.tsv with the CRC-32 of each series in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z, t).
Output in tests/data/micromanager/ is committed.

Usage: python3 -I tests/scripts/make_micromanager_samples.py
"""

import json
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "micromanager"
W, H = 7, 5

INDEX_MAP_OFFSET_HEADER = 54773648
INDEX_MAP_HEADER = 3453623
DISPLAY_HEADER = 483765892
COMMENTS_HEADER = 99384722
SUMMARY_HEADER = 2355492
NDTIFF_MAGIC = 483729


def stack_value(x, y, c, z, t, p):
    return 1000 + x * 401 + y * 37 + c * 5000 + z * 300 + t * 11000 + p * 23000


def stack_file(images, summary, endian):
    """`images`: list of ((c, z, t, p), samples, metadata dict)."""
    e = endian
    s = json.dumps(summary).encode()
    out = bytearray((b"II*\0" if e == "<" else b"MM\0*") + bytes(4))
    out += struct.pack(e + "8I", INDEX_MAP_OFFSET_HEADER, 0, DISPLAY_HEADER, 0, COMMENTS_HEADER, 0, SUMMARY_HEADER, len(s))
    out += s
    entries = []
    prev = 4
    for coords, samples, meta in images:
        if len(out) % 2:
            out += b"\0"
        m = json.dumps(meta).encode() + b"\0"
        n = 10
        ifd_at = len(out)
        pixels_at = ifd_at + 2 + 12 * n + 4
        meta_at = pixels_at + 2 * W * H
        tags = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
                (273, 4, 1, pixels_at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, 2 * W * H),
                (51123, 2, len(m), meta_at)]
        struct.pack_into(e + "I", out, prev, ifd_at)
        out += struct.pack(e + "H", n)
        for tag, typ, count, value in tags:
            if typ == 3:
                out += struct.pack(e + "HHIH2x", tag, typ, count, value)
            else:
                out += struct.pack(e + "HHII", tag, typ, count, value)
        prev = len(out)
        out += struct.pack(e + "I", 0)
        out += struct.pack(e + f"{W * H}H", *samples)
        out += m
        entries.append((*coords, ifd_at))
    if len(out) % 2:
        out += b"\0"
    struct.pack_into(e + "I", out, 12, len(out))
    out += struct.pack(e + "II", INDEX_MAP_HEADER, len(entries))
    for entry in entries:
        out += struct.pack(e + "5I", *entry)
    return bytes(out)


def make_stack(truth):
    d = OUT / "stack"
    d.mkdir(parents=True)
    summary = {"Prefix": "syn", "Width": W, "Height": H, "PixelType": "GRAY16", "BitDepth": 12,
               "Channels": 2, "Slices": 2, "Frames": 2, "Positions": 2, "ChNames": ["DAPI", "GFP"],
               "ChColors": [-16776961, -16711936], "PixelSize_um": 0.25, "z-step_um": 1.5, "Interval_ms": 500,
               "StartTime": "2026-10-29 10:11:12.345 +0100",
               "StagePositions": [{"Label": "Left"}, {"Label": "Right"}], "MicroManagerVersion": "2.0.3"}

    def image(c, z, t, p):
        samples = [stack_value(x, y, c, z, t, p) for y in range(H) for x in range(W)]
        meta = {"ChannelIndex": c, "SliceIndex": z, "FrameIndex": t, "PositionIndex": p,
                "ElapsedTime-ms": 500 * t + 10 * c, "Exposure-ms": 20 + c, "XPositionUm": 100.0 + p,
                "YPositionUm": -50.0, "ZPositionUm": 3.0 + 1.5 * z}
        return (c, z, t, p), samples, meta

    planes = lambda t, p: [image(c, z, t, p) for z in range(2) for c in range(2)]
    (d / "syn_MMStack_Pos0.ome.tif").write_bytes(stack_file(planes(0, 0), summary, "<"))
    (d / "syn_MMStack_Pos0_1.ome.tif").write_bytes(stack_file(planes(1, 0), summary, "<"))
    pos1 = [im for t in range(2) for im in planes(t, 1) if im[0] != (1, 1, 1, 1)]
    (d / "syn_MMStack_Pos1.ome.tif").write_bytes(stack_file(pos1, summary, ">"))
    for p in range(2):
        def v(x, y, c, z, t):
            return 0 if (c, z, t, p) == (1, 1, 1, 1) else stack_value(x, y, c, z, t, p)
        px = b"".join(struct.pack("<H", v(x, y, c, z, t)) for t in range(2) for z in range(2)
                      for y in range(H) for x in range(W) for c in range(2))
        truth.append(("stack/syn_MMStack_Pos0.ome.tif", p, zlib.crc32(px)))


def plain_tiff(samples, fmt, bits):
    data = struct.pack(f"<{len(samples)}{fmt}", *samples)
    tags = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, bits), (259, 3, 1, 1), (262, 3, 1, 1),
            (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(data))]
    out = bytearray(b"II*\0" + struct.pack("<I", 8 + len(data) + len(data) % 2))
    out += data + b"\0" * (len(data) % 2)
    out += struct.pack("<H", len(tags)) + b"".join(struct.pack("<HHII", *t) for t in tags) + bytes(4)
    return bytes(out)


def make_separate(truth):
    summary = {"Prefix": "sep", "Channels": 2, "Slices": 1, "Frames": 3, "ChNames": ["BF", "FITC"],
               "PixelSize_um": 0.5, "Interval_ms": 2000, "Time": "2026-10-29 08:00:00 +0000"}
    v = lambda x, y, c, t, p: (x * 30 + y * 7 + c * 90 + t * 3 + p * 50) % 256
    for p in range(2):
        d = OUT / "sep" / f"Pos{p}"
        d.mkdir(parents=True)
        meta = {"Summary": dict(summary, PositionIndex=p)}
        for t in range(3):
            for c, name in enumerate(summary["ChNames"]):
                f = f"img_{t:09d}_{name}_000.tif"
                (d / f).write_bytes(plain_tiff([v(x, y, c, t, p) for y in range(H) for x in range(W)], "B", 8))
                meta[f"FrameKey-{t}-{c}-0"] = {"FileName": f, "ElapsedTime-ms": 2000 * t + c,
                                               "Exposure-ms": 5, "XPositionUm": 10 * p, "YPositionUm": 0}
        (d / "metadata.txt").write_text(json.dumps(meta, indent=1))
        px = bytes(v(x, y, c, t, p) for t in range(3) for y in range(H) for x in range(W) for c in range(2))
        # Either metadata.txt opens both positions.
        truth.append(("sep/Pos1/metadata.txt", p, zlib.crc32(px)))

    d = OUT / "sep2" / "Pos0"
    d.mkdir(parents=True)
    meta = {"Summary": {"Prefix": "", "ChNames": ["Cy5"], "AxisOrder": ["z", "channel"],
                        "IntendedDimensions": {"z": 2, "channel": 1, "time": 1, "position": 1}}}
    v = lambda x, y, z: 300 + x * 1000 + y * 10 + z * 4000
    for z in range(2):
        f = f"img_channel000_position000_time000000000_z{z:03d}.tif"
        (d / f).write_bytes(plain_tiff([v(x, y, z) for y in range(H) for x in range(W)], "H", 16))
        meta[f"Coords-{f}"] = {"position": 0, "time": 0, "z": z, "channel": 0}
        meta[f"Metadata-{f}"] = {"PixelSizeUm": 0.1, "ZPositionUm": 2.0 * z, "Exposure-ms": "7.5"}
    (d / "metadata.txt").write_text(json.dumps(meta, indent=1))
    px = b"".join(struct.pack("<H", v(x, y, z)) for z in range(2) for y in range(H) for x in range(W))
    truth.append(("sep2/Pos0/metadata.txt", 0, zlib.crc32(px)))


def make_ndtiff(truth):
    d = OUT / "nd"
    d.mkdir(parents=True)
    summary = json.dumps({"Prefix": "nd", "PixelType": "GRAY16", "ChNames": ["DAPI", "GFP"],
                          "z-step_um": 2}).encode()
    v = lambda x, y, c, z, p, gen=0: (x * 300 + y * 41 + c * 700 + (z + 1) * 1100 + p * 150 + gen * 7) % 4096
    out = bytearray(b"II*\0" + bytes(4))
    out += struct.pack("<5I", NDTIFF_MAGIC, 3, 0, SUMMARY_HEADER, len(summary)) + summary
    index = bytearray()
    prev = 4
    order = [(c, z, p, 0) for p in range(2) for z in (-1, 0) for c in ("DAPI", "GFP")] + [("GFP", 0, 1, 1)]
    for c, z, p, gen in order:
        ci = ["DAPI", "GFP"].index(c)
        if len(out) % 2:
            out += b"\0"
        m = json.dumps({"ElapsedTime-ms": 100 * p, "PixelSizeUm": 0.65, "ZPositionUm": 2 * z}).encode()
        ifd_at = len(out)
        n = 9
        pixels_at = ifd_at + 2 + 12 * n + 4
        meta_at = pixels_at + 2 * W * H
        tags = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
                (273, 4, 1, pixels_at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, 2 * W * H)]
        struct.pack_into("<I", out, prev, ifd_at)
        out += struct.pack("<H", n) + b"".join(struct.pack("<HHII", *t) for t in tags)
        prev = len(out)
        out += bytes(4)
        out += struct.pack(f"<{W * H}H", *[v(x, y, ci, z, p, gen) for y in range(H) for x in range(W)])
        out += m
        axes = json.dumps({"channel": c, "z": z, "position": p}).encode()
        name = b"nd_NDTiffStack.tif"
        index += struct.pack("<I", len(axes)) + axes + struct.pack("<I", len(name)) + name
        index += struct.pack("<IiiiiIii", pixels_at, W, H, 4, 0, meta_at, len(m), 0)
    (d / "nd_NDTiffStack.tif").write_bytes(bytes(out))
    (d / "NDTiff.index").write_bytes(bytes(index))
    for p in range(2):
        gen = lambda c, z: 1 if (c, z, p) == (1, 0, 1) else 0
        px = b"".join(struct.pack("<H", v(x, y, c, z, p, gen(c, z))) for z in (-1, 0)
                      for y in range(H) for x in range(W) for c in range(2))
        truth.append(("nd", p, zlib.crc32(px)))


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    make_stack(truth)
    make_separate(truth)
    make_ndtiff(truth)
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for name, series, crc in truth:
            f.write(f"{name}\t{series}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

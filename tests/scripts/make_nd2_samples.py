#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Nikon ND2 files (modern chunk container).

Written from the container, LV and XML-variant layouts described in
doc/nikon-nd2/ (OpenReadout and nd2-rs notes). Also writes truth.tsv with
the CRC-32 of each series in the layout of tests/common/mod.rs `decode()`.
Output in tests/data/nd2/ is committed.

Usage: python3 -I tests/scripts/make_nd2_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "nd2"
MAGIC = 0x0ABECEDA
TRUTH = []


# --- LV encoding --------------------------------------------------------------

def name(n):
    units = n.encode("utf-16le") + b"\0\0"
    return bytes([len(units) // 2]) + units


def u32(n, v):
    return bytes([3]) + name(n) + struct.pack("<I", v)


def f64(n, v):
    return bytes([6]) + name(n) + struct.pack("<d", v)


def text(n, v):
    return bytes([8]) + name(n) + (v + "\0").encode("utf-16le")


def blob(n, v):
    return bytes([9]) + name(n) + struct.pack("<Q", len(v)) + v


def level(n, items):
    body = b"".join(items)
    return bytes([11]) + name(n) + struct.pack("<IQ", len(items), len(body)) + body + bytes(8 * len(items))


def loop(kind, count, params=(), children=None, extra=()):
    items = [u32("eType", kind), level("uLoopPars", [u32("uiCount", count), *params]), *extra]
    if children:
        items.append(level("ppNextLevelEx", [level("", children)]))
    return items


# --- Container ----------------------------------------------------------------

def chunk(name_, payload, name_len=None):
    raw = name_.encode() if isinstance(name_, str) else name_
    name_len = name_len or len(raw)
    return struct.pack("<IIQ", MAGIC, name_len, len(payload)) + raw.ljust(name_len, b"\0") + payload


def write(path, version, chunks):
    data = bytearray(chunk("ND2 FILE SIGNATURE CHUNK NAME01!", version.encode().ljust(64, b"\0")))
    entries = b""
    for n, payload in chunks:
        offset = len(data)
        data += chunk(n, payload)
        entries += n.encode() + struct.pack("<QQ", offset, len(payload))
    entries += b"ND2 CHUNK MAP SIGNATURE 0000001!" + struct.pack("<QQ", 0, 0)
    map_offset = len(data)
    data += chunk("ND2 FILEMAP SIGNATURE NAME 0001!", entries)
    data += b"ND2 CHUNK MAP SIGNATURE 0000001!" + struct.pack("<Q", map_offset)
    path.write_bytes(bytes(data))


def value(f, y, x, s):
    return (f * 7919 + y * 131 + x * 17 + s * 101) % 4000


def frames(n, w, h, comp, bytes_per, stride):
    """Raw frames: a time stamp, then rows of `stride` bytes."""
    fmt = {1: "B", 2: "H"}[bytes_per]
    out = []
    for f in range(n):
        rows = b""
        for y in range(h):
            row = b"".join(struct.pack("<" + fmt, value(f, y, x, s) % (1 << (8 * bytes_per)))
                           for x in range(w) for s in range(comp))
            rows += row.ljust(stride, b"\0")
        out.append(rows)
    return out


def truth(file, series, frame_ids, w, h, comp, bytes_per, planes_rgb=False):
    """Expected decoding: frames in z, t order, samples per pixel (RGB reversed)."""
    fmt = {1: "B", 2: "H"}[bytes_per]
    data = b""
    for f in frame_ids:
        for y in range(h):
            for x in range(w):
                order = [2, 1, 0] if planes_rgb else range(comp)
                for s in order:
                    data += struct.pack("<" + fmt, value(f, y, x, s) % (1 << (8 * bytes_per)))
    TRUTH.append((file, series, f"{zlib.crc32(data):08x}"))


def attributes(w, h, comp, bits, count, compression=2, stride=None):
    stride = stride or w * comp * bits // 8
    return level("SLxImageAttributes", [
        u32("uiWidth", w), u32("uiHeight", h), u32("uiWidthBytes", stride), u32("uiComp", comp),
        u32("uiBpcInMemory", bits), u32("uiBpcSignificant", bits), u32("uiSequenceCount", count),
        u32("eCompression", compression), u32("ePixelType", 1)])


def planes(*specs):
    return level("SLxPictureMetadata", [
        f64("dCalibration", 0.5), f64("dTimeAbsolute", 2457910.885497454),
        text("wsObjectiveName", "Plan Apo 20x"), f64("dObjectiveMag", 20.0), f64("dObjectiveNA", 0.75),
        level("sPicturePlanes", [level("sPlaneNew", [
            level(f"a{i}", [text("sDescription", d), u32("uiCompCount", c), u32("uiColor", col)])
            for i, (d, c, col) in enumerate(specs)])])])


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    # A: two channels of uint16, padded rows, Z loop of 3 under a spectral loop.
    w, h, stride = 7, 5, 7 * 2 * 2 + 4
    exp = level("SLxExperiment", loop(6, 0, children=loop(4, 3, [f64("dZStep", 0.3)])))
    data = frames(3, w, h, 2, 2, stride)
    write(OUT / "zstack_2ch.nd2", "Ver3.0", [
        ("ImageAttributesLV!", attributes(w, h, 2, 16, 3, stride=stride)),
        ("ImageMetadataLV!", exp),
        ("ImageMetadataSeqLV|0!", planes(("DAPI", 1, 0xFF0000), ("FITC", 1, 0x00FF00))),
        *[(f"ImageDataSeq|{i}!", struct.pack("<d", i * 10.0) + d) for i, d in enumerate(data)],
    ])
    truth("zstack_2ch.nd2", 0, [0, 1, 2], w, h, 2, 2)

    # B: RGB (stored blue first), zlib frames, time loop of 2.
    w, h = 6, 4
    exp = level("SLxExperiment", loop(1, 2, [f64("dPeriod", 1500.0)]))
    data = frames(2, w, h, 3, 1, w * 3)
    write(OUT / "rgb_zlib_time.nd2", "Ver3.0", [
        ("ImageAttributesLV!", attributes(w, h, 3, 8, 2, compression=0)),
        ("ImageMetadataLV!", exp),
        ("ImageMetadataSeqLV|0!", planes(("Brightfield", 3, 0xFFFFFF))),
        *[(f"ImageDataSeq|{i}!", struct.pack("<d", 0.0) + zlib.compress(d)) for i, d in enumerate(data)],
    ])
    truth("rgb_zlib_time.nd2", 0, [0, 1], w, h, 3, 1, planes_rgb=True)

    # C: XY loop with three points, the second invalid, times a T loop of 2.
    w, h = 4, 3
    points = level("Points", [level("", [f64("dPosX", 100.0 * i), f64("dPosY", 5.0), f64("dPosZ", 1.0)]) for i in range(3)])
    exp = level("SLxExperiment", loop(1, 2, [f64("dPeriod", 60000.0)], children=loop(
        2, 3, [points], extra=[blob("pItemValid", bytes([1, 0, 1]))])))
    data = frames(4, w, h, 1, 2, w * 2)
    write(OUT / "positions_time.nd2", "Ver3.0", [
        ("ImageAttributesLV!", attributes(w, h, 1, 16, 4)),
        ("ImageMetadataLV!", exp),
        ("ImageMetadataSeqLV|0!", planes(("mCherry", 1, 0x0000FF))),
        *[(f"ImageDataSeq|{i}!", struct.pack("<d", 0.0) + d) for i, d in enumerate(data)],
    ])
    # frame = t * 2 + p
    truth("positions_time.nd2", 0, [0, 2], w, h, 1, 2)
    truth("positions_time.nd2", 1, [1, 3], w, h, 1, 2)

    # D: version 2 with XML variants, uint8, Z loop of 2.
    w, h = 5, 3
    def xml(body):
        return ('<?xml version="1.0" encoding="UTF-8"?><variant version="1.0"><no_name runtype="CLxListVariant">'
                + body + "</no_name></variant>").encode()
    attrs = xml("".join(f'<{k} runtype="lx_uint32" value="{v}"/>' for k, v in [
        ("uiWidth", w), ("uiHeight", h), ("uiWidthBytes", w), ("uiComp", 1), ("uiBpcInMemory", 8),
        ("uiBpcSignificant", 8), ("uiSequenceCount", 2), ("eCompression", 2), ("ePixelType", 1)]))
    exp = xml('<eType runtype="lx_int32" value="4"/><uLoopPars runtype="CLxListVariant">'
              '<uiCount runtype="lx_uint32" value="2"/><dZStep runtype="double" value="1.5"/></uLoopPars>')
    data = frames(2, w, h, 1, 1, w)
    write(OUT / "v2_xml_z.nd2", "Ver2.1", [
        ("ImageAttributes!", attrs),
        ("ImageMetadata!", exp),
        *[(f"ImageDataSeq|{i}!", struct.pack("<d", 0.0) + d) for i, d in enumerate(data)],
    ])
    truth("v2_xml_z.nd2", 0, [0, 1], w, h, 1, 1)

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for row in TRUTH:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

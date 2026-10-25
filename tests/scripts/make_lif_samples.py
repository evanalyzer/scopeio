#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Leica LIF files.

Written from the header, memory block and XML layouts described in
doc/leica-lif/ (OpenReadout notes). Also writes truth.tsv with the CRC-32
of each series in the layout of tests/common/mod.rs `decode()`. Output in
tests/data/lif/ is committed.

Usage: python3 -I tests/scripts/make_lif_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "lif"
TRUTH = []


def value(x, y, c=0, z=0, t=0, m=0, k=0):
    return (x * 11 + y * 7 + c * 97 + z * 31 + t * 53 + m * 13 + k * 3) % 4000


def half(v):
    return struct.pack("<e", v)


def channel(res, inc, lut, tag=0, dtype=0):
    return (f'<ChannelDescription DataType="{dtype}" ChannelTag="{tag}" Resolution="{res}" LUTName="{lut}" '
            f'BytesInc="{inc}" BitInc="0"/>')


def dim(i, n, length, inc, unit="m", origin=0.0):
    return f'<DimensionDescription DimID="{i}" NumberOfElements="{n}" Origin="{origin}" Length="{length}" Unit="{unit}" BytesInc="{inc}" BitInc="0"/>'


def element(name, channels, dims, block, size, extra=""):
    return (f'<Element Name="{name}"><Data><Image><ImageDescription><Channels>{"".join(channels)}</Channels>'
            f'<Dimensions>{"".join(dims)}</Dimensions></ImageDescription>{extra}</Image></Data>'
            f'<Memory Size="{size}" MemoryBlockID="{block}"/><Children/></Element>')


def write(path, elements, blocks):
    xml = ('<LMSDataContainerHeader Version="2"><Element Name="project"><Data><Experiment/></Data>'
           '<Memory Size="0" MemoryBlockID="MemBlock_1"/><Children>' + "".join(elements) + '</Children></Element></LMSDataContainerHeader>')
    text = xml.encode("utf-16-le")
    n = len(text) // 2
    out = bytearray(struct.pack("<II", 0x70, 2 * n + 5) + bytes([0x2A]) + struct.pack("<I", n) + text)
    for bid, data in [("MemBlock_1", b"")] + blocks:
        idu = bid.encode("utf-16-le")
        out += struct.pack("<II", 0x70, len(idu) + 14) + bytes([0x2A]) + struct.pack("<Q", len(data)) + bytes([0x2A])
        out += struct.pack("<I", len(idu) // 2) + idu + data
    path.write_bytes(bytes(out))


def crc(planes_fn, size_t, size_z, w, h, channels, fmt):
    """Expected `decode()` bytes: planes t, z; pixels with channels interleaved."""
    data = bytearray()
    for t in range(size_t):
        for z in range(size_z):
            for y in range(h):
                for x in range(w):
                    for c in range(channels):
                        data += struct.pack("<" + fmt, planes_fn(x, y, c, z, t))
    return f"{zlib.crc32(bytes(data)):08x}"


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    # A: two 12-bit channels in uint16, planar, Z = 3 inside channels, T = 2
    # outermost; XY step 0.2 µm.
    w, h, nz, nt = 6, 5, 3, 2
    plane = w * h * 2
    data = bytearray()
    for t in range(nt):
        for c in range(2):
            for z in range(nz):
                for y in range(h):
                    for x in range(w):
                        data += struct.pack("<H", value(x, y, c, z, t))
    chans = [channel(12, 0, "Green"), channel(12, nz * plane, "Red")]
    dims = [dim(1, w, 0.2e-6 * (w - 1), 2), dim(2, h, 0.2e-6 * (h - 1), w * 2), dim(3, nz, 1e-6 * (nz - 1), plane),
            dim(4, nt, 30.0, 2 * nz * plane, unit="s")]
    stamps = '<TimeStampList NumberOfTimeStamps="1">1D58B1C2A5B6E00</TimeStampList>'
    objective = '<Attachment Name="HardwareSetting"><ATLConfocalSettingDefinition ObjectiveName="HC PL APO 63x/1.40 OIL " Magnification="63" NumericalAperture="1.4" Immersion="OIL"/></Attachment>'
    a = element("stack", chans, dims, "MemBlock_2", len(data), stamps + objective)
    TRUTH.append(("planar.lif", 0, crc(lambda x, y, c, z, t: value(x, y, c, z, t), nt, nz, w, h, 2, "H")))
    blocks = [("MemBlock_2", bytes(data))]

    # B: RGB, stored blue first (tags 3, 2, 1 at 0, 1, 2), rows padded? no:
    # x stride 3, y stride 3 * w.
    w, h = 4, 3
    data = bytearray()
    for y in range(h):
        for x in range(w):
            data += bytes([value(x, y, k=2) % 256, value(x, y, k=1) % 256, value(x, y, k=0) % 256])
    chans = [channel(8, 0, "Blue", 3), channel(8, 1, "Green", 2), channel(8, 2, "Red", 1)]
    b = element("rgb", chans, [dim(1, w, 1e-6, 3), dim(2, h, 1e-6, 3 * w)], "MemBlock_3", len(data))
    TRUTH.append(("planar.lif", 1, crc(lambda x, y, c, z, t: value(x, y, k=c) % 256, 1, 1, w, h, 3, "B")))
    blocks.append(("MemBlock_3", bytes(data)))
    write(OUT / "planar.lif", [a, b], blocks)

    # C: tile scan of three 8-bit tiles, then a lambda scan of 3 steps in
    # half floats.
    w, h, nm = 5, 4, 3
    data = bytearray()
    for m in range(nm):
        for y in range(h):
            for x in range(w):
                data += bytes([value(x, y, m=m) % 256])
    tiles = "".join(f'<Tile FieldX="{m}" FieldY="0" PosX="{m * 1e-4}" PosY="0.002" PosZ="0"/>' for m in range(nm))
    info = f'<Attachment Name="TileScanInfo" FlipX="0" FlipY="0" SwapXY="0">{tiles}</Attachment>'
    c = element("tiles", [channel(8, 0, "Gray")], [dim(1, w, 4e-6, 1), dim(2, h, 3e-6, w), dim(10, nm, 0, w * h)], "MemBlock_4", len(data), info)
    for m in range(nm):
        TRUTH.append(("tiles_lambda.lif", m, crc(lambda x, y, c, z, t, m=m: value(x, y, m=m) % 256, 1, 1, w, h, 1, "B")))
    blocks = [("MemBlock_4", bytes(data))]
    w, h, nl = 3, 2, 3
    data = bytearray()
    for l in range(nl):
        for y in range(h):
            for x in range(w):
                data += half(value(x, y, c=l) / 8.0)
    d = element("lambda", [channel(16, 0, "Red", 0, 1)],
                [dim(1, w, 1e-6, 2), dim(2, h, 1e-6, 2 * w), dim(5, nl, 40e-9, 2 * w * h, origin=500e-9)], "MemBlock_5", len(data))
    TRUTH.append(("tiles_lambda.lif", 3, crc(lambda x, y, c, z, t: value(x, y, c=c) / 8.0, 1, 1, w, h, nl, "f")))
    blocks.append(("MemBlock_5", bytes(data)))
    write(OUT / "tiles_lambda.lif", [c, d], blocks)

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for row in TRUTH:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

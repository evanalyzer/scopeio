#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Opera Flex file.

Layout from the public Flex files of IDR studies (own analysis): a TIFF
with one page per image and the Flex XML in tag 65200 (Root/Arrays and
FLEX with Plate, Well/WellCoordinate and Images: BufferNo, ExposureNo,
CameraRef, Sublayout, Stack, PositionX/Y/Z, MeasurementTimeOffset,
CameraExposureTime, ImageResolutionX/Y, CompressionType).

- well.flex: well C04, 2 fields x 2 cameras x 2 planes, uint16 7 x 5,
  pages in field, plane, camera order; 0.25 um pixels.

Also writes truth.tsv with the CRC-32 of each field in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/flex/ is
committed.

Usage: python3 -I tests/scripts/make_flex_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "flex"
W, H = 7, 5


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    v = lambda f, z, c, x, y: (f * 9000 + z * 3000 + c * 700 + x * 31 + y * 7) % 65536
    order = [(f, z, c) for f in range(2) for z in range(2) for c in range(2)]
    images = "".join(f'''<Image BufferNo="{k}"><ObjectiveRef>20x</ObjectiveRef><ExposureNo Type="integer">1</ExposureNo>\
<CameraExposureTime Type="decimal" Unit="s">{0.1 * (c + 1):.3f}</CameraExposureTime><Stack Type="integer">{z + 1}</Stack>\
<Sublayout Type="integer">{f + 1}</Sublayout><PositionX Type="decimal" Unit="m">{f * 1e-4:.3e}</PositionX>\
<PositionY Type="decimal" Unit="m">0.000e+000</PositionY><PositionZ Type="decimal" Unit="m">{z * 2e-6:.3e}</PositionZ>\
<MeasurementTimeOffset Type="decimal" Unit="s">{10 + k}.00</MeasurementTimeOffset><CameraRef>Camera_{c + 1}</CameraRef>\
<CompressionType Type="string"></CompressionType><ImageResolutionX Type="decimal" Unit="m">2.500e-007</ImageResolutionX>\
<ImageResolutionY Type="decimal" Unit="m">2.500e-007</ImageResolutionY></Image>''' for k, (f, z, c) in enumerate(order))
    xml = ('<?xml version="1.0" encoding="UTF-8"?><Root><Arrays/><FLEX version="1.8.1.0"><Objectives><Objective ID="20x">'
           '<Magnification Type="decimal">20.0</Magnification><NumAperture Type="decimal">0.7</NumAperture></Objective></Objectives>'
           '<Plate><Barcode Type="string">synthetic</Barcode><StartTime Type="date">2026-11-04T10:11:12</StartTime></Plate>'
           f'<Well><WellCoordinate Row="3" Col="4"/><Images>{images}</Images></Well></FLEX></Root>').encode() + b"\0"
    out = bytearray(b"II*\0" + bytes(4))
    xml_at = len(out)
    out += xml + b"\0" * (len(xml) % 2)
    prev = 4
    for k, (f, z, c) in enumerate(order):
        data = struct.pack(f"<{W * H}H", *[v(f, z, c, x, y) for y in range(H) for x in range(W)])
        at = len(out)
        out += data
        tags = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
                (273, 4, 1, at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(data))]
        if k == 0:
            tags.append((65200, 2, len(xml), xml_at))
        ifd = len(out)
        struct.pack_into("<I", out, prev, ifd)
        out += struct.pack("<H", len(tags)) + b"".join(struct.pack("<HHII", *t) for t in tags)
        prev = len(out)
        out += bytes(4)
    (OUT / "well.flex").write_bytes(bytes(out))
    with open(OUT / "truth.tsv", "w") as t:
        t.write("series\tcrc32\n")
        for f in range(2):
            px = b"".join(struct.pack("<H", v(f, z, c, x, y)) for z in range(2) for y in range(H) for x in range(W) for c in range(2))
            t.write(f"{f}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

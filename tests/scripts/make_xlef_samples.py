#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Leica LOF and XLEF/XLCF/XLIF files.

Layout from liffile (BSD-3-Clause) and the LIF notes (doc/leica-lif/):

- image.lof: LIF header block (0x70, length, 0x2A, UTF-16 `LMS_Object_File`),
  two (0x2A, u32) words, the memory block (0x2A, u64 length, samples), then
  a second header block with the UTF-16 XML: one 10 x 6 RGB image (channel
  tags 1-3 interleaved, 8 bits).
- experiment/experiment.xlef: references `.\\Metadata\\collection.xlcf`
  (percent-encoded, opened without regard to case), which references
  `image.xlif`: 12 x 8, 2 channels x 3 z, 16 bits, its memory block made of
  one uncompressed TIFF frame per plane (Offset/Size in the block),
  listed out of order.

Also writes truth.tsv with the CRC-32 of the pixels in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/xlef/ is
committed.

Usage: python3 -I tests/scripts/make_xlef_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "xlef"


def utf16(text):
    return text.encode("utf-16-le")


def header_block(text):
    """0x70, length, 0x2A, u32 units, UTF-16 text."""
    t = utf16(text)
    return struct.pack("<IIBI", 0x70, len(t) + 5, 0x2A, len(text)) + t


def description(channels, dims):
    ch = "".join(f'<ChannelDescription DataType="0" ChannelTag="{tag}" Resolution="{res}" BytesInc="{inc}" LUTName="{lut}"/>'
                 for tag, res, inc, lut in channels)
    dm = "".join(f'<DimensionDescription DimID="{d}" NumberOfElements="{n}" Origin="0" Length="{length}" Unit="m" BytesInc="{inc}"/>'
                 for d, n, length, inc in dims)
    return f"<ImageDescription><Channels>{ch}</Channels><Dimensions>{dm}</Dimensions></ImageDescription>"


def tiff16(width, height, data):
    entries = [(256, 3, 1, width), (257, 3, 1, height), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, height), (279, 4, 1, len(data))]
    ifd = struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    return b"II*\0" + struct.pack("<I", 8 + len(data)) + data + ifd


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "experiment" / "Metadata").mkdir(parents=True)
    truth = []

    # LOF: 10 x 6 RGB, channel tags 3, 2, 1 at byte increments 0, 1, 2 (blue first).
    W, H = 10, 6
    rgb = lambda x, y, k: (x * 23 + y * 41 + k * 70) % 256
    samples = b"".join(bytes([rgb(x, y, 2), rgb(x, y, 1), rgb(x, y, 0)]) for y in range(H) for x in range(W))
    desc = description([(3, 8, 0, "Blue"), (2, 8, 1, "Green"), (1, 8, 2, "Red")],
                       [(1, W, 1e-6 * (W - 1), 3), (2, H, 1e-6 * (H - 1), 3 * W)])
    xml = (f'<LMSDataContainerHeader Version="2"><Element Name="rgb image" UniqueID="u1"><Data><Image>{desc}</Image></Data>'
           f'<Memory Size="{len(samples)}" MemoryBlockID="MemBlock_7"/></Element></LMSDataContainerHeader>')
    lof = header_block("LMS_Object_File") + struct.pack("<BIBI", 0x2A, 2, 0x2A, 1) + struct.pack("<BQ", 0x2A, len(samples)) + samples
    lof += header_block(xml)
    (OUT / "image.lof").write_bytes(lof)
    truth.append(("image.lof", b"".join(bytes([rgb(x, y, 0), rgb(x, y, 1), rgb(x, y, 2)]) for y in range(H) for x in range(W))))

    # XLIF: 12 x 8, 2 channels (plane-separated) x 3 z, 16 bits; one TIFF per plane.
    W, H, C, Z = 12, 8, 2, 3
    val = lambda x, y, c, z: (x * 301 + y * 97 + c * 4000 + z * 777) % 65536
    plane = W * H * 2
    desc = description([(0, 16, 0, "Green"), (0, 16, plane, "Red")],
                       [(1, W, 0.5e-6 * (W - 1), 2), (2, H, 0.5e-6 * (H - 1), 2 * W), (3, Z, 2e-6 * (Z - 1), C * plane)])
    frames = []
    for z in range(Z):
        for c in range(C):
            data = b"".join(struct.pack("<H", val(x, y, c, z)) for y in range(H) for x in range(W))
            name = f"image--Z{z:02d}--C{c:02d}.tif"
            (OUT / "experiment" / name).write_bytes(tiff16(W, H, data))
            frames.append((name, (z * C + c) * plane, plane))
    frames.reverse()
    frame_xml = "".join(f'<Frame File="..%5C{n}" Offset="{o}" Size="{s}" UUID="f{i}"/>' for i, (n, o, s) in enumerate(frames))
    xlif = (f'<?xml version="1.0" encoding="utf-8"?><LMSDataContainerHeader Version="2"><Element Name="stack" UniqueID="u2">'
            f'<Data><Image>{desc}</Image></Data><Memory Size="{C * Z * plane}" MemoryBlockID="MemBlock_1">{frame_xml}</Memory>'
            f'</Element></LMSDataContainerHeader>')
    (OUT / "experiment" / "Metadata" / "image.xlif").write_bytes(b"\xef\xbb\xbf" + xlif.encode())
    xlcf = ('<?xml version="1.0" encoding="utf-8"?><LMSDataContainerHeader Version="2"><Element Name="folder A" UniqueID="u3">'
            '<Data><Collection/></Data><Children><Reference File=".%5Cimage.xlif" UUID="u2"/></Children></Element></LMSDataContainerHeader>')
    (OUT / "experiment" / "Metadata" / "collection.xlcf").write_bytes(xlcf.encode())
    xlef = ('<?xml version="1.0" encoding="utf-8"?><LMSDataContainerHeader Version="2"><Element Name="experiment.xlef" UniqueID="u4">'
            '<Data><Experiment/></Data><Children><Reference File=".%5Cmetadata%5Ccollection.xlcf" UUID="u3"/></Children>'
            '</Element></LMSDataContainerHeader>')
    (OUT / "experiment" / "experiment.xlef").write_bytes(b"\xef\xbb\xbf" + xlef.encode())
    px = b"".join(struct.pack("=H", val(x, y, c, z)) for z in range(Z) for y in range(H) for x in range(W) for c in range(C))
    truth.append(("experiment/experiment.xlef", px))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, data in truth:
            f.write(f"{name}\t{zlib.crc32(data):08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic OME-XML files with inline pixels.

Layout from the OME 2016-06 schema (doc/ome-xml/): one BinData element per
plane in DimensionOrder order, base64 text, Compression "none" or "zlib",
BigEndian.

- zlib_be.ome.xml: two images. Image 0: int16, 7 x 5, C 2, Z 3,
  DimensionOrder XYZCT, zlib-compressed big-endian planes, 0.5 um pixels.
  Image 1: uint8, 7 x 5, T 2, uncompressed, second plane an empty BinData
  (reads as zeros).
- bit.ome.xml: type "bit", 9 x 3, packed most significant bit first.

Also writes truth.tsv with the CRC-32 of each image in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z,
t). Output in tests/data/omexml/ is committed.

Usage: python3 -I tests/scripts/make_omexml_samples.py
"""

import base64
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "omexml"
W, H = 7, 5
HEAD = '<?xml version="1.0" encoding="UTF-8"?>\n<OME xmlns="http://www.openmicroscopy.org/Schemas/OME/2016-06">\n'


def bindata(raw, compression="none", big=False):
    data = zlib.compress(raw) if compression == "zlib" else raw
    text = base64.b64encode(data).decode()
    return f'      <BinData BigEndian="{"true" if big else "false"}" Compression="{compression}" Length="{len(text)}">{text}</BinData>\n'


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    v = lambda x, y, c, z: x * 301 - y * 77 + c * 5000 - z * 1200
    bins = "".join(bindata(b"".join(struct.pack(">h", v(x, y, c, z)) for y in range(H) for x in range(W)), "zlib", True)
                   for c in range(2) for z in range(3))
    u = lambda x, y, t: (x * 31 + y * 7 + t * 100) % 256
    bins1 = bindata(bytes(u(x, y, 0) for y in range(H) for x in range(W))) + '      <BinData BigEndian="false" Length="0"/>\n'
    xml = HEAD + f'''  <Image ID="Image:0" Name="compressed">
    <Pixels ID="Pixels:0" DimensionOrder="XYZCT" Type="int16" SizeX="{W}" SizeY="{H}" SizeZ="3" SizeC="2" SizeT="1" PhysicalSizeX="0.5" PhysicalSizeY="0.5">
      <Channel ID="Channel:0:0" Name="DAPI" SamplesPerPixel="1"/>
      <Channel ID="Channel:0:1" Name="GFP" SamplesPerPixel="1"/>
{bins}    </Pixels>
  </Image>
  <Image ID="Image:1" Name="plain">
    <Pixels ID="Pixels:1" DimensionOrder="XYCZT" Type="uint8" SizeX="{W}" SizeY="{H}" SizeZ="1" SizeC="1" SizeT="2">
      <Channel ID="Channel:1:0" SamplesPerPixel="1"/>
{bins1}    </Pixels>
  </Image>
</OME>
'''
    (OUT / "zlib_be.ome.xml").write_text(xml)
    truth.append(("zlib_be.ome.xml", 0, zlib.crc32(b"".join(struct.pack("<h", v(x, y, c, z)) for z in range(3) for y in range(H) for x in range(W) for c in range(2)))))
    truth.append(("zlib_be.ome.xml", 1, zlib.crc32(bytes(u(x, y, 0) for y in range(H) for x in range(W)) + bytes(W * H))))

    bw, bh = 9, 3
    bits = [(x * 3 + y * 5) % 7 < 3 for y in range(bh) for x in range(bw)]
    packed = bytes(sum(bits[i + k] << (7 - k) for k in range(8) if i + k < len(bits)) for i in range(0, len(bits), 8))
    xml = HEAD + f'''  <Image ID="Image:0" Name="bits">
    <Pixels ID="Pixels:0" DimensionOrder="XYZCT" Type="bit" SizeX="{bw}" SizeY="{bh}" SizeZ="1" SizeC="1" SizeT="1">
      <Channel ID="Channel:0:0" SamplesPerPixel="1"/>
{bindata(packed)}    </Pixels>
  </Image>
</OME>
'''
    (OUT / "bit.ome.xml").write_text(xml)
    truth.append(("bit.ome.xml", 0, zlib.crc32(bytes(int(b) for b in bits))))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for name, s, crc in truth:
            f.write(f"{name}\t{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

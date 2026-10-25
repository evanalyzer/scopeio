#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Olympus OIR acquisition with a continuation file.

Layout from the OpenReadout OIR notes (doc/olympus-oir/) and oirfile
(BSD-3-Clause): 96-byte header (`OLYMPUSRAWFORMAT`, declared size, index
offset, block count), blocks (`u32 payload length, u32 kind`), an index of
block offsets after the marker 0xFFFFFFFF. Kinds: 0 XML documents (36
unread bytes, then `u32 length` + XML per document), 1 frame properties,
3 chunk tag (`u32 plane offset, u32 chunk length, u32 name length, name`),
4 pixels, 5 separator. 2 channels x 3 z x 2 t of 24 x 10 uint16; each
plane in chunks of 3 rows, the two channels' chunks interleaved; channel
order from `@order` (opposite to the uuid order); time point 2 is stored in
`synthetic_00001`. Also writes truth.tsv with the CRC-32 of the pixels in
the layout of tests/common/mod.rs `decode_level()`. Output in
tests/data/oir/ is committed.

Usage: python3 -I tests/scripts/make_oir_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "oir"
W, H, Z, T = 24, 10, 3, 2
# Channels by uuid; `order` puts the second uuid first.
CHANNELS = [("aaaaaaaa-0000-4000-8000-000000000001", 2, "CH2"), ("bbbbbbbb-0000-4000-8000-000000000002", 1, "CH1")]
ROWS_PER_CHUNK = 3


def value(x, y, c, z, t):
    return (x * 53 + y * 17 + c * 1500 + z * 333 + t * 2001) % 65536


def plane_bytes(c, z, t):
    return b"".join(struct.pack("<H", value(x, y, c, z, t)) for y in range(H) for x in range(W))


def block(kind, payload):
    return struct.pack("<II", len(payload), kind) + payload


def documents(docs):
    payload = bytes(36)
    for d in docs:
        b = d.encode()
        payload += struct.pack("<I", len(b)) + b
    return payload


IMAGE_PROPERTIES = """<?xml version="1.0" encoding="ASCII"?>
<lsmimage:imageProperties xmlns:lsmimage="l" xmlns:commonimage="ci" xmlns:commonphase="cp" xmlns:commonparam="cr" xmlns:base="b" xmlns:opticalelement="o">
  <commonimage:general><base:creationDateTime>2024-01-02T03:04:05.600+01:00</base:creationDateTime></commonimage:general>
  <commonimage:system><base:systemName>FV4000</base:systemName></commonimage:system>
  <commonimage:imageInfo>
    <commonimage:phase id="p1">
      <commonphase:group id="g1">
        {channels}
      </commonphase:group>
    </commonimage:phase>
    <commonimage:axis><commonparam:axis>ZSTACK</commonparam:axis><commonparam:startPosition>10.0</commonparam:startPosition><commonparam:endPosition>13.0</commonparam:endPosition><commonparam:step>-1.5</commonparam:step><commonparam:maxSize>3</commonparam:maxSize></commonimage:axis>
    <commonimage:axis><commonparam:axis>TIMELAPSE</commonparam:axis><commonparam:startPosition>0.0</commonparam:startPosition><commonparam:endPosition>0.0</commonparam:endPosition><commonparam:step>0.0</commonparam:step><commonparam:maxSize>2</commonparam:maxSize></commonimage:axis>
  </commonimage:imageInfo>
  <commonimage:acquisition>
    <commonphase:length><commonparam:x>0.25</commonparam:x><commonparam:y>0.25</commonparam:y></commonphase:length>
    <commonphase:pixelUnit><commonphase:x>MICRO_METER</commonphase:x><commonphase:y>MICRO_METER</commonphase:y></commonphase:pixelUnit>
    <commonimage:objectiveLens id="OBJ"><opticalelement:name>UPLXAPO20X</opticalelement:name><opticalelement:magnification>20.0</opticalelement:magnification><opticalelement:naValue>0.8</opticalelement:naValue></commonimage:objectiveLens>
    <lsmimage:imagingMainLaser enable="true" id="L488"><commonimage:wavelength>488</commonimage:wavelength></lsmimage:imagingMainLaser>
    <lsmimage:imagingMainLaser enable="false" id="L561"><commonimage:wavelength>561</commonimage:wavelength></lsmimage:imagingMainLaser>
  </commonimage:acquisition>
</lsmimage:imageProperties>"""


def channel_xml(uid, order, name):
    return (f'<commonphase:channel id="{uid}" order="{order}"><commonphase:name>{name}</commonphase:name>'
            f'<commonphase:laserDataId>L488</commonphase:laserDataId><commonphase:laserDataId>L561</commonphase:laserDataId></commonphase:channel>')


def lsm_channel(uid, dye):
    return (f'<?xml version="1.0" encoding="ASCII"?>\n<lsmimage:lsmChannel xmlns:lsmimage="l" xmlns:commonphase="cp" id="{uid}" order="1">'
            f'<commonphase:dyeName>{dye}</commonphase:dyeName></lsmimage:lsmChannel>')


def frame_xml(z, t):
    return (f'<?xml version="1.0" encoding="ASCII"?>\n<lsmframe:frameProperties xmlns:lsmframe="f" xmlns:commonframe="c" xmlns:commonparam="p">'
            f'<commonframe:imageDefinition><commonparam:width>{W}</commonparam:width><commonparam:height>{H}</commonparam:height>'
            f'<commonparam:depth>2</commonparam:depth><commonparam:bitCounts>12</commonparam:bitCounts><commonparam:colorType>GlayScale</commonparam:colorType></commonframe:imageDefinition>'
            f'<commonframe:axisValue><commonparam:axisType>TIMELAPSE</commonparam:axisType><commonparam:position>{2500.0 * t + 100.0 * z}</commonparam:position></commonframe:axisValue>'
            f'<commonframe:axisValue><commonparam:axisType>ZSTACK</commonparam:axisType><commonparam:position>{10.0 + 1.5 * z}</commonparam:position></commonframe:axisValue>'
            f'</lsmframe:frameProperties>')


def oir_file(blocks):
    data = bytearray(bytes(96))
    offsets = []
    for b in blocks:
        offsets.append(len(data))
        data += b
    index = len(data)
    data += struct.pack("<I", 0xFFFFFFFF) + b"".join(struct.pack("<Q", o) for o in offsets)
    header = b"OLYMPUSRAWFORMAT" + struct.pack("<4I", 12, 0, 1, 2) + struct.pack("<QQI3I", len(data), index, len(offsets), 0, 1, 0)
    header += struct.pack("<Q", 0xFFFFFFFFFFFFFFFF) + b"FLUOVIEW" + struct.pack("<IIQ", 3, 2, 0xFFFFFFFFFFFFFFFF)
    assert len(header) == 96
    data[:96] = header
    return bytes(data)


def frame_blocks(z, t):
    """Chunk tags and pixels of both channels, interleaved, then frame properties."""
    out = []
    planes = {ci: plane_bytes(ci, z, t) for ci in range(len(CHANNELS))}
    step = ROWS_PER_CHUNK * W * 2
    for n, at in enumerate(range(0, W * H * 2, step)):
        for ci, (uid, _, _) in enumerate(CHANNELS):
            chunk = planes[ci][at:at + step]
            name = f"z{z + 1:03d}t{t + 1:03d}_0_1_{uid}_{n}".encode()
            out.append(block(3, struct.pack("<III", at, len(chunk), len(name)) + name))
            out.append(block(4, chunk))
    out.append(block(1, bytes(12) + frame_xml(z, t).encode()))
    out.append(block(5, b""))
    return out


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    channels = "".join(channel_xml(*c) for c in CHANNELS)
    docs = ['<?xml version="1.0" encoding="ASCII"?>\n<fileinfo:fileInfomation xmlns:fileinfo="f"><fileinfo:version>2.1.2.3</fileinfo:version></fileinfo:fileInfomation>',
            IMAGE_PROPERTIES.replace("{channels}", channels)] + [lsm_channel(u, "DAPI" if o == 1 else "GFP") for u, o, _ in CHANNELS]
    main_blocks = [block(0, documents(docs))]
    for z in range(Z):
        main_blocks += frame_blocks(z, 0)
    (OUT / "synthetic.oir").write_bytes(oir_file(main_blocks))
    cont = []
    for z in range(Z):
        cont += frame_blocks(z, 1)
    (OUT / "synthetic_00001").write_bytes(oir_file(cont))

    # Truth: planes t, z; channels in `order` (CH1 = second uuid first), interleaved per pixel.
    order = sorted(range(len(CHANNELS)), key=lambda i: CHANNELS[i][1])
    px = bytearray()
    for t in range(T):
        for z in range(Z):
            for y in range(H):
                for x in range(W):
                    for ci in order:
                        px += struct.pack("=H", value(x, y, ci, z, t))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        f.write(f"synthetic.oir\t{zlib.crc32(bytes(px)):08x}\n")


if __name__ == "__main__":
    main()

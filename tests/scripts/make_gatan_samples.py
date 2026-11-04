#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Gatan DigitalMicrograph files.

Layout from C. Boothroyd's DM3/DM4 description and the OpenReadout DM notes
(doc/gatan-dm/): header (version, root length, value byte order), tag
groups (0x14: sorted, open, count) and data tags (0x15: name, %%%%, info
words, values); structure words big endian; DM4 with 8-byte counts and
info words and a byte length per tag. Images in ImageList with ImageData
(DataType, Dimensions, Calibrations.Dimension, Data) and Name; entries
named in Thumbnails[*].ImageIndex are thumbnails.

- stack.dm3: DM3, little-endian values: entry 0 a uint8 thumbnail, entry 1
  int16 7 x 5 x 3 with nm calibrations (z planes), entry 2 a float32
  spectrum image 7 x 5 x 4 with an eV third axis (channels).
- rgba.dm4: DM4, big-endian values: an RGBA (type 23) 7 x 5 image and a
  uint16 sequence 7 x 5 x 2 (IsSequence, time points).

Also writes truth.tsv with the CRC-32 of each series in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/gatan/ is
committed.

Usage: python3 -I tests/scripts/make_gatan_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "gatan"
W, H = 7, 5
TYPES = {"i2": 2, "i4": 3, "u2": 4, "u4": 5, "f4": 6, "f8": 7, "u1": 10}
FMT = {"i2": "h", "i4": "i", "u2": "H", "u4": "I", "f4": "f", "f8": "d", "u1": "B"}


class Writer:
    def __init__(self, dm4, little):
        self.dm4, self.e = dm4, "<" if little else ">"

    def word(self, v):
        return struct.pack(">Q" if self.dm4 else ">I", v)

    def tag(self, name, body, kind):
        n = name.encode()
        head = bytes([kind]) + struct.pack(">H", len(n)) + n
        return head + (struct.pack(">Q", len(body)) if self.dm4 else b"") + body

    def group(self, name, entries):
        body = bytes([1, 0]) + self.word(len(entries)) + b"".join(entries)
        return self.tag(name, body, 0x14)

    def scalar(self, name, t, v):
        body = b"%%%%" + self.word(1) + self.word(TYPES[t]) + struct.pack(self.e + FMT[t], v)
        return self.tag(name, body, 0x15)

    def text(self, name, s):
        u = s.encode("utf-16-le" if self.e == "<" else "utf-16-be")
        body = b"%%%%" + self.word(3) + self.word(20) + self.word(4) + self.word(len(u) // 2) + u
        return self.tag(name, body, 0x15)

    def array(self, name, t, raw, count):
        body = b"%%%%" + self.word(3) + self.word(20) + self.word(TYPES[t]) + self.word(count) + raw
        return self.tag(name, body, 0x15)

    def image(self, name, data_type, dims, raw, elem_t, units, tags=()):
        cals = [self.group("", [self.scalar("Origin", "f4", 0.0), self.scalar("Scale", "f4", s), self.text("Units", u)]) for s, u in units]
        data = self.group("ImageData", [
            self.group("Calibrations", [self.group("Dimension", cals)]),
            self.array("Data", elem_t, raw, len(raw) // struct.calcsize(FMT[elem_t])),
            self.scalar("DataType", "i4", data_type),
            self.group("Dimensions", [self.scalar("", "u4", d) for d in dims]),
        ])
        return self.group("", [data, self.group("ImageTags", list(tags)), self.text("Name", name)])

    def file(self, images, thumbs):
        root = bytes([1, 0]) + self.word(2) + self.group("ImageList", images) + \
            self.group("Thumbnails", [self.group("", [self.scalar("ImageIndex", "u4", t)]) for t in thumbs])
        version = 4 if self.dm4 else 3
        head = struct.pack(">I", version) + (struct.pack(">Q", len(root)) if self.dm4 else struct.pack(">I", len(root)))
        head += struct.pack(">I", 1 if self.e == "<" else 0)
        return head + root + bytes(8)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    w = Writer(False, True)
    thumb = w.image("thumb", 6, [4, 4], bytes(range(16)), "u1", [(1, ""), (1, "")])
    v = lambda x, y, z: x * 301 - y * 77 + z * 1000
    raw = b"".join(struct.pack("<h", v(x, y, z)) for z in range(3) for y in range(H) for x in range(W))
    stack = w.image("z stack", 1, [W, H, 3], raw, "i2", [(0.5, "nm"), (0.5, "nm"), (2.0, "nm")])
    f = lambda x, y, c: x * 0.5 + y * 0.25 + c * 10.0
    raw2 = b"".join(struct.pack("<f", f(x, y, c)) for c in range(4) for y in range(H) for x in range(W))
    si = w.image("spectrum image", 2, [W, H, 4], raw2, "f4", [(1.5, "µm"), (1.5, "µm"), (0.25, "eV")],
                 [w.group("Meta Data", [w.text("Format", "Spectrum image")])])
    (OUT / "stack.dm3").write_bytes(w.file([thumb, stack, si], [0]))
    truth.append(("stack.dm3", 0, zlib.crc32(raw)))
    truth.append(("stack.dm3", 1, zlib.crc32(b"".join(struct.pack("<f", f(x, y, c)) for y in range(H) for x in range(W) for c in range(4)))))

    w = Writer(True, False)
    rgba = bytes(b for y in range(H) for x in range(W) for b in ((x * 30) % 256, (y * 50) % 256, (x + y) % 256, 255))
    img = w.image("colour", 23, [W, H], rgba, "u1", [(1, ""), (1, "")])
    u = lambda x, y, t: (x * 1000 + y * 13 + t * 7) % 65536
    raw3 = b"".join(struct.pack(">H", u(x, y, t)) for t in range(2) for y in range(H) for x in range(W))
    seq = w.image("sequence", 10, [W, H, 2], raw3, "u2", [(3.0, "Å"), (3.0, "Å"), (1.0, "nm")],
                  [w.group("Meta Data", [w.scalar("IsSequence", "u1", 1)])])
    (OUT / "rgba.dm4").write_bytes(w.file([img, seq], []))
    truth.append(("rgba.dm4", 0, zlib.crc32(rgba)))
    truth.append(("rgba.dm4", 1, zlib.crc32(b"".join(struct.pack("<H", u(x, y, t)) for t in range(2) for y in range(H) for x in range(W)))))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tseries\tcrc32\n")
        for name, s, crc in truth:
            out.write(f"{name}\t{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

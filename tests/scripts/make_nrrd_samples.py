#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic NRRD files.

Layout from the Teem NRRD file format definition (doc/nrrd/):

- scalar.nrrd: attached header, raw little-endian ushort, sizes 7 5 3,
  kinds space space space, spacings 0.5 0.5 2 (um): one channel, 3 z.
- rgb.nhdr + rgb.raw.gz: detached header, gzip, uchar, sizes 3 7 5,
  kinds RGB-color space space, space directions none (0.25,0,0)
  (0,0.25,0): three channels.
- text.nrrd: ascii short, sizes 2 7 5 4, kinds list space space time,
  spacings NaN 1 1 2 with units "" "um" "um" "s": 2 channels, 4 times.
- hex.nhdr + hex.txt: hex big-endian float, sizes 7 5, line skip 2 and
  byte skip 3 in the data file.
- tail.nhdr + tail.bin: raw big-endian uint, sizes 7 5, byte skip -1
  (samples at the end of the file after a junk header).

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/nrrd/ is
committed.

Usage: python3 -I tests/scripts/make_nrrd_samples.py
"""

import gzip
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "nrrd"
W, H = 7, 5


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # Scalar volume: x fastest, then y, z.
    v = [(x * 1000 + y * 77 + z * 9000) % 65536 for z in range(3) for y in range(H) for x in range(W)]
    raw = struct.pack(f"<{len(v)}H", *v)
    head = b"NRRD0004\n# scalar volume\ntype: ushort\ndimension: 3\nsizes: 7 5 3\nkinds: space space space\n" \
           b"spacings: 0.5 0.5 2\nunits: \"um\" \"um\" \"um\"\nendian: little\nencoding: raw\nlabel:=synthetic\n\n"
    (OUT / "scalar.nrrd").write_bytes(head + raw)
    truth.append(("scalar.nrrd", zlib.crc32(raw)))

    # RGB, channels interleaved (fastest axis).
    rgb = bytes(((x * 30 + c * 80) + y * 11) % 256 for y in range(H) for x in range(W) for c in range(3))
    (OUT / "rgb.raw.gz").write_bytes(gzip.compress(rgb, mtime=0))
    (OUT / "rgb.nhdr").write_bytes(b"NRRD0005\ntype: uchar\ndimension: 3\nsizes: 3 7 5\nkinds: RGB-color space space\n"
                                   b"space dimension: 2\nspace directions: none (0.25,0) (0,0.25)\nspace units: \"um\" \"um\"\n"
                                   b"encoding: gzip\ndata file: rgb.raw.gz\n")
    truth.append(("rgb.nhdr", zlib.crc32(rgb)))

    # Text: channel fastest, then x, y, time.
    f = lambda c, x, y, t: (x * 100 - y * 300 + c * 7 + t * 1000)
    vals = [f(c, x, y, t) for t in range(4) for y in range(H) for x in range(W) for c in range(2)]
    lines = "\n".join(" ".join(str(v) for v in vals[i:i + 14]) for i in range(0, len(vals), 14))
    (OUT / "text.nrrd").write_bytes(b"NRRD0004\ntype: short\ndimension: 4\nsizes: 2 7 5 4\nkinds: list space space time\n"
                                    b"spacings: NaN 1 1 2\nunits: \"\" \"um\" \"um\" \"s\"\nencoding: ascii\n\n" + lines.encode() + b"\n")
    out = b"".join(struct.pack("<h", f(c, x, y, t)) for t in range(4) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("text.nrrd", zlib.crc32(out)))

    # Hex after two lines and three bytes of junk.
    fv = [x * 0.5 - y * 1.25 for y in range(H) for x in range(W)]
    be = struct.pack(f">{len(fv)}f", *fv)
    hexed = "\n".join(be[i:i + 35].hex() for i in range(0, len(be), 35))
    (OUT / "hex.txt").write_bytes(b"junk line one\njunk line two\nXYZ" + hexed.encode() + b"\n")
    (OUT / "hex.nhdr").write_bytes(b"NRRD0004\ntype: float\ndimension: 2\nsizes: 7 5\nendian: big\nencoding: hex\n"
                                   b"line skip: 2\nbyte skip: 3\ndata file: ./hex.txt\n")
    truth.append(("hex.nhdr", zlib.crc32(struct.pack(f"<{len(fv)}f", *fv))))

    # Samples at the end of a file with a header of unknown length.
    uv = [x * 100000 + y * 7 for y in range(H) for x in range(W)]
    (OUT / "tail.bin").write_bytes(b"some other header\0\1\2" + struct.pack(f">{len(uv)}I", *uv))
    (OUT / "tail.nhdr").write_bytes(b"NRRD0004\ntype: uint\ndimension: 2\nsizes: 7 5\nendian: big\nencoding: raw\n"
                                    b"byte skip: -1\ndata file: tail.bin\n\n")
    truth.append(("tail.nhdr", zlib.crc32(struct.pack(f"<{len(uv)}I", *uv))))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic ICS files.

Layout from the ICS proposal (Dean et al. 1990) and the libics
documentation (doc/ics/): first line the field and record separators, then
`category subcategory values` lines; `layout order` (after `bits`) fastest
first, `representation byte_order` as byte positions; ICS 1.0 keeps the
samples in the .ids file, ICS 2.0 after the `end` line.

- be_u16.ics/.ids: ICS 1.0, uint16 big endian (byte_order 2 1), order
  x y z ch l (l, an unnamed extra dimension, adds channels after ch),
  7 x 5 x 3 x 2 x 2, 0.25 x 0.25 x 1 um, cartesian coordinates (rows
  stored bottom-up).
- ch_first.ics: ICS 2.0, float32 little endian, order ch x y t with 3
  channels fastest, 7 x 5, 2 time points of 1.5 s.
- gzip.ics: ICS 2.0, int8, gzip-compressed, order x y p (2 probes).

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()` (channels interleaved, then x, y, z,
t). Output in tests/data/ics/ is committed.

Usage: python3 -I tests/scripts/make_ics_samples.py
"""

import gzip
import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "ics"
W, H = 7, 5


def header(lines):
    return ("\t\n" + "".join("\t".join(map(str, l)) + "\n" for l in lines)).encode()


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    nz, nc, nl = 3, 2, 2
    v = lambda x, y, z, c, l: x * 401 + y * 37 + z * 3000 + c * 9000 + l * 20000
    data = b"".join(struct.pack(">H", v(x, H - 1 - y, z, c, l)) for l in range(nl) for c in range(nc) for z in range(nz) for y in range(H) for x in range(W))
    (OUT / "be_u16.ics").write_bytes(header([
        ["ics_version", "1.0"], ["filename", "be_u16"], ["layout", "parameters", 6],
        ["layout", "order", "bits", "x", "y", "z", "ch", "l"], ["layout", "sizes", 16, W, H, nz, nc, nl], ["layout", "coordinates", "cartesian"],
        ["representation", "format", "integer"], ["representation", "sign", "unsigned"],
        ["representation", "compression", "uncompressed"], ["representation", "byte_order", 2, 1],
        ["parameter", "scale", 1, 0.25, 0.25, 1.0, 1, 1], ["parameter", "units", "bits", "micrometers", "micrometers", "micrometers", "undefined", "undefined"],
        ["sensor", "s_params", "LambdaEx", 488, 561], ["sensor", "s_params", "NumAperture", 1.4]]))
    (OUT / "be_u16.ids").write_bytes(data)
    truth.append(("be_u16.ics", zlib.crc32(b"".join(struct.pack("<H", v(x, y, z, c, l)) for z in range(nz) for y in range(H) for x in range(W) for l in range(nl) for c in range(nc)))))

    f = lambda c, x, y, t: c * 100.0 + x * 0.5 - y * 1.25 + t * 10
    data = b"".join(struct.pack("<f", f(c, x, y, t)) for t in range(2) for y in range(H) for x in range(W) for c in range(3))
    (OUT / "ch_first.ics").write_bytes(header([
        ["ics_version", "2.0"], ["filename", "ch_first"], ["layout", "parameters", 5],
        ["layout", "order", "bits", "ch", "x", "y", "t"], ["layout", "sizes", 32, 3, W, H, 2],
        ["representation", "format", "real"], ["representation", "sign", "signed"],
        ["representation", "compression", "uncompressed"], ["representation", "byte_order", 1, 2, 3, 4],
        ["parameter", "scale", 1, 1, 2.0, 2.0, 1.5], ["parameter", "units", "bits", "undefined", "nm", "nm", "seconds"], ["end"]]) + data)
    truth.append(("ch_first.ics", zlib.crc32(data)))

    i8 = lambda x, y, p: (x * 20 - y * 30 + p * 50 + 128) % 256 - 128
    data = b"".join(struct.pack("<b", i8(x, y, p)) for p in range(2) for y in range(H) for x in range(W))
    (OUT / "gzip.ics").write_bytes(header([
        ["ics_version", "2.0"], ["filename", "gzip"], ["layout", "parameters", 4],
        ["layout", "order", "bits", "x", "y", "p"], ["layout", "sizes", 8, W, H, 2],
        ["representation", "format", "integer"], ["representation", "sign", "signed"],
        ["representation", "compression", "gzip"], ["representation", "byte_order", 1], ["end"]]) + gzip.compress(data, mtime=0))
    truth.append(("gzip.ics", zlib.crc32(b"".join(struct.pack("<b", i8(x, y, p)) for y in range(H) for x in range(W) for p in range(2)))))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

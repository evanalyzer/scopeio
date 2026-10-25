#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Olympus scan^R experiment.

Layout from the public scan^R plate of IDR study idr0009 (CC BY 4.0):
`experiment_descriptor.xml` is LabVIEW XML (`<LVData>`, typed elements with
`Name` / `Val` in `Cluster` and `Array` elements) with the channel
configurations (exposure time in ms before the name), positions per well
(`columns/well`, `rows/well`), `# slices`, `timeloop count`, the plate name and a
well selection table (one well number and one cDNA text per well of the
plate); `data/` holds one TIFF per plane named
`--W<well>--P<position>--Z<z>--T<t>--<channel>.tif`, wells numbered from 1
row by row.

- plate/: 96-well plate, wells 1 (A01) and 14 (B02), two positions each,
  two channels, Z 2, T 2, uint16 9 x 7; the plane W14 P2 Z1 T1 of the
  second channel is missing (reads as zeros).

Also writes truth.tsv with the CRC-32 of each series' pixels in the layout
of tests/common/mod.rs `decode_level()`. Output in tests/data/scanr/ is
committed.

Usage: python3 -I tests/scripts/make_scanr_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "scanr"
W, H = 9, 7
CHANNELS = [("dapi", 120.0), ("gfp", 250.0)]
WELLS, POSITIONS, NZ, NT = [1, 14], [1, 2], 2, 2
MISSING = (14, 2, 1, 1, "gfp")


def element(tag, name, value):
    return f"<{tag}>\n<Name>{name}</Name>\n<Val>{value}</Val>\n</{tag}>\n"


def descriptor():
    channels = "".join(
        "<Cluster>\n<Name>channel config</Name>\n<NumElts>2</NumElts>\n"
        + element("DBL", "exposure time", f"{ms:.5f}") + element("String", "name", name) + "</Cluster>\n"
        for name, ms in CHANNELS)
    table = "".join(element("String", "", str(w)) + element("String", "", "") for w in range(1, 97))
    return ("<?xml version='1.0' standalone='yes' ?>\n<LVData>\n<Version>7.0</Version>\n"
            "<Cluster>\n<Name>Preferences</Name>\n<NumElts>10</NumElts>\n"
            + element("I16", "columns/well", len(POSITIONS)) + element("U16", "rows/well", 1)
            + element("I32", "custom column", 12) + element("I32", "custom row", 8)
            + element("U32", "timeloop count", NT) + element("U16", "# slices", NZ)
            + element("DBL", "slices distance", "1.50000")
            + f"<Array>\n<Name>multiple_channel_typedef</Name>\n<Dimsize>{len(CHANNELS)}</Dimsize>\n{channels}</Array>\n"
            + f"<Array>\n<Name>well selection table + cDNA</Name>\n<Dimsize>192</Dimsize>\n{table}</Array>\n"
            + element("String", "plate name", "synthetic plate")
            + "</Cluster>\n</LVData>\n")


def tiff(samples):
    data = struct.pack(f"<{len(samples)}H", *samples)
    out = bytearray(b"II*\0" + struct.pack("<I", 8 + len(data)) + data)
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(data))]
    out += struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    return bytes(out)


def value(x, y, w, p, z, t, c):
    return (x * 300 + y * 40 + w * 1000 + p * 5000 + z * 7000 + t * 11000 + c * 20000) % 65536


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    plate = OUT / "plate"
    (plate / "data").mkdir(parents=True)
    (plate / "experiment_descriptor.xml").write_text(descriptor())
    truth = []
    for w in WELLS:
        for p in POSITIONS:
            px = bytearray()
            for t in range(NT):
                for z in range(NZ):
                    for y in range(H):
                        for x in range(W):
                            for c, (name, _) in enumerate(CHANNELS):
                                v = 0 if (w, p, z, t, name) == MISSING else value(x, y, w, p, z, t, c)
                                px += struct.pack("<H", v)
            for t in range(NT):
                for z in range(NZ):
                    for c, (name, _) in enumerate(CHANNELS):
                        if (w, p, z, t, name) == MISSING:
                            continue
                        samples = [value(x, y, w, p, z, t, c) for y in range(H) for x in range(W)]
                        (plate / "data" / f"--W{w:05}--P{p:05}--Z{z:05}--T{t:05}--{name}.tif").write_bytes(tiff(samples))
            truth.append((f"W{w}P{p}", zlib.crc32(bytes(px))))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("series\tcrc32\n")
        for name, crc in truth:
            f.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

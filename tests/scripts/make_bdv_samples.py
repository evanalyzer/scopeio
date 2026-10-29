#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic BigDataViewer XML/HDF5 dataset.

HDF5 structures from the writer of make_ims_samples.py (HDF5 File Format
Specification); BigDataViewer layout from doc/bdv/ and the public samples:
`t<tp:05>/s<setup:02>/<level>/cells` (z, y, x) int16 volumes holding
unsigned values, `s<setup:02>/resolutions` (float64 x, y, z factors).

Four setups: channel 0/1 x angle 0/1, so two series of two channels. Time
points listed as "0, 2". Two levels: level 0 (4 x 7 x 9) chunked (2, 4, 4)
with the scale-offset (integer, minimum bits) and deflate filters, level 1
(2 x 4 x 5) contiguous. The view of time point 2, setup 3 is missing (XML
MissingViews, absent from the HDF5 file) and reads as zeros.

Also writes truth.tsv with the CRC-32 of each series and level in the
layout of tests/common/mod.rs `decode_level()` (channels interleaved, then
x, y, z, t; every level has the z planes of level 0, each the nearest
stored one). Output in tests/data/bdv/ is committed.

Usage: python3 -I tests/scripts/make_bdv_samples.py
"""

import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_ims_samples import H5  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "bdv"
SIZES = [(4, 7, 9), (2, 4, 5)]  # z, y, x per level
TIMEPOINTS = [0, 2]
MISSING = (2, 3)  # (time point id, setup)


class SignedH5(H5):
    """Writes datasets as int16, as BigDataViewer does."""

    def datatype(self, kind, size):
        if kind == "u":
            return struct.pack("<B3sI", 0x10, bytes([0x08, 0, 0]), size) + struct.pack("<HH", 0, 8 * size)
        return H5.datatype(kind, size)


def value(setup, t, z, y, x, level):
    # Values above 32767 check that int16 data are read as unsigned.
    return (40000 + setup * 3000 + t * 700 + z * 90 + y * 11 + x * 5 + level * 17) % 65536


def scale_offset(values):
    """Scale-offset filter output for unsigned values: minimum bits, no fill value."""
    lo = min(values)
    bits = (max(values) - lo).bit_length()
    head = struct.pack("<IB", bits, 8) + struct.pack("<Q", lo)
    head += bytes(21 - len(head))
    if bits == 16:
        # Full precision: the values are stored as they are.
        return head + struct.pack(f"<{len(values)}H", *values)
    acc, n = 0, 0
    for v in values:
        acc = (acc << bits) | (v - lo)
        n += bits
    n_bytes = -(-n // 8)
    packed = (acc << (n_bytes * 8 - n)).to_bytes(n_bytes, "big") if n else b""
    return head + packed


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    h = SignedH5()
    setups = [(c + 2 * a, c, a) for a in range(2) for c in range(2)]
    tp_groups = []
    for ti, tp in enumerate(TIMEPOINTS):
        s_groups = []
        for s, _, _ in setups:
            if (tp, s) == MISSING:
                continue
            levels = []
            for lv, (Z, Y, X) in enumerate(SIZES):
                vol = lambda z, y, x: value(s, ti, z, y, x, lv)
                if lv == 0:
                    cz, cy, cx = 2, 4, 4
                    chunks = {}
                    for gz in range(0, Z, cz):
                        for gy in range(0, Y, cy):
                            for gx in range(0, X, cx):
                                vals = [vol(z, y, x) if z < Z and y < Y and x < X else 0
                                        for z in range(gz, gz + cz) for y in range(gy, gy + cy) for x in range(gx, gx + cx)]
                                chunks[(gz, gy, gx)] = zlib.compress(scale_offset(vals))
                    so = [2, 0, cz * cy * cx, 0, 2, 1, 0, 0] + [0] * 12
                    data = h.chunked([Z, Y, X], [cz, cy, cx], 2, chunks, [(6, so), (1, [6])])
                else:
                    raw = b"".join(struct.pack("<H", vol(z, y, x)) for z in range(Z) for y in range(Y) for x in range(X))
                    data = h.contiguous([Z, Y, X], "u", 2, raw)
                levels.append((str(lv), h.group([("cells", data)])))
            s_groups.append((f"s{s:02d}", h.group(levels)))
        tp_groups.append((f"t{tp:05d}", h.group(s_groups)))
    res = b"".join(struct.pack("<3d", f, f, f) for f in (1.0, 2.0))
    setup_info = [(f"s{s:02d}", h.group([("resolutions", h.contiguous([2, 3], "f", 8, res))])) for s, _, _ in setups]
    (OUT / "synthetic.h5").write_bytes(h.finish(h.group(tp_groups + setup_info)))

    view_setups = "".join(f"""
      <ViewSetup>
        <id>{s}</id>
        <name>setup {s}</name>
        <size>9 7 4</size>
        <voxelSize>
          <unit>um</unit>
          <size>0.5 0.5 2.0</size>
        </voxelSize>
        <attributes>
          <illumination>0</illumination>
          <channel>{c}</channel>
          <angle>{a}</angle>
        </attributes>
      </ViewSetup>""" for s, c, a in setups)
    (OUT / "synthetic.xml").write_text(f"""<?xml version="1.0" encoding="UTF-8"?>
<SpimData version="0.2">
  <BasePath type="relative">.</BasePath>
  <SequenceDescription>
    <ImageLoader format="bdv.hdf5">
      <hdf5 type="relative">synthetic.h5</hdf5>
    </ImageLoader>
    <ViewSetups>{view_setups}
      <Attributes name="channel">
        <Channel><id>0</id><name>DAPI</name></Channel>
        <Channel><id>1</id><name>GFP</name></Channel>
      </Attributes>
      <Attributes name="angle">
        <Angle><id>0</id><name>0</name></Angle>
        <Angle><id>1</id><name>90</name></Angle>
      </Attributes>
    </ViewSetups>
    <Timepoints type="list">
      <integerpattern>0, 2</integerpattern>
    </Timepoints>
    <MissingViews>
      <MissingView timepoint="{MISSING[0]}" setup="{MISSING[1]}" />
    </MissingViews>
  </SequenceDescription>
</SpimData>
""")

    with open(OUT / "truth.tsv", "w") as f:
        f.write("series\tlevel\tcrc32\n")
        Z0 = SIZES[0][0]
        for a in range(2):
            for lv, (Z, Y, X) in enumerate(SIZES):
                out = bytearray()
                for ti, tp in enumerate(TIMEPOINTS):
                    for z0 in range(Z0):
                        z = z0 * Z // Z0
                        for y in range(Y):
                            for x in range(X):
                                for c in range(2):
                                    s = c + 2 * a
                                    v = 0 if (tp, s) == MISSING else value(s, ti, z, y, x, lv)
                                    out += struct.pack("<H", v)
                f.write(f"{a}\t{lv}\t{zlib.crc32(bytes(out)):08x}\n")


if __name__ == "__main__":
    main()

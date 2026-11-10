#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Bruker Prairie View recordings.

Layout from own analysis of the CatalystNeuro Bruker test recordings: a
folder with a PVScan XML file (global PVStateShard, one Sequence per
cycle, a File per channel in each Frame) and uint16 TIFF files. Every
pixel of a plane holds 1000 * t + 10 * z + channel number.

- zseries/: "TSeries ZSeries Element", 2 cycles of 3 frames, channels 1
  and 2, bidirectional z (the second cycle lists its frames from the
  deepest up), z positions 10, 12, 14 um, 0.5 um pixels, one file per
  plane.
- timed/: one "TSeries Timed Element" of 4 frames, channels 2 and 3, one
  multi-page file per channel (page attribute, Prairie View 5.8).
- multiseq/: two "TSeries Timed Element" sequences of 2 frames, channel 2;
  frames of all sequences are time points.
- missing/: one timed sequence of 3 frames, channel 1; the file of the
  second frame is missing and reads as zeros.

Also writes truth.tsv (folder, channels x z x t, channel numbers, missing
time point). Output in tests/data/prairie/ is committed.

Usage: python3 -I tests/scripts/make_prairie_samples.py
"""

import shutil
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "prairie"
W, H = 6, 4


def tiff(pages):
    """Classic little-endian TIFF, uint16, one strip per page."""
    data = bytearray(b"II*\0" + bytes(4))
    prev = 4
    for value in pages:
        px = struct.pack(f"<{W * H}H", *([value] * (W * H)))
        at = len(data)
        data += px
        entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
                   (273, 4, 1, at), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(px))]
        ifd = len(data)
        struct.pack_into("<I", data, prev, ifd)
        data += struct.pack("<H", len(entries))
        for e in entries:
            data += struct.pack("<HHII", *e)
        prev = len(data)
        data += bytes(4)
    return bytes(data)


GLOBAL = """  <PVStateShard>
    <PVStateValue key="framePeriod" value="0.25"/>
    <PVStateValue key="micronsPerPixel">
      <IndexedValue index="XAxis" value="0.5"/>
      <IndexedValue index="YAxis" value="0.5"/>
      <IndexedValue index="ZAxis" value="2"/>
    </PVStateValue>
    <PVStateValue key="objectiveLens" value="Nikon 16X"/>
    <PVStateValue key="objectiveLensMag" value="16"/>
    <PVStateValue key="objectiveLensNA" value="0.8"/>
    <PVStateValue key="laserWavelength">
      <IndexedValue index="0" value="920" description="Excitation 1"/>
    </PVStateValue>
  </PVStateShard>
"""


def frame(index, time, files, z=None):
    pos = ""
    if z is not None:
        pos = ("      <PVStateShard>\n        <PVStateValue key=\"positionCurrent\">\n"
               "          <SubindexedValues index=\"ZAxis\">\n"
               f"            <SubindexedValue subindex=\"0\" value=\"{z}\" description=\"Z Focus\"/>\n"
               "          </SubindexedValues>\n        </PVStateValue>\n      </PVStateShard>\n")
    fl = "".join(f'      <File channel="{c}" channelName="Ch{c}" filename="{name}"{f" page=\"{p}\"" if p else ""}/>\n'
                 for c, name, p in files)
    return f'    <Frame relativeTime="{time}" absoluteTime="{time + 3.5}" index="{index}" parameterSet="CurrentSettings">\n{fl}{pos}    </Frame>\n'


def write(name, sequences):
    d = OUT / name
    d.mkdir(parents=True)
    body = "".join(f'  <Sequence type="{t}" cycle="{i + 1}"{extra}>\n{"".join(frames)}  </Sequence>\n'
                   for i, (t, extra, frames) in enumerate(sequences))
    xml = f'<?xml version="1.0" encoding="utf-8"?>\n<PVScan version="5.8.64.800" date="2/2/2026 3:04:05 PM" notes="">\n{GLOBAL}{body}</PVScan>\n'
    (d / f"{name}.xml").write_text(xml)
    return d


def zseries():
    d = OUT / "zseries"
    seqs = []
    for t in range(2):
        frames = []
        order = [0, 1, 2] if t == 0 else [2, 1, 0]
        for i, z in enumerate(order):
            files = []
            for c in (1, 2):
                fn = f"zseries_Cycle{t + 1:05d}_Ch{c}_{i + 1:06d}.tif"
                files.append((c, fn, None))
            frames.append(frame(i + 1, t * 2.0 + i * 0.25, files, 10 + 2 * z))
        seqs.append(("TSeries ZSeries Element", ' bidirectionalZ="True"', frames, order))
    d = write("zseries", [(a, b, c) for a, b, c, _ in seqs])
    for t, (_, _, _, order) in enumerate(seqs):
        for i, z in enumerate(order):
            for c in (1, 2):
                (d / f"zseries_Cycle{t + 1:05d}_Ch{c}_{i + 1:06d}.tif").write_bytes(tiff([1000 * t + 10 * z + c]))
    return "zseries", "2x3x2", "1,2", "-"


def timed():
    frames = [frame(i + 1, i * 0.25, [(c, f"timed_Cycle00001_Ch{c}_000001.ome.tif", i + 1) for c in (2, 3)]) for i in range(4)]
    d = write("timed", [("TSeries Timed Element", "", frames)])
    for c in (2, 3):
        (d / f"timed_Cycle00001_Ch{c}_000001.ome.tif").write_bytes(tiff([1000 * t + c for t in range(4)]))
    return "timed", "2x1x4", "2,3", "-"


def multiseq():
    seqs = []
    for s in range(2):
        frames = [frame(i + 1, s * 10.0 + i * 0.25, [(2, f"multiseq_Cycle{s + 1:05d}_Ch2_{i + 1:06d}.tif", None)]) for i in range(2)]
        seqs.append(("TSeries Timed Element", "", frames))
    d = write("multiseq", seqs)
    for s in range(2):
        for i in range(2):
            (d / f"multiseq_Cycle{s + 1:05d}_Ch2_{i + 1:06d}.tif").write_bytes(tiff([1000 * (2 * s + i) + 2]))
    return "multiseq", "1x1x4", "2", "-"


def missing():
    frames = [frame(i + 1, i * 0.25, [(1, f"missing_Cycle00001_Ch1_{i + 1:06d}.tif", None)]) for i in range(3)]
    d = write("missing", [("TSeries Timed Element", "", frames)])
    for i in (0, 2):
        (d / f"missing_Cycle00001_Ch1_{i + 1:06d}.tif").write_bytes(tiff([1000 * i + 1]))
    return "missing", "1x1x3", "1", "1"


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    rows = [zseries(), timed(), multiseq(), missing()]
    with open(OUT / "truth.tsv", "w") as f:
        f.write("folder\tc_z_t\tchannels\tmissing_t\n")
        for r in rows:
            f.write("\t".join(r) + "\n")


if __name__ == "__main__":
    main()

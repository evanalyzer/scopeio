#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic ScanImage TIFF files.

Layouts from the ScanImage BigTiff Specification (doc/scanimage/) and own
analysis of the CatalystNeuro ScanImage test files: pages in the order
volume, slice (flyback frames included), frame, channel; int16 pixels.
Every pixel of a page holds 1000 * t + 10 * z + channel, where t is the
time point the reader returns (volume * frames per slice + frame), z the
slice and channel the ScanImage channel number; flyback frames hold -2,
the fly-to rows between regions of interest -1, and region r adds
10000 * r.

- v4_fast_flyback.tif: format 4 BigTIFF header, channels [1 3], fast z of
  3 slices plus 2 flyback frames, 2 volumes, 8 x 6, imagingFovUm
  [-40 -30; 40 -30; 40 30; -40 30] um over 8 x 6 pixels (10 um pixels).
- v3_mroi.tif: format 3 header with ROI groups, channel 2, planar,
  3 frames; ROIs of 8 x 4, 8 x 6 and 8 x 4 pixels stacked with 3 fly-to
  rows, objective resolution 10 um/deg, ROI scan fields 1.6 deg wide.
- f1_slow.tif: format 1 (classic TIFF, scanimage.SI.* in every page),
  channel 1, motor stack of 3 slices with 2 frames saved per slice.
- f2_json.tif: format 2 (BigTIFF, settings as JSON-like text in the first
  page), channels [1 2], planar, 2 frames.
- si4_planar.tif: release 4.x (scanimage.SI4.* in every page), channels
  [1;2], 3 frames.
- state_stack.tif: release 3.x (state.* in the first page), channels 1
  and 2, 4 slices, frames averaged.
- v4_stopped.tif: format 4, motor stack of 10 slices stopped after 3 full
  slices and one frame of the fourth (2 frames per slice).

Also writes truth.tsv (series x channels x z x t, ScanImage channel
numbers, x pixel size in nm). Output in tests/data/scanimage/ is committed.

Usage: python3 -I tests/scripts/make_scanimage_samples.py
"""

import json
import shutil
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "scanimage"
MAGIC = 117637889


def page_bytes(w, h, rows):
    """`rows`: one value per row, or a single value for the whole page."""
    vals = rows if isinstance(rows, list) else [rows] * h
    return b"".join(struct.pack(f"<{w}h", *([v] * w)) for v in vals)


class Writer:
    """Classic or BigTIFF, one strip per page, int16."""

    def __init__(self, big, header=b""):
        self.big = big
        self.data = bytearray(b"II+\0\x08\0\0\0" + bytes(8) if big else b"II*\0" + bytes(4))
        self.data += header
        self.prev = 8 if big else 4

    def page(self, w, h, pixels, description=""):
        at = len(self.data)
        self.data += pixels
        d = description.encode() + b"\0"
        dat = len(self.data)
        self.data += d
        if len(self.data) % 2:
            self.data += b"\0"
        entries = [(256, 3, 1, w), (257, 3, 1, h), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
                   (270, 2, len(d), dat), (273, 4, 1, at), (277, 3, 1, 1), (278, 3, 1, h),
                   (279, 4, 1, len(pixels)), (339, 3, 1, 2)]
        ifd = len(self.data)
        if self.big:
            struct.pack_into("<Q", self.data, self.prev, ifd)
            self.data += struct.pack("<Q", len(entries))
            for tag, typ, count, value in entries:
                self.data += struct.pack("<HHQQ", tag, typ, count, value)
            self.prev = len(self.data)
            self.data += bytes(8)
        else:
            struct.pack_into("<I", self.data, self.prev, ifd)
            self.data += struct.pack("<H", len(entries))
            for tag, typ, count, value in entries:
                self.data += struct.pack("<HHII", tag, typ, count, value)
            self.prev = len(self.data)
            self.data += bytes(4)


def si_header(version, settings, rois=None):
    text = (settings + "\n").encode() + b"\0"
    roi = (json.dumps(rois).encode() + b"\0") if rois else b""
    return struct.pack("<IIII", MAGIC, version, len(text), len(roi)) + text + roi


def frame_header(n, t):
    return f"frameNumbers = {n}\nframeTimestamps_sec = {t:.6f}\nepoch = [2024  3  1 12 30 15.5]\n"


def v4_fast_flyback():
    settings = "\n".join([
        "SI.TIFF_FORMAT_VERSION = 4", "SI.VERSION_MAJOR = 2021", "SI.hChannels.channelSave = [1 3]",
        "SI.hChannels.channelName = {'Green' 'Red' 'Blue'}", "SI.hStackManager.enable = true",
        "SI.hStackManager.stackMode = 'fast'", "SI.hStackManager.numFramesPerVolume = 3",
        "SI.hStackManager.numFramesPerVolumeWithFlyback = 5", "SI.hStackManager.actualStackZStepSize = 2.5",
        "SI.hStackManager.stackDefinition = 'uniform'", "SI.hFastZ.enable = true", "SI.hScan2D.logAverageFactor = 1",
        "SI.hStackManager.framesPerSlice = 1", "SI.hRoiManager.scanFramePeriod = 0.01", "SI.hRoiManager.scanVolumeRate = 20",
        "SI.hRoiManager.imagingFovUm = [-40 -30;40 -30;40 30;-40 30]", "SI.hRoiManager.pixelsPerLine = 8",
        "SI.hRoiManager.linesPerFrame = 6"])
    t = Writer(True, si_header(4, settings))
    n = 0
    for v in range(2):
        for z in range(5):
            for ch in (1, 3):
                n += 1
                value = -2 if z >= 3 else 1000 * v + 10 * z + ch
                t.page(8, 6, page_bytes(8, 6, value), frame_header(n, n * 0.01))
    return t, "1x2x3x2", "1,3", "10000.0"


def v3_mroi():
    settings = "\n".join([
        "SI.TIFF_FORMAT_VERSION = 3", "SI.VERSION_MAJOR = '2018b'", "SI.hChannels.channelSave = 2",
        "SI.hFastZ.enable = false", "SI.hStackManager.numSlices = 1", "SI.hStackManager.framesPerSlice = 3",
        "SI.hScan2D.logAverageFactor = 1", "SI.hRoiManager.mroiEnable = 1", "SI.objectiveResolution = 10",
        "SI.hRoiManager.scanFramePeriod = 0.05"])
    sizes = [(8, 4), (8, 6), (8, 4)]
    rois = {"RoiGroups": {"imagingRoiGroup": {"rois": [
        {"name": name, "enable": 1, "scanfields": {"pixelResolutionXY": [w, h], "sizeXY": [1.6, 0.4 * h], "centerXY": [i, 0]}}
        for i, ((w, h), name) in enumerate(zip(sizes, ["A", "B", "A"]))]}}}
    t = Writer(True, si_header(3, settings, rois))
    gap = 3
    for f in range(3):
        rows = []
        for r, (_, h) in enumerate(sizes):
            if r:
                rows += [-1] * gap
            rows += [10000 * r + 1000 * f + 2] * h
        t.page(8, len(rows), page_bytes(8, len(rows), rows), frame_header(f + 1, f * 0.05))
    return t, "3x1x1x3", "2", "2000.0"


def f1_slow():
    settings = "\n".join([
        "scanimage.SI.TIFF_FORMAT_VERSION = 1", "scanimage.SI.VERSION_MAJOR = '5.1'", "scanimage.SI.hChannels.channelSave = 1",
        "scanimage.SI.hFastZ.enable = false", "scanimage.SI.hStackManager.numSlices = 3",
        "scanimage.SI.hStackManager.framesPerSlice = 2", "scanimage.SI.hScan2D.logAverageFactor = 1",
        "scanimage.SI.hStackManager.stackZStepSize = 1.5"])
    t = Writer(False)
    n = 0
    for z in range(3):
        for f in range(2):
            n += 1
            t.page(6, 4, page_bytes(6, 4, 1000 * f + 10 * z + 1), frame_header(n, n * 0.1) + settings)
    return t, "1x1x3x2", "1", "-"


def f2_json():
    desc = ('{\n\t"FrameHeader": {\n\t\t"frameNumbers": 1,\n\t\t"frameTimestamps_sec": 0.000000000,\n'
            '\t\t"epoch": [2016  5 15 17  2 56.927]\n\t},\n\n\t"SI": {\n\t\t"TIFF_FORMAT_VERSION": 2,\n'
            '\t\t"VERSION_MAJOR": "2015",\n\t\t"hChannels": {\n\t\t\t"channelSave": [\n\t\t\t\t1,\n\t\t\t\t2\n\t\t\t],\n'
            '\t\t\t"channelName": [\n\t\t\t\t"Channel 1",\n\t\t\t\t"Channel 2"\n\t\t\t]\n\t\t},\n'
            '\t\t"hFastZ": {\n\t\t\t"enable": 0\n\t\t},\n\t\t"hStackManager": {\n\t\t\t"numSlices": 1,\n'
            '\t\t\t"framesPerSlice": "_Inf_"\n\t\t},\n\t\t"hScan2D": {\n\t\t\t"logAverageFactor": 1\n\t\t}\n\t}\n}\n')
    t = Writer(True)
    for f in range(2):
        for ch in (1, 2):
            t.page(5, 3, page_bytes(5, 3, 1000 * f + ch), desc if f == 0 and ch == 1 else '{"FrameHeader": {}}')
    return t, "1x2x1x2", "1,2", "-"


def si4_planar():
    settings = "\n".join(["scanimage.SI4.channelsSave = [1;2]", "scanimage.SI4.stackNumSlices = 1",
                          "scanimage.SI4.fastZEnable = false", "scanimage.SI4.acqNumFrames = 0003",
                          "scanimage.SI4.acqNumAveragedFrames = 1", "scanimage.SI4.scanFramePeriod = 0.03",
                          "scanimage.SI4.triggerClockTimeFirst = '17-04-2015 14:35:56.050'"])
    t = Writer(False)
    for f in range(3):
        for ch in (1, 2):
            t.page(4, 4, page_bytes(4, 4, 1000 * f + ch), f"Frame Tag = {2 * f + ch:08d}\n" + settings)
    return t, "1x2x1x3", "1,2", "-"


def state_stack():
    settings = "\r".join(["state.software.version=3.6", "state.acq.numberOfZSlices=4", "state.acq.zStepSize=0.5",
                          "state.acq.averaging=1", "state.acq.numberOfFrames=4", "state.acq.savingChannel1=1",
                          "state.acq.savingChannel2=1", "state.acq.savingChannel3=0", "state.acq.frameRate=2"])
    t = Writer(False)
    for z in range(4):
        for ch in (1, 2):
            t.page(4, 3, page_bytes(4, 3, 10 * z + ch), settings if z == 0 and ch == 1 else "")
    return t, "1x2x4x1", "1,2", "-"


def v4_stopped():
    settings = "\n".join([
        "SI.TIFF_FORMAT_VERSION = 4", "SI.hChannels.channelSave = 1", "SI.hStackManager.enable = true",
        "SI.hStackManager.stackMode = 'slow'", "SI.hStackManager.numSlices = 1", "SI.hStackManager.actualNumSlices = 10",
        "SI.hStackManager.framesPerSlice = 2", "SI.hScan2D.logAverageFactor = 1", "SI.hFastZ.enable = false"])
    t = Writer(True, si_header(4, settings))
    n = 0
    for z in range(4):
        for f in range(2):
            if z == 3 and f == 1:
                break
            n += 1
            t.page(4, 4, page_bytes(4, 4, 1000 * f + 10 * z + 1), frame_header(n, n * 0.02))
    return t, "1x1x3x2", "1", "-"


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    rows = []
    for name, make in [("v4_fast_flyback.tif", v4_fast_flyback), ("v3_mroi.tif", v3_mroi), ("f1_slow.tif", f1_slow),
                       ("f2_json.tif", f2_json), ("si4_planar.tif", si4_planar), ("state_stack.tif", state_stack),
                       ("v4_stopped.tif", v4_stopped)]:
        t, dims, channels, px = make()
        (OUT / name).write_bytes(bytes(t.data))
        rows.append((name, dims, channels, px))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries_c_z_t\tchannels\tpx_nm\n")
        for r in rows:
            f.write("\t".join(r) + "\n")


if __name__ == "__main__":
    main()

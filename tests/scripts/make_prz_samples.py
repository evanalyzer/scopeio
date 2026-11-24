#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic CEOS Panta Rhei (.prz) files.

Layout from own analysis of a sample file: a ZIP archive of NumPy .npy
arrays (format from the numpy.lib.format documentation), data.npy holding
the image and meta_data_json.npy a 0-d UTF-32 string of JSON. No pickled
entries are written.

The metadata is a JSON list with one object per frame, as in the real
sample (a list of one object for a single image).

- plain.prz: stored, <u2 (5, 7), metadata with calibration, stage, time.
- deflated.prz: deflated entries, <f4 (3, 5, 7): three frames with one
  metadata object each (1 s apart, stage x +0.5 um per frame), no
  data_model_json.
- fortran.prz: stored, >i2 (5, 7) in Fortran order.
- zip64.prz: stored with ZIP64 local headers, |u1 (5, 7), no metadata.
- tem.prz: stored, <u2 (6, 8), the full key set of a TVIPS camera on a
  TEM (nested camera parameters, gun, condenser, projector, stage tilt,
  repo_id, calibration with a trailing null) with invented values.

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()` (native little-endian samples,
plane after plane). Output in tests/data/prz/ is committed.

Usage: python3 -I tests/scripts/make_prz_samples.py
"""

import json
import shutil
import struct
import zipfile
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "prz"
W, H = 7, 5


def npy(descr, shape, data, fortran=False):
    """A version 1.0 .npy file."""
    shape_s = "(" + ", ".join(str(s) for s in shape) + ("," if len(shape) == 1 else "") + ")"
    header = "{'descr': '%s', 'fortran_order': %s, 'shape': %s, }" % (descr, "True" if fortran else "False", shape_s)
    pad = 64 - (10 + len(header) + 1) % 64
    header = header + " " * pad + "\n"
    return b"\x93NUMPY\x01\x00" + struct.pack("<H", len(header)) + header.encode("latin1") + data


def text(s):
    """A 0-d <U array holding `s`."""
    return npy("<U%d" % len(s), (), s.encode("utf-32-le"))


META = {
    "camera.binning": [2, 2],
    "camera.start_time": "2026-10-02T11:43:51.742615",
    "camera.exposure_time": 1.5,
    "image_size": [W, H],
    "microscope.name": "TEM",
    "stage.x": 1.0e-6,
    "stage.y": -2.0e-6,
    "stage.z": 0.0,
    "type": "2D",
    "device.calib": [
        {"name": "Length", "value": 2.5e-10, "unit": "m", "use_prefix": True, "offset": 0.0, "pixel_factor": 2, "fixed_prefix": None},
        {"name": "Length", "value": 3.0e-10, "unit": "m", "use_prefix": True, "offset": 0.0, "pixel_factor": 2, "fixed_prefix": None},
        None,
    ],
}


# Key structure of the real sample, all values invented.
TEM = {
    "camera.binning": [1, 1],
    "camera.pixel_size": 1.4e-05,
    "camera.start_time": "2026-01-15T09:30:00.250000",
    "camera.requested_exposure_time": 0.5,
    "image_size": [8, 6],
    "camera.detector_size": [8, 6],
    "camera.area_offset": [0, 0],
    "source.name": "SyntheticCam",
    "source.type": "camera",
    "source.geometry": 0,
    "source.flip_geometry": False,
    "camera.parameters": {"next_index": 1, "readout_mode": 1, "trigger_mode": "Internal", "gain_mode": 1,
                          "integration_method": "Average", "high_tension": 300000},
    "ref_size": [8, 6],
    "camera.magnification_factor": 1.2,
    "camera.magnification_angle": 0.0,
    "camera.camera_length_factor": 1.1,
    "camera.camera_length_angle": 0.0,
    "camera.exposure_time": 0.500125,
    "camera.camc_parameters": {"BurstNumImages": 1, "ReadoutMode": 1, "HT": 300000, "exposure_time_ms": 500},
    "camera.min_exposure_time": 0.01,
    "camera.electrons_per_count": {"300000": 0.025},
    "camera.intensity_factor": 1,
    "type": "2D",
    "source.last_frame_info": {"buffer_index": 0, "missed_event": False},
    "microscope.name": "SyntheticTEM",
    "condenser.convergence_semi_angle": 0.0125,
    "condenser.mode": "TEM_1",
    "condenser.spotsize": 3,
    "electron_gun.voltage": 300000,
    "electron_gun.voltage_offset": 0.0,
    "objective.mode": "MAG",
    "projector.magnification": 20000,
    "projector.mode": "MAG",
    "projector.spectrum_mode": False,
    "stage.x": 1.25e-05,
    "stage.y": -3.5e-06,
    "stage.z": 2.0e-07,
    "stage.tilt_x": 0.1,
    "stage.tilt_y": 0.0,
    "repo_id": "synthetic_session.3",
    "device.calib": [
        {"name": "Length", "value": 1.4e-05 / 24000, "unit": "m", "use_prefix": True, "offset": 0.0, "pixel_factor": 1, "fixed_prefix": None},
        {"name": "Length", "value": 1.4e-05 / 24000, "unit": "m", "use_prefix": True, "offset": 0.0, "pixel_factor": 1, "fixed_prefix": None},
        None,
    ],
}


def write(name, entries, compression=zipfile.ZIP_STORED, zip64=False):
    with zipfile.ZipFile(OUT / name, "w") as z:
        for n, b in entries:
            info = zipfile.ZipInfo(n, date_time=(2026, 10, 2, 11, 43, 50))
            info.compress_type = compression
            with z.open(info, "w", force_zip64=zip64) as f:
                f.write(b)


def version():
    return npy("<i8", (), struct.pack("<q", 8))


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # plain: <u2, C order.
    vals = [(y * W + x) * 300 for y in range(H) for x in range(W)]
    data = struct.pack("<%dH" % len(vals), *vals)
    write("plain.prz", [("data_model_json.npy", text("[{}]")), ("meta_data_json.npy", text(json.dumps([META]))), ("data.npy", npy("<u2", (H, W), data)), ("file_format_version.npy", version())])
    truth.append(("plain.prz", data))

    # deflated: <f4, three frames.
    vals = [t * 100.0 + y * W + x + 0.25 for t in range(3) for y in range(H) for x in range(W)]
    data = struct.pack("<%df" % len(vals), *vals)
    # One second apart, stage moving 0.5 um in x per frame.
    meta = [dict(META, type="3D", **{"camera.start_time": "2026-10-02T11:43:5%d.000000" % (1 + t), "stage.x": 1.0e-6 + t * 5.0e-7}) for t in range(3)]
    write("deflated.prz", [("meta_data_json.npy", text(json.dumps(meta))), ("data.npy", npy("<f4", (3, H, W), data)), ("file_format_version.npy", version())], zipfile.ZIP_DEFLATED)
    truth.append(("deflated.prz", data))

    # fortran: >i2 in Fortran order (y fastest).
    grid = [[(y * W + x) * 50 - 800 for x in range(W)] for y in range(H)]
    stored = [grid[y][x] for x in range(W) for y in range(H)]
    data = struct.pack(">%dh" % len(stored), *stored)
    write("fortran.prz", [("meta_data_json.npy", text(json.dumps([META]))), ("data.npy", npy(">i2", (H, W), data, fortran=True))])
    native = [grid[y][x] for y in range(H) for x in range(W)]
    truth.append(("fortran.prz", struct.pack("<%dh" % len(native), *native)))

    # zip64: |u1, ZIP64 local headers, no metadata.
    data = bytes((y * W + x) * 7 % 256 for y in range(H) for x in range(W))
    write("zip64.prz", [("data.npy", npy("|u1", (H, W), data))], zip64=True)
    truth.append(("zip64.prz", data))

    # tem: <u2 (6, 8) with the full TEM key set.
    vals = [2000 + (y * 8 + x) * 37 for y in range(6) for x in range(8)]
    data = struct.pack("<%dH" % len(vals), *vals)
    write("tem.prz", [("data_model_json.npy", text("[{}]")), ("meta_data_json.npy", text(json.dumps([TEM]))), ("data.npy", npy("<u2", (6, 8), data)), ("file_format_version.npy", version())])
    truth.append(("tem.prz", data))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, pixels in truth:
            f.write("%s\t%08x\n" % (name, zlib.crc32(pixels)))


if __name__ == "__main__":
    main()

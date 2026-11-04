#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic MetaXpress plate.

Layout from the MetaXpress 6 batch import / export guide
(doc/metaxpress-htd/): <plate>.HTD in the plate folder, images named
<plate>_<well>_s<site>_w<wave>.TIF in TimePoint_<t>/ZStep_<z>/ folders
(ZStep_0 holds projections), a copy of the HTD and thumbnails in each
time point folder. Images are MetaMorph TIFFs with a UIC1 tag (33628)
holding the x / y calibration (0.5 um).

plate/: 8 x 12 plate, wells B02 and B03 selected (C04 has files but is not
selected), 2 x 2 sites with 3 selected, 2 waves, 2 time points, 2 z steps.
The image of B03 site 2, wave 2, t 2, z 1 is missing (zeros).

Also writes truth.tsv with the CRC-32 of each series in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/metaxpress/ is
committed.

Usage: python3 -I tests/scripts/make_metaxpress_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "metaxpress"
W, H = 7, 5
WELLS = ["B02", "B03", "C04"]
SITES, WAVES, TIMES, ZS = 3, 2, 2, 2
MISSING = ("B03", 2, 2, 2, 1)  # well, site, wave, t, z (from 1)


def metamorph_tiff(samples):
    """Uncompressed 16-bit TIFF with a UIC1 calibration of 0.5 um."""
    data = struct.pack(f"<{len(samples)}H", *samples)
    desc = b"Exposure: 25 ms\r\nBinning: 1 x 1\r\n\0"
    # UIC1 values after the image data: x / y rationals, unit string.
    extra_at = 8 + len(data)
    cal = struct.pack("<II", 1, 2)
    unit = struct.pack("<I", 4) + b"um\0e"
    desc_at = extra_at + 16 + len(unit)
    uic1 = struct.pack("<IIIIII", 4, extra_at, 5, extra_at + 8, 6, extra_at + 16)
    uic1_at = desc_at + len(desc)
    ifd_at = uic1_at + len(uic1)
    tags = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
            (270, 2, len(desc), desc_at), (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, H),
            (279, 4, 1, len(data)), (33628, 4, 3, uic1_at)]
    out = bytearray(b"II*\0" + struct.pack("<I", ifd_at)) + data + cal + cal + unit + desc + uic1
    out += struct.pack("<H", len(tags)) + b"".join(struct.pack("<HHII", *t) for t in tags) + bytes(4)
    return bytes(out)


def value(x, y, w, z, t, s):
    return (x * 1000 + y * 77 + w * 5000 + z * 300 + t * 11 + s * 2000) % 65536


def htd():
    rows = []
    for r in range(8):
        sel = ["TRUE" if "%s%02d" % (chr(65 + r), c + 1) in WELLS[:2] else "FALSE" for c in range(12)]
        rows.append(f'"WellsSelection{r + 1}", ' + ", ".join(sel))
    lines = ['"HTSInfoFile", Version 1.0', '"Description", "synthetic plate"', '"PlateType", 2',
             f'"TimePoints", {TIMES}', '"ZSeries", TRUE', f'"ZSteps", {ZS}', '"ZProjection", TRUE',
             '"XWells", 12', '"YWells", 8', *rows,
             '"Sites", TRUE', '"XSites", 2', '"YSites", 2',
             '"SiteSelection1", TRUE, FALSE', '"SiteSelection2", TRUE, TRUE',
             '"Waves", TRUE', f'"NWavelengths", {WAVES}', '"WaveName1", "DAPI"', '"WaveName2", "FITC"',
             '"WaveCollect1", 1', '"WaveCollect2", 1', '"EndFile"']
    return ("\r\n".join(lines) + "\r\n").encode("latin-1")


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    root = OUT / "plate"
    root.mkdir(parents=True)
    (root / "synth.HTD").write_bytes(htd())
    planes = {}
    for t in range(1, TIMES + 1):
        tdir = root / f"TimePoint_{t}"
        tdir.mkdir()
        (tdir / "synth.HTD").write_bytes(htd())
        for z in range(0, ZS + 1):
            zdir = tdir / f"ZStep_{z}"
            zdir.mkdir()
            for wi, well in enumerate(WELLS):
                for s in range(1, SITES + 1):
                    for w in range(1, WAVES + 1):
                        if (well, s, w, t, z) == MISSING:
                            continue
                        px = [value(x, y, w, z, t, wi * SITES + s) for y in range(H) for x in range(W)]
                        (zdir / f"synth_{well}_s{s}_w{w}.TIF").write_bytes(metamorph_tiff(px))
                        if z == 1:
                            (zdir / f"synth_{well}_s{s}_w{w}_Thumb.TIF").write_bytes(metamorph_tiff([0] * (W * H)))
                        planes[(well, s, w, t, z)] = px
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        series = 0
        for well in WELLS[:2]:
            for s in range(1, SITES + 1):
                out = bytearray()
                for t in range(1, TIMES + 1):
                    for z in range(1, ZS + 1):
                        for i in range(W * H):
                            for w in range(1, WAVES + 1):
                                p = planes.get((well, s, w, t, z))
                                out += struct.pack("<H", p[i] if p else 0)
                f.write(f"plate/synth.HTD\t{series}\t{zlib.crc32(out):08x}\n")
                series += 1


if __name__ == "__main__":
    main()

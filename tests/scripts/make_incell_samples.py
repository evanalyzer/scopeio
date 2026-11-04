#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic IN Cell Analyzer acquisitions (.xdce + TIFF files).

Layout from the public IN Cell acquisitions on the OME sample server: an
ImageStack XML with ObjectiveCalibration, Wavelengths, Plate and an Images
list (filename, Well Row/Column from 1, Identifier field/wave/z/time
indices from 0, PlatePosition_um). TIFF files written by the helper of
make_micromanager_samples.py.

- listed/plate.xdce: wells B3 and C4, 2 fields each, 2 waves (DAPI, FITC),
  2 z planes, uint16 7 x 5; the image of well C4 field 2, FITC, z 2 is
  listed but missing (reads as zeros).
- named/plate.xdce: an index listing no images (Images number="0"); the
  files are found by name, `A - 1(fld 1 wv DAPI - DAPI).tif`, well A1 with
  one field and two waves.

Also writes truth.tsv with the CRC-32 of each series in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/incell/ is
committed.

Usage: python3 -I tests/scripts/make_incell_samples.py
"""

import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_micromanager_samples import W, H, plain_tiff  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "incell"
WAVES = [("DAPI", 405, 455), ("FITC", 475, 511)]


def head(images):
    waves = "".join(f'''
      <Wavelength index="{i}" name="{n}_{em}">
        <ExcitationFilter name="{n}" unit="nm" wavelength="{ex}"/>
        <EmissionFilter name="{n}" unit="nm" wavelength="{em}"/>
        <Exposure time="{100 * (i + 1)}.0" unit="ms"/>
      </Wavelength>''' for i, (n, ex, em) in enumerate(WAVES))
    return f'''<?xml version="1.0" encoding="ISO-8859-1"?>
<ImageStack PlateID="Synthetic">
  <Creation date="2026-11-04" time="10:11:12"/>
  <AutoLeadAcquisitionProtocol>
    <ObjectiveCalibration magnification="20.00" numerical_aperture="0.75" pixel_height="0.3250" pixel_width="0.3250" unit="\xb5m"/>
    <Wavelengths>{waves}
    </Wavelengths>
    <Plate columns="12" name="96 well" rows="8"/>
    <ZDimensionParameters enabled="true" number_of_slices="2" step="1.500"/>
  </AutoLeadAcquisitionProtocol>
{images}
</ImageStack>
'''


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "listed").mkdir(parents=True)
    (OUT / "named").mkdir(parents=True)
    truth = []
    v = lambda row, col, f, c, z, x, y: (row * 5000 + col * 700 + f * 300 + c * 2000 + z * 90 + x * 11 + y * 3) % 65536
    missing = (3, 4, 1, 1, 1)
    entries = []
    for row, col in ((2, 3), (3, 4)):
        for f in range(2):
            px = b""
            for z in range(2):
                for y in range(H):
                    for x in range(W):
                        for c in range(2):
                            px += struct.pack("<H", 0 if (row, col, f, c, z) == missing else v(row, col, f, c, z, x, y))
            truth.append(("listed/plate.xdce", len(truth), zlib.crc32(px)))
            for c, (n, _, _) in enumerate(WAVES):
                for z in range(2):
                    name = f"{chr(64 + row)} - {col:02d}(fld {f + 1} wv {n} - {n} z {z + 1:02d}).tif"
                    if (row, col, f, c, z) != missing:
                        (OUT / "listed" / name).write_bytes(plain_tiff([v(row, col, f, c, z, x, y) for y in range(H) for x in range(W)], "H", 16))
                    entries.append(f'''    <Image filename="{name}" timestamp_sec="{1000 + z + 10 * c}.5" version="1.1">
      <Well label="{chr(64 + row)} - {col}"><Row number="{row}"/><Column number="{col}"/></Well>
      <PlatePosition_um x="{1000 * col + 100 * f}.0" y="{2000 * row}.0"/>
      <FocusPosition unit="\xb5m" z="{100 + 1.5 * z}"/>
      <Identifier field_index="{f}" time_index="0" wave_index="{c}" z_index="{z}" version="1.1"/>
    </Image>''')
    images = f'  <Images image_format="TIFF" number="{len(entries)}" version="1.0">\n' + "\n".join(entries) + "\n  </Images>"
    (OUT / "listed" / "plate.xdce").write_bytes(head(images).encode("latin-1"))

    u = lambda c, x, y: (c * 1000 + x * 7 + y * 31) % 65536
    for c, (n, _, _) in enumerate(WAVES):
        (OUT / "named" / f"A - 1(fld 1 wv {n} - {n}).tif").write_bytes(plain_tiff([u(c, x, y) for y in range(H) for x in range(W)], "H", 16))
    (OUT / "named" / "plate.xdce").write_bytes(head('  <Images image_format="TIFF" number="0" version="1.0"/>').encode("latin-1"))
    px = b"".join(struct.pack("<H", u(c, x, y)) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("named/plate.xdce", 0, zlib.crc32(px)))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tseries\tcrc32\n")
        for name, s, crc in truth:
            out.write(f"{name}\t{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Olympus OMP2INFO acquisition.

Layout from the public acquisition Olympus-OIR/gh-4205/zenodo-13680725
(CC BY 4.0): `matl:properties` with the stage overlap (percent), a
`matl:group` with `matl:areaInfo` (tile counts, left, top, tile width and
height in nm) and one `matl:area` per tile (OIR file, x and y index); the
vendor stitch is `Stitch_<area>_<group>.oir` next to the tiles.

Tiles and stitch are copies of tests/data/oir/synthetic.oir (written by
make_oir_samples.py) with its continuation file renamed to match:

- acq/acq.omp2info: two tiles (x 0 and 1, y 0), 10 % overlap, origin
  (-1000000, 2000000) nm, 100000 nm tiles, and Stitch_A01_G001.oir.

Output in tests/data/omp2info/ is committed.

Usage: python3 -I tests/scripts/make_omp2info_samples.py
"""

import shutil
from pathlib import Path

DATA = Path(__file__).resolve().parent.parent / "data"
OUT = DATA / "omp2info"

XML = """<?xml version="1.0" encoding="ASCII"?>
<matl:properties xmlns:matl="http://www.olympus.co.jp/hpf/protocol/matl/model/matl" version="2.3">
  <matl:stage>
    <matl:overlap>10</matl:overlap>
  </matl:stage>
  <matl:group>
    <matl:areaInfo>
      <matl:numOfXAreas>2</matl:numOfXAreas>
      <matl:numOfYAreas>1</matl:numOfYAreas>
      <matl:areaLeft>-1000000</matl:areaLeft>
      <matl:areaTop>2000000</matl:areaTop>
      <matl:areaWidth>100000</matl:areaWidth>
      <matl:areaHeight>100000</matl:areaHeight>
    </matl:areaInfo>
{areas}  </matl:group>
</matl:properties>
"""


def copy_oir(name):
    acq = OUT / "acq"
    shutil.copyfile(DATA / "oir" / "synthetic.oir", acq / f"{name}.oir")
    shutil.copyfile(DATA / "oir" / "synthetic_00001", acq / f"{name}_00001")


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "acq").mkdir(parents=True)
    areas = ""
    for i in range(2):
        name = f"syn_A01_G001_{i + 1:04}"
        copy_oir(name)
        areas += (f"    <matl:area>\n      <matl:image>{name}.oir</matl:image>\n"
                  f"      <matl:xIndex>{i}</matl:xIndex>\n      <matl:yIndex>0</matl:yIndex>\n    </matl:area>\n")
    copy_oir("Stitch_A01_G001")
    (OUT / "acq" / "acq.omp2info").write_text(XML.replace("{areas}", areas))


if __name__ == "__main__":
    main()

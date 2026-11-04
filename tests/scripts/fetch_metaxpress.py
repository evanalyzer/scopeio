#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Builds trimmed copies of the public MetaXpress plates.

The full plates are 0.8 to 17 GB. This keeps a few wells: it downloads
the .HTD files of downloads.openmicroscopy.org/images/MetaXpress/<plate>/
(also those in TimePoint_<n>/ folders), writes them with the
WellsSelection entries of all other wells set to FALSE and downloads the
image files of the kept wells (no thumbnails), keeping the folder layout.

Output in tests/reference/ome/MetaXpress/trim/<name>/, not committed. The
server states no licence for these IDR plates: used locally only.

Usage: python3 -I tests/scripts/fetch_metaxpress.py
"""

import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch_ome import OUT, fetch, listing  # noqa: E402

# name: (plate folder, kept wells)
PLATES = {
    "idr0005": ("MetaXpress/idr0005/Primary_001/", ["A01", "B02"]),
    "idr0006": ("MetaXpress/idr0006/plate 11001_Plate_136/", ["A01"]),
    "idr0008": ("MetaXpress/idr0008/Plate10_Actinome1/", ["A01", "A02", "A03"]),
    "idr0081": ("MetaXpress/idr0081/BSF018292-1A/", ["A01", "A02"]),
}


def well_of(name):
    m = re.search(r"_([A-Z]+\d+)(?=[_.])", name)
    return m.group(1) if m else None


def trim_htd(text, wells):
    out = []
    for line in text.splitlines():
        m = re.match(r'"WellsSelection(\d+)",(.*)', line)
        if m:
            row = int(m.group(1))
            vals = [v.strip() for v in m.group(2).split(",")]
            vals = [v if "%s%02d" % (chr(64 + row), col) in wells else "FALSE" for col, v in enumerate(vals, 1)]
            line = f'"WellsSelection{row}", ' + ", ".join(vals)
        out.append(line)
    return "\r\n".join(out) + "\r\n"


def trim(name, folder, wells):
    rels = listing(folder)
    dst = OUT / "MetaXpress" / "trim" / name
    for rel in rels:
        sub = rel[len(folder):]
        base = sub.rsplit("/", 1)[-1]
        target = dst / sub
        if base.upper().endswith(".HTD"):
            fetch(rel)
            target.parent.mkdir(parents=True, exist_ok=True)
            text = (OUT / rel).read_bytes().decode("latin-1")
            target.write_bytes(trim_htd(text, wells).encode("latin-1"))
        elif well_of(base) in wells and "thumb" not in base.lower() and not target.exists():
            fetch(rel)
            target.parent.mkdir(parents=True, exist_ok=True)
            os.link(OUT / rel, target)


def main():
    for name, args in PLATES.items():
        trim(name, *args)


if __name__ == "__main__":
    main()

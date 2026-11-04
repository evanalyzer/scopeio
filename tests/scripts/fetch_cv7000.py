#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Builds trimmed copies of the public Yokogawa CV7000 / CV8000 plates.

The full plates are 4 to 150 GB. This keeps a few wells and fields: it
downloads the small index files of downloads.openmicroscopy.org/images/
CV7000/<plate>/, writes MeasurementData.mlf with only the records of the
kept wells and fields (other lines unchanged), sets FieldCount in
MeasurementDetail.mrf and downloads the image files those records name.
Files missing on the server are skipped (idr0088 lacks some wells).

Output in tests/reference/ome/CV7000/trim/<name>/, not committed. Licences:
idr0088 CC BY-NC 4.0 (local only), idr0093 CC BY 4.0, cpg0016 (Cell
Painting Gallery) CC0 1.0.

Usage: python3 -I tests/scripts/fetch_cv7000.py
"""

import re
import sys
import urllib.error
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch_ome import OUT, fetch, listing  # noqa: E402

# name: (plate folder, kept wells, kept fields)
PLATES = {
    "idr0088": ("CV7000/idr0088/110000251230/", ["B02", "B03", "M19"], 2),
    "idr0093": ("CV7000/idr0093/020/", ["B02"], 2),
    "cpg0016": ("CV7000/cpg0016/Dest21053D1-15214/", ["A01", "A02"], 2),
}
INDEX = re.compile(r"\.(wpi|wpp|mes|mrf|mlf)$")


def trim(name, folder, wells, fields):
    rels = [r for r in listing(folder) if INDEX.search(r)]
    for rel in rels:
        fetch(rel)
    src = OUT / folder
    dst = OUT / "CV7000" / "trim" / name
    dst.mkdir(parents=True, exist_ok=True)
    images = []
    for rel in rels:
        f = OUT / rel
        text = f.read_text(encoding="utf-8-sig")
        if f.name == "MeasurementData.mlf":
            kept = []
            for line in text.splitlines():
                m = re.search(r'bts:Column="(\d+)" bts:Row="(\d+)".*?bts:FieldIndex="(\d+)"', line)
                if m:
                    well = "%s%02d" % (chr(64 + int(m.group(2))), int(m.group(1)))
                    if well not in wells or int(m.group(3)) > fields:
                        continue
                    image = re.search(r">([^<]+\.tif)<", line)
                    if image:
                        images.append(image.group(1))
                kept.append(line)
            text = "\n".join(kept) + "\n"
        elif f.name == "MeasurementDetail.mrf":
            text = re.sub(r'bts:FieldCount="\d+"', f'bts:FieldCount="{fields}"', text)
        (dst / f.name).write_text(text, encoding="utf-8")
    for image in images:
        if (dst / image).exists():
            continue
        try:
            fetch(folder + image)
        except urllib.error.HTTPError as e:
            print(f"skip {image}: {e}", file=sys.stderr)
            continue
        (dst / image).hardlink_to(src / image)


def main():
    for name, args in PLATES.items():
        trim(name, *args)


if __name__ == "__main__":
    main()

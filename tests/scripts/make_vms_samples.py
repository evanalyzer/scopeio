#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Hamamatsu VMS slide.

Layout from the openslide.org Hamamatsu page (doc/hamamatsu-ndpi-vms/): an
INI index with a [Virtual Microscope Specimen] section naming a grid of
JPEG files (ImageFile, ImageFile(x,y), ImageFile(z,x,y)), a MapFile and a
MacroImage. JPEG files written by ImageMagick through the helper of
make_ndpi_samples.py (constant 8 x 8 blocks, exact in any decoder).

- Grid of 2 x 2 files: columns 32 and 16 px wide, rows 16 and 8 px high
  (48 x 24 px), restart interval 2 MCUs (16 px).
- A second focal plane with only ImageFile(1,0,0); its other files are
  missing and read as zeros.
- Map 24 x 12 px (factor 2, no restart markers), macro 16 x 8 px.

Also writes truth.tsv with the CRC-32 of each level (both focal planes)
and of the macro image, in the layout of tests/common/mod.rs
`decode_level()`. Output in tests/data/vms/ is committed.

Usage: python3 -I tests/scripts/make_vms_samples.py   (needs `magick`)
"""

import shutil
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_ndpi_samples import gray_blocks, jpeg  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "vms"
COLS = [32, 16]
ROWS = [16, 8]


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    W, H = sum(COLS), sum(ROWS)
    planes = [bytearray(W * H * 3), bytearray(W * H * 3)]
    lines = ["[Virtual Microscope Specimen]", "NoLayers=2", f"NoJpegColumns={len(COLS)}", f"NoJpegRows={len(ROWS)}"]
    seed = 0
    for z in range(2):
        for y, h in enumerate(ROWS):
            for x, w in enumerate(COLS):
                if z == 1 and (x, y) != (0, 0):
                    continue
                seed += 1
                px = gray_blocks(w, h, seed)
                name = f"slide({z},{x},{y}).jpg"
                (OUT / name).write_bytes(jpeg(px, w, h, 2))
                key = "ImageFile" if (z, x, y) == (0, 0, 0) else (f"ImageFile({x},{y})" if z == 0 else f"ImageFile({z},{x},{y})")
                lines.append(f"{key}={name}")
                ox, oy = sum(COLS[:x]), sum(ROWS[:y])
                for r in range(h):
                    planes[z][((oy + r) * W + ox) * 3:((oy + r) * W + ox + w) * 3] = px[r * w * 3:(r + 1) * w * 3]
    mw, mh = W // 2, H // 2
    map_px = gray_blocks(mw, mh, 99)
    (OUT / "slide_map.jpg").write_bytes(jpeg(map_px, mw, mh, 0))
    macro_px = gray_blocks(16, 8, 7)
    (OUT / "slide_macro.jpg").write_bytes(jpeg(macro_px, 16, 8, 0))
    lines += ["MapFile=slide_map.jpg", "MacroImage=slide_macro.jpg", "SourceLens=20.000000",
              f"PhysicalWidth={W * 500}", f"PhysicalHeight={H * 500}", "PhysicalMacroHeight=4000;"]
    (OUT / "slide.vms").write_bytes(("\r\n\r\n".join(lines) + "\r\n").encode())
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        f.write(f"level0\t{zlib.crc32(bytes(planes[0] + planes[1])):08x}\n")
        f.write(f"level1\t{zlib.crc32(map_px + map_px):08x}\n")
        f.write(f"macro\t{zlib.crc32(macro_px):08x}\n")


if __name__ == "__main__":
    main()

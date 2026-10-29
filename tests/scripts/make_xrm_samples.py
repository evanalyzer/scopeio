#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Zeiss Xradia files (compound files, cfbwriter.py).

Stream layout from the Xradia sample on Zenodo and dxchange (BSD-3-Clause,
read as documentation): ImageInfo/NoOfImages, ImageWidth, ImageHeight,
DataType (5 uint16, 10 float32) as u32, PixelSize (f32, um), Angles,
X/Y/ZPosition and ExpTimes (f32 per image), image n (from 1) in
ImageData<ceil(n/100)>/Image<n>.

- projections.txrm: 150 uint16 images of 7 x 5 (two ImageData storages,
  images in the mini stream), 0.5 um pixels, angles and positions.
- single.xrm: one float32 image of 40 x 30 (regular sectors).

Also writes truth.tsv with the CRC-32 of each file in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/xrm/ is
committed.

Usage: python3 -I tests/scripts/make_xrm_samples.py
"""

import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from cfbwriter import cfb_tree  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "xrm"


def info(n, w, h, dtype, pixel_um, angles=(), xs=(), ys=(), zs=(), exposures=()):
    u = lambda v: struct.pack("<I", v)
    f = lambda vs: struct.pack(f"<{len(vs)}f", *vs)
    d = {"NoOfImages": u(n), "ImageWidth": u(w), "ImageHeight": u(h), "DataType": u(dtype), "PixelSize": f([pixel_um])}
    for name, vs in (("Angles", angles), ("XPosition", xs), ("YPosition", ys), ("ZPosition", zs), ("ExpTimes", exposures)):
        if vs:
            d[name] = f(vs)
    return d


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    n, w, h = 150, 7, 5
    v = lambda x, y, k: (x * 301 + y * 47 + k * 113) % 65536
    tree = {"ImageInfo": info(n, w, h, 5, 0.5, [k * 1.2 for k in range(n)], [10.0] * n, [-2.5] * n,
                              [k * 0.25 for k in range(n)], [0.75] * n)}
    for k in range(1, n + 1):
        group = tree.setdefault(f"ImageData{-(-k // 100)}", {})
        group[f"Image{k}"] = b"".join(struct.pack("<H", v(x, y, k)) for y in range(h) for x in range(w))
    (OUT / "projections.txrm").write_bytes(cfb_tree(tree))
    truth.append(("projections.txrm", zlib.crc32(b"".join(struct.pack("<H", v(x, y, k)) for k in range(1, n + 1) for y in range(h) for x in range(w)))))

    w, h = 40, 30
    f = lambda x, y: x * 0.5 - y * 0.25
    data = b"".join(struct.pack("<f", f(x, y)) for y in range(h) for x in range(w))
    tree = {"ImageInfo": info(1, w, h, 10, 1.5), "ImageData1": {"Image1": data}}
    (OUT / "single.xrm").write_bytes(cfb_tree(tree))
    truth.append(("single.xrm", zlib.crc32(data)))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

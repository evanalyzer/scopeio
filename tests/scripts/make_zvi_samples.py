#!/usr/bin/env python3
"""Writes small synthetic Zeiss AxioVision ZVI files.

Layout from the OpenReadout ZVI notes (doc/zeiss-zvi/): a compound file
([MS-CFB], written by cfbwriter.py) with `Image/Item(n)/Contents` (typed
values, then the raw-image header: marker 0x10002000, width, height, depth,
bytes per pixel, pixel format, valid bits, then the samples),
`Image/Item(n)/Tags/Contents` and `Image/Tags/Contents` tag lists of
[MS-OAUT] typed values. Files:

- gray.zvi: 2 channels x 3 z of 9 x 7 uint16, stored channel-interleaved
  with non-contiguous indices (z 13..15, channels 2 and 4), 0.25 um pixels,
  1.5 um z step, channel names, colours, exposure and relative times.
- colour.zvi: one 40 x 30 colour item of 3 x uint16 stored blue first,
  uncalibrated, exposure only in the image tags (regular, not mini, sectors).

Also writes truth.tsv with the CRC-32 of the pixels in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/zvi/ is
committed.

Usage: python3 -I tests/scripts/make_zvi_samples.py
"""

import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from cfbwriter import cfb_tree  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "zvi"


def i32(v):
    return struct.pack("<Hi", 3, v)


def f64(v):
    return struct.pack("<Hd", 5, v)


def text(s):
    b = (s + "\0").encode("utf-16-le")
    return struct.pack("<HI", 8, len(b)) + b


def date(days):
    return struct.pack("<Hd", 7, days)


def tags(entries):
    """Tag list: version, count, then (value, id, attribute)."""
    out = i32(0x20001000) + i32(len(entries))
    for tid, value in entries:
        out += value + i32(tid) + i32(0)
    return out


def item(width, height, bpp, fmt, samples):
    prefix = i32(1) + f64(0.0) + text("item")
    header = struct.pack("<7I", 0x10002000, width, height, 1, bpp, fmt, 16)
    return prefix + header + samples


def value(x, y, c, z):
    return (x * 211 + y * 97 + c * 3000 + z * 450) % 65536


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # Gray: items channel-interleaved; stored indices z 13..15, channels 2 and 4.
    W, H = 9, 7
    channels = [(2, "DAPI", 0xFF0000, 120.0), (4, "FITC", 0x00FF00, 45.5)]
    items = {}
    n = 0
    for zi, z in enumerate(range(13, 16)):
        for ci, (c, name, colour, exposure) in enumerate(channels):
            samples = b"".join(struct.pack("<H", value(x, y, ci, zi)) for y in range(H) for x in range(W))
            item_tags = [(2819, i32(z)), (2820, i32(c)), (2821, i32(0)), (1284, text(name)), (1282, i32(colour)),
                         (2564, f64(exposure)), (300, f64(n * 1.5 / 86400)), (1025, date(45021.5 + n / 86400))]
            items[f"Item({n})"] = {"Contents": item(W, H, 2, 4, samples), "Tags": {"Contents": tags(item_tags)}}
            n += 1
    image_tags = [(769, f64(0.25)), (770, i32(76)), (772, f64(0.25)), (773, i32(76)), (775, f64(1.5)), (776, i32(76)),
                  (1553, text("gray.zvi")), (2049, text("Plan-Apochromat 63x/1.40 Oil")), (2076, f64(63.0)), (2077, f64(1.4))]
    tree = {"Image": {"Contents": i32(0), "Tags": {"Contents": tags(image_tags)}, **items}}
    (OUT / "gray.zvi").write_bytes(cfb_tree(tree))
    px = b"".join(struct.pack("=H", value(x, y, c, z)) for z in range(3) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("gray.zvi", px))

    # Colour: 40 x 30 x (B, G, R) uint16.
    W, H = 40, 30
    rgb = lambda x, y, k: (x * 1000 + y * 300 + k * 20000) % 65536
    samples = b"".join(struct.pack("<3H", rgb(x, y, 2), rgb(x, y, 1), rgb(x, y, 0)) for y in range(H) for x in range(W))
    tree = {"Image": {"Contents": i32(0), "Tags": {"Contents": tags([(770, i32(0)), (2564, f64(20.0))])},
                      "Item(0)": {"Contents": item(W, H, 6, 8, samples), "Tags": {"Contents": tags([(2819, i32(0))])}}}}
    (OUT / "colour.zvi").write_bytes(cfb_tree(tree))
    truth.append(("colour.zvi", b"".join(struct.pack("=3H", rgb(x, y, 0), rgb(x, y, 1), rgb(x, y, 2)) for y in range(H) for x in range(W))))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, data in truth:
            f.write(f"{name}\t{zlib.crc32(data):08x}\n")


if __name__ == "__main__":
    main()

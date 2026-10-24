#!/usr/bin/env python3
"""Writes small synthetic PNG, JPEG and JPEG 2000 files.

Pixel values are generated here and converted by ImageMagick (run as a
program; libpng, libjpeg-turbo and OpenJPEG). PNG files cover every colour
type and bit depth (gray 1/8/16, gray + alpha, RGB 8/16, RGBA, palette,
interlaced) and a pHYs chunk; JPEG 2000 files are lossless (JP2 and raw
J2K, 8 and 16 bits). truth.tsv holds the CRC-32 of the pixels each lossless
file must decode to in the layout of tests/common/mod.rs `decode_level()`
(channels interleaved per pixel, native-endian samples); palette PNGs are
expanded to RGB, low-bit gray keeps its values (0 and 1 for 1 bit). The lossy JPEG files are checked
against the reference readers instead. Output in tests/data/plain/ is
committed.

Usage: python3 -I tests/scripts/make_plain_samples.py   (needs `magick`)
"""

import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "plain"
W, H = 37, 23


def value(x, y, k, bits):
    return ((x * 7 + y * 13 + k * 101) * (1 if bits == 8 else 257) + (x * y if bits == 16 else 0)) % (1 << bits)


def raw(samples, bits):
    fmt = ">H" if bits == 16 else "B"
    return b"".join(struct.pack(fmt, value(x, y, k, bits)) for y in range(H) for x in range(W) for k in range(samples))


def native(samples, bits):
    fmt = "=H" if bits == 16 else "B"
    return b"".join(struct.pack(fmt, value(x, y, k, bits)) for y in range(H) for x in range(W) for k in range(samples))


def magick(src_kind, samples, bits, data, out, *opts):
    with tempfile.TemporaryDirectory() as d:
        src = Path(d) / "in.raw"
        src.write_bytes(data)
        subprocess.run(["magick", "-size", f"{W}x{H}", "-depth", str(bits), "-endian", "MSB", f"{src_kind}:{src}", *opts, str(out)], check=True)


def main():
    if shutil.which("magick") is None:
        sys.exit("needs ImageMagick (`magick`)")
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # PNG: (name, source kind, samples, bits, extra options)
    for name, kind, s, bits, opts in [
        ("gray8.png", "gray", 1, 8, ["-define", "png:color-type=0"]),
        ("gray16.png", "gray", 1, 16, ["-define", "png:color-type=0"]),
        ("rgb8.png", "rgb", 3, 8, ["-define", "png:color-type=2", "-units", "PixelsPerCentimeter", "-density", "40000"]),
        ("rgb16.png", "rgb", 3, 16, ["-define", "png:color-type=2"]),
        ("rgba8.png", "rgba", 4, 8, ["-define", "png:color-type=6"]),
        ("graya8.png", "rgba", 2, 8, []),  # written below from gray + alpha
        ("rgb8_interlaced.png", "rgb", 3, 8, ["-define", "png:color-type=2", "-interlace", "PNG"]),
    ]:
        if name == "graya8.png":
            rgba = b"".join(bytes([value(x, y, 0, 8)] * 3 + [value(x, y, 1, 8)]) for y in range(H) for x in range(W))
            magick("rgba", 4, 8, rgba, OUT / name, "-define", "png:color-type=4")
            truth.append((name, native(2, 8)))
            continue
        magick(kind, s, bits, raw(s, bits), OUT / name, *opts)
        truth.append((name, native(s, bits)))

    # 1-bit gray: a checkerboard, read back as 0 and 1.
    bits1 = bytes((255 if (x // 3 + y // 2) % 2 else 0) for y in range(H) for x in range(W))
    magick("gray", 1, 8, bits1, OUT / "gray1.png", "-define", "png:bit-depth=1", "-define", "png:color-type=0")
    truth.append(("gray1.png", bytes(v // 255 for v in bits1)))
    # 2-bit gray, interlaced: values 0..=3 kept as they are.
    bits2 = bytes(((x + 2 * y) % 4) * 85 for y in range(H) for x in range(W))
    magick("gray", 1, 8, bits2, OUT / "gray2_interlaced.png", "-define", "png:bit-depth=2", "-define", "png:color-type=0", "-interlace", "PNG")
    truth.append(("gray2_interlaced.png", bytes(v // 85 for v in bits2)))
    # Palette: four colours, expanded to RGB.
    pal = [(255, 0, 0), (0, 128, 0), (0, 0, 255), (250, 250, 10)]
    pal_px = b"".join(bytes(pal[(x // 5 + y // 4) % 4]) for y in range(H) for x in range(W))
    magick("rgb", 3, 8, pal_px, OUT / "palette.png", "-define", "png:color-type=3")
    truth.append(("palette.png", pal_px))

    # JPEG 2000, lossless.
    magick("rgb", 3, 8, raw(3, 8), OUT / "rgb8.jp2", "-define", "jp2:quality=0")
    truth.append(("rgb8.jp2", native(3, 8)))
    magick("gray", 1, 16, raw(1, 16), OUT / "gray16.j2k", "-define", "jp2:quality=0")
    truth.append(("gray16.j2k", native(1, 16)))

    # JPEG (lossy): gray and RGB, the latter with a JFIF density of 254 dpi.
    magick("gray", 1, 8, raw(1, 8), OUT / "gray8.jpg", "-quality", "95")
    magick("rgb", 3, 8, raw(3, 8), OUT / "rgb8.jpg", "-quality", "95", "-units", "PixelsPerInch", "-density", "254")

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

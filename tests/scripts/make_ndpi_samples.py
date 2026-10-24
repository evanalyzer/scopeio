#!/usr/bin/env python3
"""Writes a small synthetic Hamamatsu NDPI file.

Layout from the openslide.org Hamamatsu page (doc/hamamatsu-ndpi-vms/):
classic little-endian TIFF with a 64-bit first-IFD offset and next-IFD
links, the high 32 bits of every entry value after each IFD, tag 65420,
65421 (source lens; -1 macro) and 65426 (offsets of the restart intervals,
relative to the JPEG stream). The JPEG streams are written by ImageMagick
(run as a separate program) with restart markers; every 8 x 8 block is a
constant gray, which decodes exactly in any decoder. Also writes truth.tsv
with the CRC-32 of each level. Output in tests/data/ndpi/ is committed.

Usage: python3 -I tests/scripts/make_ndpi_samples.py   (needs `magick`)
"""

import shutil
import struct
import subprocess
import tempfile
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "ndpi"


def gray_blocks(w, h, seed):
    """RGB pixels with a constant gray per 8 x 8 block."""
    return bytes(((x // 8 * 37 + y // 8 * 53 + seed * 11) % 200 + 20) for y in range(h) for x in range(w) for _ in range(3))


def jpeg(pixels, w, h, restart):
    with tempfile.TemporaryDirectory() as tmp:
        raw, out = Path(tmp) / "in.rgb", Path(tmp) / "out.jpg"
        raw.write_bytes(pixels)
        args = ["magick", "-size", f"{w}x{h}", "-depth", "8", f"rgb:{raw}", "-type", "TrueColor", "-quality", "100", "-sampling-factor", "1x1"]
        if restart:
            args += ["-define", f"jpeg:restart-interval={restart}"]
        subprocess.run(args + [str(out)], check=True)
        data = out.read_bytes()
        back = subprocess.run(["magick", str(out), "-depth", "8", "rgb:-"], check=True, capture_output=True).stdout
        assert back == pixels, "JPEG round trip not exact"
        return data


def rst_offsets(data):
    """Offsets of the entropy data after SOS and after every RSTn marker."""
    pos = 2
    while True:
        marker = data[pos + 1]
        length = struct.unpack(">H", data[pos + 2:pos + 4])[0]
        if marker == 0xDA:
            start = pos + 2 + length
            break
        pos += 2 + length
    offs = [start]
    i = start
    while i < len(data) - 1:
        if data[i] == 0xFF and 0xD0 <= data[i + 1] <= 0xD7:
            offs.append(i + 2)
        i += 1
    return offs


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    levels = [(64, 24, 2), (32, 12, 0)]  # width, height, restart interval in MCUs
    data = bytearray(b"II*\0" + bytes(8))
    images = []
    truth = []
    for i, (w, h, restart) in enumerate(levels):
        px = gray_blocks(w, h, i)
        images.append((w, h, jpeg(px, w, h, restart), 40.0 / (1 << (2 * i)), restart))
        truth.append((f"level{i}", px))
    macro = gray_blocks(16, 8, 9)
    images.append((16, 8, jpeg(macro, 16, 8, 0), -1.0, 0))
    truth.append(("macro", macro))
    prev = 4
    for w, h, stream, lens, restart in images:
        at = len(data)
        data += stream
        extra = bytearray()
        entries = []

        def ext(b):
            base = len(data) + len(extra)
            extra.extend(b)
            return base

        offs = rst_offsets(stream) if restart else []
        entries += [(256, 4, 1, w), (257, 4, 1, h), (258, 3, 3, None), (259, 3, 1, 7), (262, 3, 1, 6),
                    (271, 2, 10, None), (273, 4, 1, at), (277, 3, 1, 3), (278, 4, 1, h), (279, 4, 1, len(stream)),
                    (282, 5, 1, None), (283, 5, 1, None), (296, 3, 1, 3), (306, 2, 20, None), (65420, 3, 1, 1),
                    (65421, 11, 1, struct.unpack("<I", struct.pack("<f", lens))[0])]
        if offs:
            entries.append((65426, 4, len(offs), None))
        values = {258: struct.pack("<3H", 8, 8, 8), 271: b"Hamamatsu\0", 282: struct.pack("<II", 40000, 1),
                  283: struct.pack("<II", 40000, 1), 306: b"2021:05:06 07:08:09\0",
                  65426: struct.pack(f"<{len(offs)}I", *offs)}
        resolved = []
        for tag, typ, count, value in entries:
            if value is None:
                b = values[tag]
                value = ext(b) if len(b) > 4 else struct.unpack("<I", b.ljust(4, b"\0"))[0]
            resolved.append((tag, typ, count, value))
        data += extra
        ifd_at = len(data)
        struct.pack_into("<Q", data, prev, ifd_at)
        data += struct.pack("<H", len(resolved))
        for tag, typ, count, value in resolved:
            v = struct.pack("<HH", value, 0) if typ == 3 and count == 1 else struct.pack("<I", value & 0xFFFFFFFF)
            data += struct.pack("<HHI", tag, typ, count) + v
        prev = len(data)
        data += bytes(8)
        data += bytes(4 * len(resolved))  # high 32 bits of every value
    (OUT / "synthetic.ndpi").write_bytes(bytes(data))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

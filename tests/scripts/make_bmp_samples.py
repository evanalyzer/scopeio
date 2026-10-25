#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic BMP files covering the variants of the format.

Written from the Microsoft bitmap documentation in doc/bmp/. Also writes
truth.tsv with the CRC-32 of the pixels each file must decode to, computed
from the values encoded here, in scopeio's output layout: top row first,
interleaved R, G, B(, A), 8 bits per sample; gray palettes as one sample.
The output in tests/data/bmp/ is committed; rerun only to add or change
samples.

Usage: python3 tests/scripts/make_bmp_samples.py
"""

import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "bmp"

BI_RGB, BI_RLE8, BI_RLE4, BI_BITFIELDS = 0, 1, 2, 3

# (file name, samples per pixel, expected pixel bytes)
TRUTH = []


def expect(name, samples, values):
    TRUTH.append((name, samples, bytes(values)))


def expect_indexed(name, palette, index_rows):
    """Expected output of palette indices, top row first."""
    gray = all(r == g == b for r, g, b in palette)
    out = []
    for row in index_rows:
        for i in row:
            out.extend(palette[i][:1] if gray else palette[i])
    expect(name, 1 if gray else 3, out)


def stride(width, bits):
    return ((width * bits + 31) // 32) * 4


def pack_row(values, bits):
    """Packs palette indices or raw pixel bytes into one padded row."""
    if bits >= 8:
        row = bytes(values)
    else:
        row = bytearray()
        per_byte = 8 // bits
        for i in range(0, len(values), per_byte):
            byte = 0
            for j, v in enumerate(values[i:i + per_byte]):
                byte |= v << (8 - bits * (j + 1))
            row.append(byte)
        row = bytes(row)
    return row


def info_header(size, width, height, bits, compression, image_size=0, ppm=(2835, 2835),
                colors_used=0, masks=None):
    """BITMAPINFOHEADER and, for size >= 52, the V2/V3/V4/V5 mask fields."""
    h = struct.pack("<IiiHHIIiiII", size, width, height, 1, bits, compression, image_size,
                    ppm[0], ppm[1], colors_used, 0)
    if size >= 52:
        r, g, b, a = masks or (0, 0, 0, 0)
        h += struct.pack("<III", r, g, b)
        if size >= 56:
            h += struct.pack("<I", a)
    if size >= 108:
        h += struct.pack("<I", 0x73524742)  # 'sRGB' colour space
        h += bytes(36 + 12)  # endpoints and gamma
    if size >= 124:
        h += struct.pack("<IIII", 4, 0, 0, 0)  # intent, profile data, profile size, reserved
    assert len(h) == size, (len(h), size)
    return h


def write(name, header, palette, pixels):
    offset = 14 + len(header) + len(palette)
    data = header + palette + pixels
    file_header = struct.pack("<2sIHHI", b"BM", 14 + len(data), 0, 0, offset)
    (OUT / name).write_bytes(file_header + data)


def rows_bottom_up(rows):
    return b"".join(reversed(rows))


def rgbquad(colors):
    return b"".join(bytes([b, g, r, 0]) for r, g, b in colors)


def indexed(name, width, height, bits, palette, top_down=False, colors_used=0):
    rows = []
    for y in range(height):
        values = [(x * 3 + y * 5) % len(palette) for x in range(width)]
        row = pack_row(values, bits)
        rows.append(row + bytes(stride(width, bits) - len(row)))
    pixels = b"".join(rows) if top_down else rows_bottom_up(rows)
    expect_indexed(name, palette, [[(x * 3 + y * 5) % len(palette) for x in range(width)]
                                   for y in range(height)])
    h = -height if top_down else height
    write(name, info_header(40, width, h, bits, BI_RGB, colors_used=colors_used), rgbquad(palette),
          pixels)


def gradient(x, y, c):
    return (x * 37 + y * 71 + c * 101) % 256


def rgb24(name, width, height):
    rows = []
    for y in range(height):
        row = b"".join(bytes([gradient(x, y, 2), gradient(x, y, 1), gradient(x, y, 0)])
                       for x in range(width))
        rows.append(row + bytes(stride(width, 24) - len(row)))
    expect(name, 3, [gradient(x, y, c) for y in range(height) for x in range(width) for c in range(3)])
    write(name, info_header(40, width, height, 24, BI_RGB), b"", rows_bottom_up(rows))


def packed(name, width, height, bits, masks, header_size=40, compression=BI_BITFIELDS,
           unused_fill=0):
    """16- or 32-bit pixels built from per-channel values and bit masks."""
    def quantize(mask, value):
        top = mask >> ((mask & -mask).bit_length() - 1)
        return (value * top + 127) // 255, top

    def field(mask, value):
        if not mask:
            return 0
        q, _ = quantize(mask, value)
        return q << ((mask & -mask).bit_length() - 1)

    def decoded(mask, value):
        """8-bit value a reader recovers from the stored field (rounded rescale)."""
        q, top = quantize(mask, value)
        return (q * 255 + top // 2) // top

    nbytes = bits // 8
    used = 0
    for m in masks:
        used |= m
    rows = []
    for y in range(height):
        row = bytearray()
        for x in range(width):
            v = 0
            for c, mask in enumerate(masks):
                v |= field(mask, gradient(x, y, c))
            v |= unused_fill & ~used & ((1 << bits) - 1)
            row += v.to_bytes(nbytes, "little")
        rows.append(bytes(row) + bytes(stride(width, bits) - len(row)))
    expect(name, len(masks), [decoded(m, gradient(x, y, c)) for y in range(height)
                              for x in range(width) for c, m in enumerate(masks)])
    extra = b""
    if compression == BI_BITFIELDS and header_size == 40:
        extra = struct.pack("<III", *masks[:3])
    header = info_header(header_size, width, height, bits, compression, masks=tuple(masks) + (0,) * (4 - len(masks)))
    write(name, header, extra, rows_bottom_up(rows))


def rle(name, bits, width, height, stream, palette, index_rows):
    """RLE file; `index_rows` are the indices the stream decodes to, top row first."""
    expect_indexed(name, palette, index_rows)
    write(name, info_header(40, width, height, bits, BI_RLE8 if bits == 8 else BI_RLE4,
                            image_size=len(stream), colors_used=len(palette)),
          rgbquad(palette), stream)


def core_header(name, width, height):
    palette = [(i, 255 - i, (i * 7) % 256) for i in range(256)]
    rows = []
    for y in range(height):
        row = bytes((x + y * 3) % 256 for x in range(width))
        rows.append(row + bytes(stride(width, 8) - len(row)))
    expect_indexed(name, palette, [[(x + y * 3) % 256 for x in range(width)] for y in range(height)])
    header = struct.pack("<IHHHH", 12, width, height, 1, 8)
    triples = b"".join(bytes([b, g, r]) for r, g, b in palette)
    write(name, header, triples, rows_bottom_up(rows))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    color16 = [(i * 16, 255 - i * 16, (i * 40) % 256) for i in range(16)]
    gray16 = [(i * 17,) * 3 for i in range(16)]
    color20 = [((i * 13) % 256, (i * 29) % 256, (i * 47) % 256) for i in range(20)]

    indexed("pal1_color.bmp", 13, 5, 1, [(255, 0, 0), (0, 0, 255)])
    indexed("pal4_gray.bmp", 7, 4, 4, gray16)
    indexed("pal4_color.bmp", 9, 3, 4, color16)
    indexed("pal8_color_clrused.bmp", 5, 6, 8, color20, colors_used=20)
    indexed("pal8_gray_topdown.bmp", 6, 4, 8, [(i, i, i) for i in range(256)], top_down=True)
    rgb24("rgb24_odd_width.bmp", 3, 4)
    packed("rgb16_555.bmp", 5, 3, 16, (0x7C00, 0x03E0, 0x001F), compression=BI_RGB)
    packed("rgb16_565.bmp", 5, 3, 16, (0xF800, 0x07E0, 0x001F))
    packed("rgb32_unused_byte.bmp", 4, 3, 32, (0xFF0000, 0x00FF00, 0x0000FF), compression=BI_RGB,
           unused_fill=0xAB000000)
    packed("rgb32_bitfields_swapped.bmp", 4, 3, 32, (0x000000FF, 0x0000FF00, 0x00FF0000))
    packed("rgba32_v5.bmp", 4, 3, 32, (0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000),
           header_size=124)
    packed("rgb16_565_v4.bmp", 6, 2, 16, (0xF800, 0x07E0, 0x001F), header_size=108)

    gray_pal = [(i * 10,) * 3 for i in range(16)]
    # RLE8 after the example in doc/bmp/ms-bitmap-compression.html, 16 x 3 pixels.
    # Row 0: encoded runs, a padded absolute run, then a delta to (14, 1);
    # row 1 gets two pixels, the rest is skipped; row 2: absolute and encoded
    # runs, end of bitmap.
    rle8 = bytes([3, 4, 5, 6, 0, 3, 1, 2, 3, 0, 2, 7, 0, 2, 1, 1,
                  2, 7, 0, 0,
                  0, 4, 8, 9, 10, 11, 9, 1, 0, 1])
    rle("rle8.bmp", 8, 16, 3, rle8, gray_pal, [
        [8, 9, 10, 11] + [1] * 9 + [0] * 3,
        [0] * 14 + [7, 7],
        [4, 4, 4, 6, 6, 6, 6, 6, 1, 2, 3, 7, 7, 0, 0, 0],
    ])
    # RLE4, 8 x 2 pixels: encoded run, padded absolute run, end of line, a
    # short run, end of line, end of bitmap.
    rle4 = bytes([3, 0x12, 0, 5, 0x34, 0x56, 0x70, 0, 0, 0,
                  4, 0x89, 0, 0, 0, 1])
    rle("rle4.bmp", 4, 8, 2, rle4, color16, [
        [8, 9, 8, 9, 0, 0, 0, 0],
        [1, 2, 1, 3, 4, 5, 6, 7],
    ])
    core_header("core_pal8.bmp", 5, 3)

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tsamples\tcrc32\n")
        for name, samples, data in sorted(TRUTH):
            f.write(f"{name}\t{samples}\t{zlib.crc32(data):08x}\n")


if __name__ == "__main__":
    main()

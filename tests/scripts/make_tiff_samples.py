#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic TIFF files covering the variants of the format.

Written from TIFF 6.0, the BigTIFF specification and the Adobe
floating-point predictor note (doc/tiff/, doc/bigtiff/). Also writes
truth.tsv with the expected decoding of every series, computed from the
values encoded here, in the layout of tests/common/mod.rs `decode()`:
channels interleaved per pixel, planes ordered by z then t, native byte
order. The output in tests/data/tiff/ is committed.

Usage: python3 -I tests/scripts/make_tiff_samples.py
"""

import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "tiff"

# Field types
BYTE, ASCII, SHORT, LONG, RATIONAL, DOUBLE, LONG8 = 1, 2, 3, 4, 5, 12, 16
FMT = {BYTE: "B", SHORT: "H", LONG: "I", LONG8: "Q", DOUBLE: "d"}
SIZE = {BYTE: 1, ASCII: 1, SHORT: 2, LONG: 4, RATIONAL: 8, DOUBLE: 8, LONG8: 8}

TRUTH = []


# --- Codecs -----------------------------------------------------------------

def packbits(data):
    """PackBits encoder (TIFF 6.0 section 9): literal and replicate runs."""
    out, i = bytearray(), 0
    while i < len(data):
        run = 1
        while i + run < len(data) and run < 128 and data[i + run] == data[i]:
            run += 1
        if run >= 3:
            out += bytes([257 - run, data[i]])
            i += run
            continue
        start = i
        while i < len(data) and i - start < 128:
            if i + 2 < len(data) and data[i] == data[i + 1] == data[i + 2]:
                break
            i += 1
        out.append(i - start - 1)
        out += data[start:i]
    return bytes(out)


def lzw(data):
    """TIFF LZW encoder (TIFF 6.0 section 13): MSB-first codes, Clear at the
    start and before the table is full.

    Codes are written with the width the decoder uses when reading them: the
    decoder adds a table entry for every code but the first after a Clear,
    and widens the code one entry early, at 511, 1023 and 2047 entries.
    """
    out, acc, nacc = bytearray(), 0, 0
    dec = {"free": 258, "width": 9, "first": True}

    def emit(code):
        nonlocal acc, nacc
        width = dec["width"]
        acc = (acc << width) | code
        nacc += width
        while nacc >= 8:
            nacc -= 8
            out.append((acc >> nacc) & 0xFF)
        if code == 256:
            dec.update(free=258, width=9, first=True)
            return
        if not dec["first"]:
            dec["free"] += 1
            if dec["free"] == (1 << dec["width"]) - 1 and dec["width"] < 12:
                dec["width"] += 1
        dec["first"] = False

    table = {bytes([i]): i for i in range(256)}
    next_code = 258
    emit(256)
    w = b""
    for b in data:
        wc = w + bytes([b])
        if wc in table:
            w = wc
            continue
        emit(table[w])
        table[wc] = next_code
        next_code += 1
        if next_code == 4093:
            emit(256)
            table = {bytes([i]): i for i in range(256)}
            next_code = 258
        w = bytes([b])
    if w:
        emit(table[w])
    emit(257)
    if nacc:
        out.append((acc << (8 - nacc)) & 0xFF)
    return bytes(out)


COMPRESS = {1: lambda d: d, 5: lzw, 8: zlib.compress, 32773: packbits}


# --- Sample values ----------------------------------------------------------

def value(kind, bits, x, y, c, page):
    """Deterministic sample value covering the range of the type."""
    n = x * 37 + y * 71 + c * 101 + page * 53
    if kind == "float":
        return (n % 1000) / 7.0 - 50.0
    if kind == "int":
        span = 1 << (bits - 1)
        return (n * 2654435761) % (2 * span) - span
    return (n * 2654435761) % (1 << bits)


def pack_samples(values, kind, bits, little):
    """Stores sample values of one row as TIFF data."""
    e = "<" if little else ">"
    if kind == "float":
        if bits == 16:
            return b"".join(struct.pack(e + "e", v) for v in values)
        return b"".join(struct.pack(e + ("f" if bits == 32 else "d"), v) for v in values)
    if bits in (8, 16, 32) or (bits == 64):
        code = {8: "B", 16: "H", 32: "I", 64: "Q"}[bits]
        if kind == "int":
            code = code.lower()
        return b"".join(struct.pack(e + code, v) for v in values)
    # Packed MSB-first bit stream, padded to a byte per row.
    acc, n, out = 0, 0, bytearray()
    for v in values:
        acc = (acc << bits) | v
        n += bits
        while n >= 8:
            n -= 8
            out.append((acc >> n) & 0xFF)
    if n:
        out.append((acc << (8 - n)) & 0xFF)
    return bytes(out)


def native(values, out_type):
    code = {"U8": "B", "I8": "b", "U16": "H", "I16": "h", "U32": "I", "I32": "i", "F32": "f",
            "F64": "d"}[out_type]
    return struct.pack("<" + code * len(values), *values)


def out_type(kind, bits):
    if kind == "float":
        return "F32" if bits <= 32 else "F64"
    if kind == "int":
        return {8: "I8", 16: "I16", 32: "I32"}[bits]
    return "U8" if bits <= 8 else ("U16" if bits <= 16 else "U32")


# --- Writer -----------------------------------------------------------------

class Writer:
    """Collects data and IFDs and lays them out in one file."""

    def __init__(self, little=True, big=False):
        self.little, self.big = little, big
        self.e = "<" if little else ">"
        self.buf = bytearray(16 if big else 8)

    def blob(self, data):
        if len(self.buf) % 2:
            self.buf.append(0)
        offset = len(self.buf)
        self.buf += data
        return offset

    def ifd(self, tags):
        """Writes an IFD; `tags` maps tag -> (type, values). Returns its offset
        and the position of its next-IFD link."""
        e = self.e
        entries = []
        inline = 8 if self.big else 4
        for t in sorted(tags):
            typ, vals = tags[t]
            if typ == ASCII:
                raw = vals.encode() + b"\0"
                count = len(raw)
            elif typ == RATIONAL:
                raw = b"".join(struct.pack(e + "II", *r) for r in vals)
                count = len(vals)
            else:
                raw = struct.pack(e + FMT[typ] * len(vals), *vals)
                count = len(vals)
            if len(raw) <= inline:
                value = raw + bytes(inline - len(raw))
            else:
                off = self.blob(raw)
                value = struct.pack(e + ("Q" if self.big else "I"), off)
            entries.append((t, typ, count, value))
        if len(self.buf) % 2:
            self.buf.append(0)
        offset = len(self.buf)
        if self.big:
            self.buf += struct.pack(e + "Q", len(entries))
            for t, typ, count, value in entries:
                self.buf += struct.pack(e + "HHQ", t, typ, count) + value
            link = len(self.buf)
            self.buf += bytes(8)
        else:
            self.buf += struct.pack(e + "H", len(entries))
            for t, typ, count, value in entries:
                self.buf += struct.pack(e + "HHI", t, typ, count) + value
            link = len(self.buf)
            self.buf += bytes(4)
        return offset, link

    def link(self, at, offset):
        fmt = self.e + ("Q" if self.big else "I")
        struct.pack_into(fmt, self.buf, at, offset)

    def save(self, name, chain):
        e = self.e
        if self.big:
            self.buf[:16] = (b"II" if self.little else b"MM") + struct.pack(e + "HHHQ", 43, 8, 0, chain[0][0])
        else:
            self.buf[:8] = (b"II" if self.little else b"MM") + struct.pack(e + "HI", 42, chain[0][0])
        for (_, link), (nxt, _) in zip(chain, chain[1:]):
            self.link(link, nxt)
        (OUT / name).write_bytes(bytes(self.buf))


def image_tags(w, h, spp, bits, kind, chunks, counts, tiled, chunk, compression=1, photometric=1,
               planar=1, predictor=1, extra=None, big=False):
    off_type = LONG8 if big else LONG
    tags = {
        256: (LONG, [w]), 257: (LONG, [h]), 258: (SHORT, [bits] * spp), 259: (SHORT, [compression]),
        262: (SHORT, [photometric]), 277: (SHORT, [spp]), 284: (SHORT, [planar]),
        339: (SHORT, [{"uint": 1, "int": 2, "float": 3}[kind]] * spp),
        282: (RATIONAL, [(300, 1)]), 283: (RATIONAL, [(300, 1)]), 296: (SHORT, [2]),
    }
    if predictor != 1:
        tags[317] = (SHORT, [predictor])
    if tiled:
        tags[322] = (SHORT, [chunk[0]])
        tags[323] = (SHORT, [chunk[1]])
        tags[324] = (off_type, chunks)
        tags[325] = (off_type, counts)
    else:
        tags[278] = (LONG, [chunk[1]])
        tags[273] = (off_type, chunks)
        tags[279] = (off_type, counts)
    if extra:
        tags.update(extra)
    return tags


def encode_rows(rows, kind, bits, little, predictor):
    """Stored bytes for a list of rows of sample values (one chunk plane)."""
    out = bytearray()
    for row, samples in rows:
        if predictor == 2:
            diffs = list(row[:samples])
            mask = (1 << bits) - 1
            for i in range(samples, len(row)):
                diffs.append((row[i] - row[i - samples]) & mask)
            out += pack_samples(diffs, kind if kind != "int" else "uint", bits, little)
        elif predictor == 3:
            n = bits // 8
            raw = pack_samples(row, kind, bits, False)  # big-endian bytes per sample
            count = len(row)
            planes = bytearray(raw[i * n + b] for b in range(n) for i in range(count))
            for i in range(len(planes) - 1, samples - 1, -1):
                planes[i] = (planes[i] - planes[i - samples]) & 0xFF
            out += planes
        else:
            out += pack_samples(row, kind, bits, little)
    return bytes(out)


def add_image(wr, w, h, spp, bits, kind, page=0, tiled=False, chunk=None, compression=1,
              planar=1, predictor=1, photometric=1, extra=None, values=None):
    """Writes the pixel data of one image and returns its tags and the
    expected decoded values (rows of interleaved samples)."""
    values = values or (lambda x, y, c: value(kind, bits, x, y, c, page))
    chunk = chunk or ((16, 16) if tiled else (w, 3))
    cw, ch = chunk if tiled else (w, chunk[1])
    planes = spp if planar == 2 else 1
    per_chunk = 1 if planar == 2 else spp
    offsets, counts = [], []
    for plane in range(planes):
        for ty in range(0, h, ch):
            for tx in range(0, w, cw):
                rows = []
                height = ch if tiled else min(ch, h - ty)
                for y in range(ty, ty + height):
                    row = []
                    for x in range(tx, tx + cw):
                        for s in range(per_chunk):
                            c = plane if planar == 2 else s
                            inside = x < w and y < h
                            row.append(values(x, y, c) if inside else (0 if kind != "float" else 0.0))
                    rows.append((row, per_chunk))
                data = COMPRESS[compression](encode_rows(rows, kind, bits, wr.little, predictor))
                offsets.append(wr.blob(data))
                counts.append(len(data))
    tags = image_tags(w, h, spp, bits, kind, offsets, counts, tiled, (cw, ch), compression,
                      photometric, planar, predictor, extra, wr.big)
    expected = [[values(x, y, c) for x in range(w) for c in range(spp)] for y in range(h)]
    return tags, expected


def finish_value(v, kind, bits, photometric):
    if photometric == 0 and kind == "uint":
        return (1 << bits) - 1 - v
    if kind == "float" and bits == 16:
        return struct.unpack("<e", struct.pack("<e", v))[0]
    return v


def record(name, series, planes, channels, size_z, size_t, otype, levels=1, kind="uint", bits=8,
           photometric=1):
    """Adds the expected decoding of one series; `planes` is a list of
    (rows of interleaved samples) in z, t order."""
    data = b"".join(
        native([finish_value(v, kind, bits, photometric) for row in p for v in row], otype)
        for p in planes)
    TRUTH.append((name, series, channels, size_z, size_t, otype, levels,
                  f"{zlib.crc32(data):08x}"))


def single(name, w, h, spp, bits, kind, little=True, big=False, photometric=None, **kw):
    photometric = photometric if photometric is not None else (2 if spp >= 3 else 1)
    wr = Writer(little, big)
    tags, expected = add_image(wr, w, h, spp, bits, kind, photometric=photometric, **kw)
    if photometric == 2 and spp == 4:
        tags[338] = (SHORT, [2])
    chain = [wr.ifd(tags)]
    wr.save(name, chain)
    channels = 1 if photometric == 2 else spp
    samples = spp if photometric == 2 else spp
    record(name, 0, [expected], samples, 1, 1, out_type(kind, bits), kind=kind, bits=bits,
           photometric=photometric)
    return channels


def palette_rgb(name, w, h, bits, cmap, otype):
    """Palette image whose colour map is no ramp: expected as RGB."""
    wr = Writer()
    tags, expected = add_image(wr, w, h, 1, bits, "uint", photometric=3, extra={320: (SHORT, cmap)})
    wr.save(name, [wr.ifd(tags)])
    n = 1 << bits
    shift = 8 if otype == "U8" else 0
    rgb = [[cmap[k * n + v] >> shift for v in row for k in range(3)] for row in expected]
    record(name, 0, [rgb], 3, 1, 1, otype)


# --- Samples ----------------------------------------------------------------

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for f in OUT.glob("*.tif"):
        f.unlink()

    # Sample types and byte orders, uncompressed strips of 3 rows.
    single("u8_le.tif", 13, 7, 1, 8, "uint")
    single("u16_be.tif", 11, 6, 1, 16, "uint", little=False)
    single("i16_le.tif", 9, 5, 1, 16, "int")
    single("u32_be.tif", 7, 5, 1, 32, "uint", little=False)
    single("i32_le.tif", 7, 4, 1, 32, "int")
    single("f32_le.tif", 9, 4, 1, 32, "float")
    single("f64_be.tif", 6, 4, 1, 64, "float", little=False)
    single("f16_le.tif", 6, 4, 1, 16, "float")
    single("i8_be.tif", 8, 3, 1, 8, "int", little=False)
    # Sub-byte and odd bit depths, packed MSB first.
    single("u1_miniswhite.tif", 21, 5, 1, 1, "uint", photometric=0)
    single("u4_be.tif", 9, 5, 1, 4, "uint", little=False)
    single("u12_le.tif", 7, 5, 1, 12, "uint")
    single("u10_rgb_be.tif", 5, 4, 3, 10, "uint", little=False)
    # Colour layouts.
    single("rgb8_tiles.tif", 37, 21, 3, 8, "uint", tiled=True)
    single("rgb8_planar.tif", 10, 7, 3, 8, "uint", planar=2)
    single("rgb16_planar_tiles_be.tif", 20, 18, 3, 16, "uint", little=False, tiled=True, planar=2)
    single("rgba16_be.tif", 6, 5, 4, 16, "uint", little=False)
    # Multi-sample gray data: samples become channels.
    single("gray2_chunky.tif", 8, 5, 2, 16, "uint", photometric=1)
    single("gray3_planar.tif", 8, 5, 3, 8, "uint", photometric=1, planar=2)
    # Compression and predictors.
    single("u8_packbits.tif", 40, 9, 1, 8, "uint", compression=32773,
           values=lambda x, y, c: (x // 7 + y) % 3 * 50)
    single("u8_lzw.tif", 50, 30, 1, 8, "uint", compression=5)
    single("rgb8_lzw_pred.tif", 33, 20, 3, 8, "uint", compression=5, predictor=2)
    single("u16_deflate_pred.tif", 30, 20, 1, 16, "uint", compression=8, predictor=2)
    single("u16_deflate_pred_be_tiles.tif", 30, 20, 1, 16, "uint", little=False, compression=8,
           predictor=2, tiled=True)
    single("f32_deflate_fpred.tif", 30, 9, 1, 32, "float", compression=8, predictor=3)
    single("f64_deflate_fpred_rgb.tif", 7, 6, 3, 64, "float", compression=8, predictor=3,
           little=False)
    single("u16_lzw_big_table.tif", 300, 200, 1, 16, "uint", compression=5, chunk=(300, 200))
    # BigTIFF.
    single("bigtiff_u16_tiles.tif", 40, 33, 1, 16, "uint", big=True, tiled=True, compression=8)
    single("bigtiff_rgb8_be.tif", 9, 8, 3, 8, "uint", big=True, little=False)

    # Multi-page: pages become time points.
    wr = Writer()
    chain, planes = [], []
    for page in range(3):
        tags, expected = add_image(wr, 10, 6, 1, 16, "uint", page=page)
        chain.append(wr.ifd(tags))
        planes.append(expected)
    wr.save("pages3_u16.tif", chain)
    record("pages3_u16.tif", 0, planes, 1, 1, 3, "U16")

    # Two series: pages of different size.
    wr = Writer()
    chain = []
    t1, e1 = add_image(wr, 12, 8, 1, 8, "uint")
    t2, e2 = add_image(wr, 5, 4, 3, 8, "uint", page=1, photometric=2)
    chain = [wr.ifd(t1), wr.ifd(t2)]
    wr.save("two_series.tif", chain)
    record("two_series.tif", 0, [e1], 1, 1, 1, "U8")
    record("two_series.tif", 1, [e2], 3, 1, 1, "U8")

    # Palettes. A gray or single-colour ramp returns the indices as stored
    # (colour map as LUT); any other colour map is expanded to RGB, as u8 if
    # every entry is an 8-bit value times 257, otherwise as u16.
    cmap = [i * 257 for i in range(256)] + [(255 - i) * 257 for i in range(256)] + \
        [(i * 7 % 256) * 257 for i in range(256)]
    palette_rgb("palette8.tif", 9, 5, 8, cmap, "U8")
    cmap = [i * 251 for i in range(256)] + [65535 - i * 3 for i in range(256)] + \
        [(i * 7 % 256) * 199 for i in range(256)]
    palette_rgb("palette8_rgb16.tif", 9, 5, 8, cmap, "U16")
    cmap = [i * 0x1111 for i in range(16)] * 3
    single("palette4_gray.tif", 9, 5, 1, 4, "uint", photometric=3, extra={320: (SHORT, cmap)})
    cmap = [0] * 256 + [i * 257 for i in range(256)] + [0] * 256
    single("palette8_green.tif", 9, 5, 1, 8, "uint", photometric=3, extra={320: (SHORT, cmap)})

    # Pyramid in SubIFDs: full resolution with two reduced levels.
    wr = Writer()
    full_tags, full = add_image(wr, 32, 24, 1, 8, "uint", tiled=True)
    subs = []
    for level, (w, h) in enumerate([(16, 12), (8, 6)], start=1):
        tags, _ = add_image(wr, w, h, 1, 8, "uint", tiled=True,
                            values=lambda x, y, c, level=level: (x * 2 ** level + y) % 256)
        tags[254] = (LONG, [1])
        subs.append(wr.ifd(tags)[0])
    full_tags[330] = (LONG, subs)
    chain = [wr.ifd(full_tags)]
    wr.save("pyramid_subifd.tif", chain)
    record("pyramid_subifd.tif", 0, [full], 1, 1, 1, "U8", levels=3)

    # Pyramid as reduced-resolution IFDs in the main chain.
    wr = Writer()
    full_tags, full = add_image(wr, 32, 24, 1, 8, "uint", tiled=True)
    chain = [wr.ifd(full_tags)]
    for w, h in [(16, 12), (8, 6)]:
        tags, _ = add_image(wr, w, h, 1, 8, "uint", tiled=True)
        tags[254] = (LONG, [1])
        chain.append(wr.ifd(tags))
    wr.save("pyramid_reduced.tif", chain)
    record("pyramid_reduced.tif", 0, [full], 1, 1, 1, "U8", levels=3)

    # ImageJ hyperstack: 2 channels, 3 slices, 2 frames.
    c, z, t = 2, 3, 2
    wr = Writer()
    chain, pages = [], []
    desc = (f"ImageJ=1.54f\nimages={c * z * t}\nchannels={c}\nslices={z}\nframes={t}\n"
            "hyperstack=true\nunit=micron\nspacing=0.25\nfinterval=2.5\n")
    for page in range(c * z * t):
        extra = {270: (ASCII, desc), 296: (SHORT, [1]), 282: (RATIONAL, [(5, 1)]),
                 283: (RATIONAL, [(5, 1)])} if page == 0 else None
        tags, expected = add_image(wr, 8, 6, 1, 16, "uint", page=page, extra=extra)
        chain.append(wr.ifd(tags))
        pages.append(expected)
    wr.save("imagej_hyperstack.tif", chain)
    # Channels interleaved per pixel, planes by z then t.
    planes = []
    for ti in range(t):
        for zi in range(z):
            chans = [pages[ci + c * (zi + z * ti)] for ci in range(c)]
            planes.append([[chans[ci][y][x] for x in range(8) for ci in range(c)]
                           for y in range(6)])
    record("imagej_hyperstack.tif", 0, planes, c, z, t, "U16")

    # One image stored four ways; readers must return identical buffers.
    for name, kw in [
        ("same_rgb16_le_chunky.tif", dict()),
        ("same_rgb16_be_planar.tif", dict(little=False, planar=2)),
        ("same_rgb16_le_tiles_deflate_pred.tif", dict(tiled=True, compression=8, predictor=2)),
        ("same_rgb16_be_bigtiff_lzw.tif", dict(little=False, big=True, compression=5)),
    ]:
        single(name, 23, 17, 3, 16, "uint", **kw)

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tchannels\tsize_z\tsize_t\tpixel_type\tlevels\tcrc32\n")
        for row in TRUTH:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

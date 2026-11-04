#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Becker & Hickl SDT files.

Layout from sdtfile (BSD-3-Clause, Christoph Gohlke; read as
documentation): 42-byte FILE_HEADER (revision, info, setup, data block and
measurement description offsets and counts, header_valid 0x5555, chksum
0x55AA), an *IDENTIFICATION info text, MEASURE_INFO records (adc_re at 82,
tac_r at 64, tac_g at 68, scan_x/y at 173/177, image_x/y at 309/313) and
22-byte block headers (revision >= 15: data_offs_ext, next_block_offs_ext,
data_offs, next_block_offs, block_type, meas_desc_block_no, lblock_no,
block_length). Image blocks are y, x, time bin (fastest), uint16.

- image.sdt: revision 15, two image blocks of 7 x 5 x 8 bins: block 0
  uncompressed, block 1 a ZIP archive with one deflated member.
- decay.sdt: revision 12 (old block header), one decay curve of 16 bins.

Also writes truth.tsv with the CRC-32 of each block in the layout of
tests/common/mod.rs `decode_level()` (bins as time points). Output in
tests/data/sdt/ is committed.

Usage: python3 -I tests/scripts/make_sdt_samples.py
"""

import io
import shutil
import struct
import zipfile
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "sdt"
W, H, BINS = 7, 5, 8
INFO = "*IDENTIFICATION\r\n  ID        : SPC Setup & Data File\r\n  Title     : synthetic\r\n*END\r\n\r\n"
MEAS_LEN = 512


def measure(adc_re, image=(0, 0), scan=(0, 0)):
    m = bytearray(MEAS_LEN)
    m[0:8] = b"10:11:12"
    m[9:19] = b"2026-11-04"
    struct.pack_into("<f", m, 64, 5e-8)
    struct.pack_into("<h", m, 68, 4)
    struct.pack_into("<h", m, 82, adc_re)
    struct.pack_into("<ii", m, 173, *scan)
    struct.pack_into("<ii", m, 309, *image)
    return bytes(m)


def sdt(revision, measures, blocks, new_header):
    """`blocks`: list of (block_type, meas index, stored bytes, uncompressed length)."""
    info = INFO.encode("cp1250")
    meas_at = 42 + len(info)
    data_at = meas_at + MEAS_LEN * len(measures)
    out = bytearray(struct.pack("<hihiHihIihhHIHH", revision, 42, len(info), 0, 0, data_at, len(blocks), 0,
                                meas_at, len(measures), MEAS_LEN, 0x5555, 0, 0, 0x55AA))
    out += info + b"".join(measures)
    at = data_at
    for k, (btype, mi, stored, length) in enumerate(blocks):
        data = at + 22
        nxt = data + len(stored) if k + 1 < len(blocks) else 0
        if new_header:
            out += struct.pack("<BBIIHhII", 0, 0, data, nxt, btype, mi, k, length)
        else:
            out += struct.pack("<hiiHhII", k, data, nxt, btype, mi, k, length)
        out += stored
        at = data + len(stored)
    return bytes(out)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    v = lambda b, y, x, t: (b * 1000 + y * 97 + x * 13 + t * 5) % 65536
    raw = [b"".join(struct.pack("<H", v(b, y, x, t)) for y in range(H) for x in range(W) for t in range(BINS)) for b in range(2)]
    zbuf = io.BytesIO()
    with zipfile.ZipFile(zbuf, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("data_block", raw[1])
    # IMG_BLOCK (0x60) | MEAS_DATA (1), ZIP compression bit 12.
    blocks = [(0x61, 0, raw[0], len(raw[0])), (0x1061, 1, zbuf.getvalue(), len(raw[1]))]
    measures = [measure(BINS, image=(W, H)), measure(BINS, scan=(W, H))]
    (OUT / "image.sdt").write_bytes(sdt(0x10F, measures, blocks, True))
    for b in range(2):
        px = b"".join(struct.pack("<H", v(b, y, x, t)) for t in range(BINS) for y in range(H) for x in range(W))
        truth.append(("image.sdt", b, zlib.crc32(px)))

    d = lambda t: (t * 37 + 3) % 65536
    curve = b"".join(struct.pack("<H", d(t)) for t in range(16))
    # DECAY_BLOCK (0x00) | MEAS_DATA (1), revision 12.
    (OUT / "decay.sdt").write_bytes(sdt(0x10C, [measure(16)], [(0x01, 0, curve, len(curve))], False))
    truth.append(("decay.sdt", 0, zlib.crc32(curve)))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for name, s, crc in truth:
            f.write(f"{name}\t{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

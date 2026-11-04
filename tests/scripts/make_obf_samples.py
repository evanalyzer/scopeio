#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic Imspector OBF files.

Layout from the Imspector "File Formats" documentation
(doc/imspector-obf/): file header (OMAS_BF magic, version, first stack
position, description), stack headers of 15 axes, name, data, footer of
1468 bytes (has_col_positions/labels, metadata length, SI units of value
and axes, flush points, tag dictionary length, stack end, minimum version,
samples written, chunk positions), then the axis labels.

- two.obf: two stacks. Stack 0: uint16, rank 3, 7 x 5 x 2, axes X, Y and a
  channel axis "Detector" without unit, 0.1 um pixels, uncompressed,
  version 6 with one chunk position (the data split in two parts with a gap).
  Stack 1: float32, rank 2, 7 x 5, zlib-compressed, version 4, X and Y.

Also writes truth.tsv with the CRC-32 of each stack in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/obf/ is committed.

Usage: python3 -I tests/scripts/make_obf_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "obf"
N = 15
W, H = 7, 5


def si(metre=0, second=0):
    exps = [(metre, 1), (0, 1), (second, 1)] + [(0, 1)] * 6
    return b"".join(struct.pack("<ii", *e) for e in exps) + struct.pack("<d", 1.0)


def footer(rank, units, samples, chunks, labels):
    f = struct.pack("<I", 1468) + bytes(4 * N) + bytes(4 * N) + struct.pack("<I", 0)
    f += si() + b"".join(si(*units[i]) if i < rank else si() for i in range(N))
    f += struct.pack("<QQQQIQQQ", 0, 0, 0, 0, 1, 0, samples, len(chunks))
    assert len(f) == 1468
    f += b"".join(struct.pack("<I", len(l)) + l.encode() for l in labels)
    f += b"".join(struct.pack("<QQ", *c) for c in chunks)
    return f


def stack_header(version, res, lens, dt, compression, name, data_len, next_pos):
    r = list(res) + [0] * (N - len(res))
    l = list(lens) + [0.0] * (N - len(lens))
    h = b"OMAS_BF_STACK\n\xff\xff" + struct.pack("<II", version, len(res)) + struct.pack(f"<{N}I", *r)
    h += struct.pack(f"<{N}d", *l) + struct.pack(f"<{N}d", *([0.0] * N))
    h += struct.pack("<5I", dt, compression, 6 if compression else 0, len(name), 0)
    h += struct.pack("<QQQ", 0, data_len, next_pos)
    return h + name.encode()


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []
    out = bytearray(b"OMAS_BF\n\xff\xff" + struct.pack("<IQI", 2, 0, 0) + struct.pack("<Q", 0))
    first = len(out)
    struct.pack_into("<Q", out, 14, first)

    # Stack 0: uint16 with a channel axis, data in two chunks with a gap.
    v = lambda x, y, c: x * 401 + y * 37 + c * 9000
    data = b"".join(struct.pack("<H", v(x, y, c)) for c in range(2) for y in range(H) for x in range(W))
    split, gap = 30, 6
    body = data[:split] + bytes(gap) + data[split:]
    name = "Detector stack"
    head_len = len(stack_header(6, [W, H, 2], [0, 0, 0], 4, 0, name, 0, 0))
    stack1_at = first + head_len + len(body) + len(footer(3, [(1, 0), (1, 0), (0, 0)], W * H * 2, [(split, split + gap)], ["X", "Y", "Detector"]))
    out += stack_header(6, [W, H, 2], [W * 1e-7, H * 1e-7, 0.0], 4, 0, name, len(body), stack1_at)
    out += body + footer(3, [(1, 0), (1, 0), (0, 0)], W * H * 2, [(split, split + gap)], ["X", "Y", "Detector"])
    truth.append((0, zlib.crc32(b"".join(struct.pack("<H", v(x, y, c)) for y in range(H) for x in range(W) for c in range(2)))))

    # Stack 1: float32, zlib.
    assert len(out) == stack1_at
    f = lambda x, y: x * 0.5 - y * 1.25
    raw = b"".join(struct.pack("<f", f(x, y)) for y in range(H) for x in range(W))
    comp = zlib.compress(raw)
    out += stack_header(4, [W, H], [W * 2e-7, H * 2e-7], 0x40, 1, "Float stack", len(comp), 0)
    out += comp + footer(2, [(1, 0), (1, 0)], W * H, [], ["X", "Y"])
    truth.append((1, zlib.crc32(raw)))

    (OUT / "two.obf").write_bytes(bytes(out))
    with open(OUT / "truth.tsv", "w") as t:
        t.write("series\tcrc32\n")
        for s, crc in truth:
            t.write(f"{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

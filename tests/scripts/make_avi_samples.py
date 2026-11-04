#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic AVI files.

Layout from the Microsoft AVI RIFF File Reference and the OpenDML AVI File
Format Extensions 1.02 (doc/avi/), BITMAPINFOHEADER and RLE from the bitmap
pages (doc/bmp/); Motion JPEG frames by a small baseline encoder below
(ITU-T T.81: standard quantisation tables K.1 and Huffman tables K.3).

Files (3 frames unless noted, idx1 offsets relative to 'movi'):
- gray8.avi: 8-bit, gray palette, 5 x 4 (rows padded to 8 bytes).
- palette8.avi: 8-bit, colour palette, 6 x 3.
- bgr24.avi: 24-bit bottom-up, 5 x 4; avih.dwTotalFrames is wrong (99).
- topdown24.avi: 24-bit top-down (negative height), 5 x 4.
- bgra32.avi: 32-bit, alpha all 255 (dropped), 4 x 3; fccHandler 'DIB '.
- y800.avi: 'Y800' gray, 5 x 4, top-down, rows not padded.
- mjpeg.avi: 'MJPG' 32 x 16 4:2:0 colour with DHT, fccHandler 'DIB '.
- mjpeg_nodht.avi: the same frames without DHT segments.
- mjpeg_gray.avi: 'MJPG' gray 16 x 16 without DHT.
- rle8.avi, rle4.avi: MSRLE, 8 x 4, key frame then delta frames (only
  the first frame flagged as key frame).
- noidx.avi: 8-bit gray, no idx1, frames grouped in LIST 'rec '.
- absidx.avi: 8-bit gray, idx1 offsets absolute; an audio stream too.
- dropped.avi: 8-bit gray, 4 frames, the third an empty chunk.
- opendml.avi: 8-bit gray 4 x 2, 5 frames: indx super index, ix00 chunks
  in RIFF 'AVI ' (2 frames, also idx1) and RIFF 'AVIX' (3 frames).
- h264.avi: 'H264' stream (stub data): must be rejected.
- truncated.avi: bgr24.avi cut in its second frame.

Also writes truth.tsv: frames, samples per pixel, CRC-32 of all frames in
the layout of tests/common/mod.rs `decode_level()` (or '-' for Motion
JPEG) and the mean sample value. Output in tests/data/avi/ is committed.

Usage: python3 -I tests/scripts/make_avi_samples.py [--bench DIR]
  --bench DIR also writes large benchmark files (not committed) to DIR.
"""

import math
import shutil
import struct
import sys
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "avi"


def fcc(s):
    return s.encode("latin-1") if isinstance(s, str) else s


def chunk(ck, data):
    return fcc(ck) + struct.pack("<I", len(data)) + data + (b"\0" if len(data) % 2 else b"")


def lst(kind, typ, body):
    return fcc(kind) + struct.pack("<I", len(body) + 4) + fcc(typ) + body


# --- Baseline JPEG encoder ---------------------------------------------------

ZIGZAG = [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21,
          28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54,
          47, 55, 62, 63]
Q_LUMA = [16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51,
          87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101,
          72, 92, 95, 98, 112, 100, 103, 99]
Q_CHROMA = [17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99, 99,
            99, 99, 99, 99] + [99] * 32
DC_LUMA = ([0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0], list(range(12)))
DC_CHROMA = ([0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0], list(range(12)))


def _ac(prefix, ranges):
    return prefix + [v for lo, hi in ranges for v in range(lo, hi + 1)]


AC_LUMA = ([0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d], _ac(
    [0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32,
     0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16,
     0x17, 0x18, 0x19, 0x1a],
    [(0x25, 0x2a), (0x34, 0x3a), (0x43, 0x4a), (0x53, 0x5a), (0x63, 0x6a), (0x73, 0x7a), (0x83, 0x8a), (0x92, 0x9a),
     (0xa2, 0xaa), (0xb2, 0xba), (0xc2, 0xca), (0xd2, 0xda), (0xe1, 0xea), (0xf1, 0xfa)]))
AC_CHROMA = ([0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77], _ac(
    [0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81,
     0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34,
     0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a],
    [(0x26, 0x2a), (0x35, 0x3a), (0x43, 0x4a), (0x53, 0x5a), (0x63, 0x6a), (0x73, 0x7a), (0x82, 0x8a), (0x92, 0x9a),
     (0xa2, 0xaa), (0xb2, 0xba), (0xc2, 0xca), (0xd2, 0xda), (0xe2, 0xea), (0xf2, 0xfa)]))
assert len(AC_LUMA[1]) == 162 and len(AC_CHROMA[1]) == 162


def huff_codes(table):
    """Code (value -> (code, length)) of T.81 Annex C."""
    bits, vals = table
    codes, code, k = {}, 0, 0
    for length in range(1, 17):
        for _ in range(bits[length - 1]):
            codes[vals[k]] = (code, length)
            code += 1
            k += 1
        code <<= 1
    return codes


COS = [[math.cos((2 * x + 1) * u * math.pi / 16) for x in range(8)] for u in range(8)]


def fdct(block):
    out = []
    for v in range(8):
        for u in range(8):
            s = sum(block[y * 8 + x] * COS[u][x] * COS[v][y] for y in range(8) for x in range(8))
            cu = math.sqrt(0.5) if u == 0 else 1.0
            cv = math.sqrt(0.5) if v == 0 else 1.0
            out.append(0.25 * cu * cv * s)
    return out


class Bits:
    def __init__(self):
        self.out, self.acc, self.n = bytearray(), 0, 0

    def put(self, code, length):
        self.acc = (self.acc << length) | code
        self.n += length
        while self.n >= 8:
            self.n -= 8
            b = (self.acc >> self.n) & 0xFF
            self.out.append(b)
            if b == 0xFF:
                self.out.append(0)

    def flush(self):
        if self.n:
            self.put((1 << (8 - self.n)) - 1, 8 - self.n)
        return bytes(self.out)


def magnitude(v):
    n = abs(v).bit_length()
    return n, (v if v >= 0 else v + (1 << n) - 1)


def encode_block(bits, block, q, dc, ac, pred):
    coef = fdct([v - 128 for v in block])
    z = [int(round(coef[ZIGZAG[i]] / q[ZIGZAG[i]])) for i in range(64)]
    diff = z[0] - pred
    n, m = magnitude(diff)
    bits.put(*dc[n])
    if n:
        bits.put(m, n)
    run = 0
    for i in range(1, 64):
        if z[i] == 0:
            run += 1
            continue
        while run > 15:
            bits.put(*ac[0xF0])
            run -= 16
        n, m = magnitude(z[i])
        bits.put(*ac[(run << 4) | n])
        bits.put(m, n)
        run = 0
    if run:
        bits.put(*ac[0x00])
    return z[0]


def jpeg(width, height, pixels, gray, dht=True):
    """Baseline JPEG of `pixels` (gray bytes or RGB triples, top row first)."""
    dcl, acl, dcc, acc = (huff_codes(t) for t in (DC_LUMA, AC_LUMA, DC_CHROMA, AC_CHROMA))
    if gray:
        planes = [list(pixels)]
    else:
        rgb = [pixels[i:i + 3] for i in range(0, len(pixels), 3)]
        ycc = [(0.299 * r + 0.587 * g + 0.114 * b, -0.168736 * r - 0.331264 * g + 0.5 * b + 128,
                0.5 * r - 0.418688 * g - 0.081312 * b + 128) for r, g, b in rgb]
        planes = [[c[k] for c in ycc] for k in range(3)]
    bits = Bits()
    preds = [0, 0, 0]

    def block(plane, w, bx, by, step):
        # 8 x 8 samples; chroma averaged over 2 x 2 for 4:2:0.
        out = []
        for y in range(8):
            for x in range(8):
                if step == 1:
                    out.append(plane[(by + y) * w + bx + x])
                else:
                    xs, ys = (bx + x) * 2, (by + y) * 2
                    out.append(sum(plane[(ys + j) * w + xs + i] for j in range(2) for i in range(2)) / 4)
        return out

    if gray:
        for by in range(0, height, 8):
            for bx in range(0, width, 8):
                preds[0] = encode_block(bits, block(planes[0], width, bx, by, 1), Q_LUMA, dcl, acl, preds[0])
    else:
        for my in range(0, height, 16):
            for mx in range(0, width, 16):
                for dy in (0, 8):
                    for dx in (0, 8):
                        preds[0] = encode_block(bits, block(planes[0], width, mx + dx, my + dy, 1), Q_LUMA, dcl, acl, preds[0])
                for k in (1, 2):
                    preds[k] = encode_block(bits, block(planes[k], width, mx // 2, my // 2, 2), Q_CHROMA, dcc, acc, preds[k])
    data = bits.flush()
    out = bytearray(b"\xFF\xD8")
    out += b"\xFF\xE0" + struct.pack(">H", 16) + b"AVI1" + bytes(10)
    out += b"\xFF\xDB" + struct.pack(">H", 67) + b"\x00" + bytes(Q_LUMA[ZIGZAG[i]] for i in range(64))
    if not gray:
        out += b"\xFF\xDB" + struct.pack(">H", 67) + b"\x01" + bytes(Q_CHROMA[ZIGZAG[i]] for i in range(64))
    comps = [(1, 0x11, 0)] if gray else [(1, 0x22, 0), (2, 0x11, 1), (3, 0x11, 1)]
    out += b"\xFF\xC0" + struct.pack(">HBHHB", 8 + 3 * len(comps), 8, height, width, len(comps))
    out += b"".join(struct.pack("BBB", *c) for c in comps)
    if dht:
        tables = [(0x00, DC_LUMA), (0x10, AC_LUMA)] + ([] if gray else [(0x01, DC_CHROMA), (0x11, AC_CHROMA)])
        for tc, (b, v) in tables:
            out += b"\xFF\xC4" + struct.pack(">HB", 3 + 16 + len(v), tc) + bytes(b) + bytes(v)
    sel = [(1, 0x00)] if gray else [(1, 0x00), (2, 0x11), (3, 0x11)]
    out += b"\xFF\xDA" + struct.pack(">HB", 6 + 2 * len(sel), len(sel)) + b"".join(struct.pack("BB", *s) for s in sel) + b"\x00\x3F\x00"
    out += data + b"\xFF\xD9"
    return bytes(out)


# --- RLE encoder -------------------------------------------------------------

def rle_encode(rows, bits, prev=None):
    """MSRLE of palette index rows (bottom row first in the stream). With
    `prev`, unchanged pixels are skipped by deltas (a delta frame)."""
    h, w = len(rows), len(rows[0])
    out = bytearray()
    for y in range(h - 1, -1, -1):
        row = rows[y]
        x = 0
        while x < w:
            if prev is not None and prev[y][x] == row[x]:
                skip = 0
                while x + skip < w and prev[y][x + skip] == row[x + skip]:
                    skip += 1
                if x + skip == w:
                    break
                out += bytes([0, 2, skip, 0])
                x += skip
                continue
            n = 1
            limit = w - x
            if bits == 8:
                while n < limit and row[x + n] == row[x] and n < 255:
                    n += 1
                out += bytes([n, row[x]])
            else:
                # Two alternating nibbles; here runs of one value.
                while n < limit and row[x + n] == row[x] and n < 255:
                    n += 1
                out += bytes([n, (row[x] << 4) | row[x]])
            x += n
        out += b"\x00\x00"
    out[-2:] = b"\x00\x01"
    return bytes(out)


# --- AVI writer ----------------------------------------------------------------

def bih(width, height, bits, compression, image_size=0, colors=0):
    comp = struct.unpack("<I", fcc(compression))[0] if isinstance(compression, str) else compression
    return struct.pack("<IiiHHIIiiII", 40, width, height, 1, bits, comp, image_size, 0, 0, colors, 0)


def strh(kind, handler, scale, rate, length, width, height, sample_size=0):
    return struct.pack("<4s4sIHHIIIIIIII4h", fcc(kind), fcc(handler), 0, 0, 0, 0, scale, rate, 0, length, 0, 0xFFFFFFFF,
                       sample_size, 0, 0, width, abs(height))


def avih(width, height, frames, streams, us=100000):
    return struct.pack("<10I4I", us, 0, 0, 0x10, frames, 0, streams, 0, width, abs(height), 0, 0, 0, 0)


def avi(width, height, strf, frames, handler="DIB ", kind="db", index="movi", total=None, rec=False,
        audio=False, key=None, scale=1, rate=10, name=b"video"):
    """An AVI with one video stream; `frames` are chunk payloads."""
    n = len(frames)
    strls = lst("LIST", "strl", chunk("strh", strh("vids", handler, scale, rate, n, width, height)) + chunk("strf", strf)
                + chunk("strn", name + b"\0"))
    if audio:
        wave = struct.pack("<HHIIHH", 1, 1, 8000, 8000, 1, 8)
        strls += lst("LIST", "strl", chunk("strh", strh("auds", "\0\0\0\0", 1, 8000, 800, 0, 0, 1)) + chunk("strf", wave))
    hdrl = lst("LIST", "hdrl", chunk("avih", avih(width, height, n if total is None else total, 2 if audio else 1)) + strls)
    info = lst("LIST", "INFO", chunk("ISFT", b"make_avi_samples\0"))
    junk = chunk("JUNK", bytes(12))
    head = b"RIFF\0\0\0\0AVI " + hdrl + info + junk
    movi_at = len(head) + 8  # position of the 'movi' fourcc
    body, entries = bytearray(), []
    ck = "00" + kind
    for i, f in enumerate(frames):
        flags = 0x10 if key is None or key(i) else 0
        if rec:
            body += lst("LIST", "rec ", chunk(ck, f))
            off = movi_at + 4 + len(body) - 8 - len(f) - len(f) % 2
        else:
            off = movi_at + 4 + len(body)
            body += chunk(ck, f)
        entries.append((ck, flags, off, len(f)))
        if audio:
            a = bytes(range(80))
            off = movi_at + 4 + len(body)
            body += chunk("01wb", a)
            entries.append(("01wb", 0x10, off, len(a)))
    movi = lst("LIST", "movi", bytes(body))
    out = bytearray(head + movi)
    if index in ("movi", "absolute"):
        base = movi_at if index == "movi" else 0
        idx = b"".join(struct.pack("<4sIII", fcc(c), fl, off - base, ln) for c, fl, off, ln in entries)
        out += chunk("idx1", idx)
    struct.pack_into("<I", out, 4, len(out) - 8)
    return bytes(out)


def opendml(width, height, frames, split):
    """OpenDML file: frames[:split] in RIFF 'AVI ', the rest in RIFF 'AVIX';
    one ix00 per movi list, indexed by the indx super index."""
    strf = bih(width, height, 8, 0) + b"".join(bytes([v, v, v, 0]) for v in range(256))
    n = len(frames)
    # indx with room for two entries (patched below).
    indx_len = 24 + 2 * 16
    strl_body = chunk("strh", strh("vids", "DIB ", 1, 25, n, width, height)) + chunk("strf", strf) + chunk("indx", bytes(indx_len))
    strl = lst("LIST", "strl", strl_body)
    odml = lst("LIST", "odml", chunk("dmlh", struct.pack("<I", n) + bytes(244)))
    hdrl = lst("LIST", "hdrl", chunk("avih", avih(width, height, split, 1, 40000)) + strl + odml)
    out = bytearray(b"RIFF\0\0\0\0AVI " + hdrl)
    indx_at = out.find(b"indx") + 8
    supers = []

    def movi_list(part):
        nonlocal out
        list_at = len(out)
        movi_fcc = list_at + 8
        body = bytearray()
        offsets = []
        for f in part:
            offsets.append(movi_fcc + 4 + len(body) + 8)
            body += chunk("00db", f)
        base = offsets[0] - 8 if offsets else 0
        ix_entries = b"".join(struct.pack("<II", o - base, len(f)) for o, f in zip(offsets, part))
        ix = struct.pack("<HBBI4sQI", 2, 0, 1, len(part), b"00db", base, 0) + ix_entries
        ix_at = movi_fcc + 4 + len(body)
        body += chunk("ix00", ix)
        out += lst("LIST", "movi", bytes(body))
        supers.append((ix_at, 8 + len(ix), len(part)))
        return movi_fcc, offsets

    movi_fcc, offsets = movi_list(frames[:split])
    idx = b"".join(struct.pack("<4sIII", b"00db", 0x10, o - 8 - movi_fcc, len(f)) for o, f in zip(offsets, frames[:split]))
    out += chunk("idx1", idx)
    struct.pack_into("<I", out, 4, len(out) - 8)
    avix_at = len(out)
    out += b"RIFF\0\0\0\0AVIX"
    movi_list(frames[split:])
    struct.pack_into("<I", out, avix_at + 4, len(out) - avix_at - 8)
    sup = struct.pack("<HBBI4s3I", 4, 0, 0, len(supers), b"00db", 0, 0, 0) + b"".join(struct.pack("<QII", a, s, d) for a, s, d in supers)
    out[indx_at:indx_at + len(sup)] = sup
    return bytes(out)


# --- Samples -------------------------------------------------------------------

def stored_rows(rows, row_bytes, top_down=False, pad=True):
    """DIB rows (bottom-up unless top_down), padded to 4 bytes."""
    stride = (row_bytes + 3) // 4 * 4 if pad else row_bytes
    order = rows if top_down else rows[::-1]
    return b"".join(bytes(r) + bytes(stride - len(r)) for r in order)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    def record(name, frames, samples, pixels, crc=True):
        flat = b"".join(pixels)
        mean = sum(flat) / len(flat)
        truth.append((name, frames, samples, f"{zlib.crc32(flat):08x}" if crc else "-", f"{mean:.3f}"))

    gray_lut = b"".join(bytes([v, v, v, 0]) for v in range(256))
    g = lambda x, y, t: (x * 40 + y * 13 + t * 50) % 256

    # 8-bit gray palette, 5 x 4.
    W, H = 5, 4
    frames = [[[g(x, y, t) for x in range(W)] for y in range(H)] for t in range(3)]
    (OUT / "gray8.avi").write_bytes(avi(W, H, bih(W, H, 8, 0) + gray_lut, [stored_rows(f, W) for f in frames]))
    record("gray8.avi", 3, 1, [bytes(v for r in f for v in r) for f in frames])

    # 8-bit colour palette, 6 x 3.
    pal = [((i * 7) % 256, (i * 31) % 256, (255 - i)) for i in range(256)]
    lut = b"".join(bytes([b, gg, r, 0]) for r, gg, b in pal)
    frames = [[[(x * 50 + y * 9 + t * 3) % 256 for x in range(6)] for y in range(3)] for t in range(3)]
    (OUT / "palette8.avi").write_bytes(avi(6, 3, bih(6, 3, 8, 0) + lut, [stored_rows(f, 6) for f in frames]))
    record("palette8.avi", 3, 3, [bytes(c for r in f for v in r for c in pal[v]) for f in frames])

    # 24-bit bottom-up and top-down, 5 x 4.
    rgb = lambda x, y, t: ((x * 50 + t * 20) % 256, (y * 60) % 256, (x * y * 10 + t) % 256)
    frames = [[[rgb(x, y, t) for x in range(W)] for y in range(H)] for t in range(3)]
    bgr = lambda f: [[c for p in r for c in (p[2], p[1], p[0])] for r in f]
    out_rgb = [bytes(c for r in f for p in r for c in p) for f in frames]
    (OUT / "bgr24.avi").write_bytes(avi(W, H, bih(W, H, 24, 0), [stored_rows(bgr(f), W * 3) for f in frames], total=99))
    record("bgr24.avi", 3, 3, out_rgb)
    (OUT / "topdown24.avi").write_bytes(avi(W, -H, bih(W, -H, 24, 0), [stored_rows(bgr(f), W * 3, top_down=True) for f in frames]))
    record("topdown24.avi", 3, 3, out_rgb)

    # 32-bit BGRA, alpha 255, 4 x 3.
    frames = [[[rgb(x, y, t) for x in range(4)] for y in range(3)] for t in range(3)]
    bgra = lambda f: [[c for p in r for c in (p[2], p[1], p[0], 255)] for r in f]
    (OUT / "bgra32.avi").write_bytes(avi(4, 3, bih(4, 3, 32, 0), [stored_rows(bgra(f), 16) for f in frames]))
    record("bgra32.avi", 3, 3, [bytes(c for r in f for p in r for c in p) for f in frames])

    # Y800, top-down, no padding.
    frames = [[[g(x, y, t) for x in range(W)] for y in range(H)] for t in range(3)]
    (OUT / "y800.avi").write_bytes(avi(W, H, bih(W, H, 8, "Y800"), [stored_rows(f, W, top_down=True, pad=False) for f in frames],
                                       handler="Y800", kind="dc"))
    record("y800.avi", 3, 1, [bytes(v for r in f for v in r) for f in frames])

    # Motion JPEG.
    MW, MH = 32, 16
    img = lambda t: [c for y in range(MH) for x in range(MW) for c in
                     ((x * 8 + t * 30) % 256, (y * 16) % 256, 128 + int(60 * math.sin((x + y + t) / 4)))]
    src = [img(t) for t in range(3)]
    for name, dht in (("mjpeg.avi", True), ("mjpeg_nodht.avi", False)):
        jp = [jpeg(MW, MH, f, False, dht) for f in src]
        (OUT / name).write_bytes(avi(MW, MH, bih(MW, MH, 24, "MJPG", MW * MH * 3), jp, kind="dc"))
        record(name, 3, 3, [bytes(f) for f in src], crc=False)
    gsrc = [[(x * 16 + y * 8 + t * 40) % 256 for y in range(16) for x in range(16)] for t in range(2)]
    jp = [jpeg(16, 16, f, True, False) for f in gsrc]
    (OUT / "mjpeg_gray.avi").write_bytes(avi(16, 16, bih(16, 16, 8, "MJPG", 256), jp, handler="MJPG", kind="dc"))
    record("mjpeg_gray.avi", 2, 1, [bytes(f) for f in gsrc], crc=False)

    # MSRLE: key frame, then deltas.
    RW, RH = 8, 4
    for bits, name in ((8, "rle8.avi"), (4, "rle4.avi")):
        top = 256 if bits == 8 else 16
        idx = [[[(x // 2 + y * 3) % top for x in range(RW)] for y in range(RH)]]
        for t in (1, 2):
            f = [r[:] for r in idx[-1]]
            f[t][t + 1] = (f[t][t + 1] + 5) % top
            f[0][0] = (f[0][0] + t) % top
            idx.append(f)
        data = [rle_encode(idx[0], bits)] + [rle_encode(idx[t], bits, idx[t - 1]) for t in (1, 2)]
        pal_n = top
        lut = b"".join(bytes([(i * 9) % 256, (i * 17) % 256, (i * 5) % 256, 0]) for i in range(pal_n))
        comp = 1 if bits == 8 else 2
        (OUT / name).write_bytes(avi(RW, RH, bih(RW, RH, bits, comp, colors=pal_n) + lut, data, handler="mrle", kind="dc",
                                     key=lambda i: i == 0))
        rgb_of = lambda i: ((i * 5) % 256, (i * 17) % 256, (i * 9) % 256)
        record(name, 3, 3, [bytes(c for r in f for v in r for c in rgb_of(v)) for f in idx])

    # Index variants: none (LIST rec), absolute with audio, dropped frame.
    frames = [[[g(x, y, t) for x in range(W)] for y in range(H)] for t in range(4)]
    raw = [stored_rows(f, W) for f in frames]
    out_g = [bytes(v for r in f for v in r) for f in frames]
    (OUT / "noidx.avi").write_bytes(avi(W, H, bih(W, H, 8, 0) + gray_lut, raw[:3], index=None, rec=True))
    record("noidx.avi", 3, 1, out_g[:3])
    (OUT / "absidx.avi").write_bytes(avi(W, H, bih(W, H, 8, 0) + gray_lut, raw[:3], index="absolute", audio=True))
    record("absidx.avi", 3, 1, out_g[:3])
    (OUT / "dropped.avi").write_bytes(avi(W, H, bih(W, H, 8, 0) + gray_lut, [raw[0], raw[1], b"", raw[3]]))
    record("dropped.avi", 4, 1, [out_g[0], out_g[1], out_g[1], out_g[3]])

    # OpenDML.
    frames = [[[(x * 60 + y * 30 + t * 17) % 256 for x in range(4)] for y in range(2)] for t in range(5)]
    (OUT / "opendml.avi").write_bytes(opendml(4, 2, [stored_rows(f, 4) for f in frames], 2))
    record("opendml.avi", 5, 1, [bytes(v for r in f for v in r) for f in frames])

    # Unsupported codec and truncated file.
    stub = b"\x00\x00\x00\x01\x67\x42\x00\x0a" + bytes(24)
    (OUT / "h264.avi").write_bytes(avi(16, 16, bih(16, 16, 24, "H264"), [stub], handler="H264", kind="dc"))
    whole = (OUT / "bgr24.avi").read_bytes()
    second = whole.find(b"00db", whole.find(b"00db") + 4)
    (OUT / "truncated.avi").write_bytes(whole[:second + 20])

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tframes\tsamples\tcrc32\tmean\n")
        for row in truth:
            f.write("\t".join(str(v) for v in row) + "\n")

    if "--bench" in sys.argv:
        bench(Path(sys.argv[sys.argv.index("--bench") + 1]))


def bench(out):
    """Large benchmark inputs: 1000 frames of 1024 x 1024 8-bit raw (an
    OpenDML file above 1 GB) and Motion JPEG (one encoded frame repeated)."""
    out.mkdir(parents=True, exist_ok=True)
    n, size = 1000, 1024
    rows = [bytes((x + y) % 256 for x in range(size)) for y in range(size)]
    frame = b"".join(rows[::-1])
    split = 500
    path = out / "bench_raw8.avi"
    with open(path, "wb") as f:
        f.write(opendml_stream(size, size, frame, n, split))
    big = [c for y in range(size) for x in range(size) for c in ((x + y) % 256, (2 * x) % 256, (3 * y) % 256)]
    jp = jpeg(size, size, big, False, True)
    (out / "bench_mjpeg.avi").write_bytes(avi(size, size, bih(size, size, 24, "MJPG", size * size * 3), [jp] * n, kind="dc"))


def opendml_stream(width, height, frame, n, split):
    return opendml(width, height, [frame] * n, split)


if __name__ == "__main__":
    main()

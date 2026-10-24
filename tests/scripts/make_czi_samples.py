#!/usr/bin/env python3
"""Writes small synthetic Zeiss CZI files.

Written from the segment, directory and subblock layouts described in
doc/zeiss-czi/ (OpenReadout notes; libCZI documentation pages on chunked
compression and valid-pixel masks). zstd data is written as raw blocks
(RFC 8878) and LZ4 data as literal-only blocks, both valid streams. Also
writes truth.tsv with the CRC-32 of each series and level in the layout of
tests/common/mod.rs `decode()` / `decode_level()`. Output in
tests/data/czi/ is committed.

Usage: python3 -I tests/scripts/make_czi_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "czi"
TRUTH = []

PIXEL = {"gray8": (0, "B", 1), "gray16": (1, "H", 1), "bgr24": (3, "B", 3), "gray32f": (2, "f", 1)}


def value(x, y, c=0, z=0, t=0, s=0, k=0, h=0):
    return (x * 7 + y * 13 + c * 101 + z * 37 + t * 59 + s * 211 + k * 17 + h * 5) % 251


# --- Codecs -------------------------------------------------------------------

def zstd_raw(data):
    """A zstd frame of raw blocks with a 4-byte content size."""
    out = bytearray(b"\x28\xb5\x2f\xfd") + bytes([0xA0]) + struct.pack("<I", len(data))
    blocks = [data[i:i + 65536] for i in range(0, len(data), 65536)] or [b""]
    for i, b in enumerate(blocks):
        last = 1 if i == len(blocks) - 1 else 0
        out += struct.pack("<I", (len(b) << 3) | last)[:3] + b
    return bytes(out)


def lz4_literals(data):
    """An LZ4 block holding only literals."""
    n = len(data)
    if n < 15:
        return bytes([n << 4]) + data
    rest, ext = n - 15, bytearray()
    while rest >= 255:
        ext.append(255)
        rest -= 255
    ext.append(rest)
    return bytes([0xF0]) + bytes(ext) + data


def varint(v):
    out = bytearray()
    while True:
        b = v & 0x7F
        v >>= 7
        out.append(b | (0x80 if v else 0))
        if not v:
            return bytes(out)


def hilo(data):
    return data[0::2] + data[1::2]


def chunked_lz4(data, chunk, split_hilo):
    raw = hilo(data) if split_hilo else data
    chunks = [raw[i:i + chunk] for i in range(0, len(raw), chunk)]
    comp = [lz4_literals(c) for c in chunks]

    def entry(i, payload):
        return varint(i) + varint(len(payload)) + payload

    head = entry(1, b"".join(varint(len(c)) for c in comp))
    sizes = [chunk] if len(chunks[-1]) == chunk else [chunk, len(chunks[-1])]
    head += entry(3, b"".join(varint(s) for s in sizes)) + entry(2, b"\x01")
    if split_hilo:
        head += entry(4, b"\x01")
    return head + b"\x00" + b"".join(comp)


# --- Container ----------------------------------------------------------------

def segment(name, payload):
    return name.encode().ljust(16, b"\0") + struct.pack("<QQ", len(payload), len(payload)) + payload


def dims(d):
    """Dimension entries: (name, start, size, stored)."""
    return b"".join(n.encode().ljust(4, b"\0") + struct.pack("<iIfI", st, sz, 0.0, ss) for n, st, sz, ss in d)


def dir_entry(pt, pos, comp, pyramid, d, part=0):
    return b"DV" + struct.pack("<IQII", pt, pos, part, comp) + bytes([pyramid]) + bytes(5) + struct.pack("<I", len(d)) + dims(d)


GUID = bytes(range(1, 17))


class Writer:
    """A CZI document; segments with `part` k > 0 go to following part k."""

    def __init__(self):
        self.parts = {0: bytearray(segment("ZISRAWFILE", bytes(512)))}
        self.data = self.parts[0]
        self.entries = []
        self.attachments = []

    def _part(self, part):
        if part not in self.parts:
            self.parts[part] = bytearray(segment("ZISRAWFILE", bytes(512)))
        return self.parts[part]

    def subblock(self, pt, comp, d, pixels, pyramid=0, attachment=b"", part=0):
        data = self._part(part)
        pos = len(data)
        entry = dir_entry(pt, pos, comp, pyramid, d, part)
        meta = b"<METADATA></METADATA>"
        head = struct.pack("<IIQ", len(meta), len(attachment), len(pixels)) + entry
        head = head.ljust(max(256, len(head)), b"\0")
        data += segment("ZISRAWSUBBLOCK", head + meta + pixels + attachment)
        self.entries.append(entry)

    def attach(self, name, content_type, payload, part=0, stored_in=None):
        """`stored_in`: the part holding the segment if not `part` (a wrong part number)."""
        data = self._part(part if stored_in is None else stored_in)
        pos = len(data)
        entry = b"A1" + bytes(10) + struct.pack("<QI", pos, part) + bytes(16) + content_type.encode().ljust(8, b"\0") + name.encode().ljust(80, b"\0")
        head = struct.pack("<I", len(payload)).ljust(16, b"\0") + entry
        data += segment("ZISRAWATTACH", head.ljust(256, b"\0") + payload)
        self.attachments.append(entry)

    def write(self, path, xml):
        meta_pos = len(self.data)
        x = xml.encode()
        self.data += segment("ZISRAWMETADATA", struct.pack("<II", len(x), 0).ljust(256, b"\0") + x)
        dir_pos = len(self.data)
        self.data += segment("ZISRAWDIRECTORY", struct.pack("<I", len(self.entries)).ljust(128, b"\0") + b"".join(self.entries))
        att_pos = 0
        if self.attachments:
            att_pos = len(self.data)
            self.data += segment("ZISRAWATTDIR", struct.pack("<I", len(self.attachments)).ljust(256, b"\0") + b"".join(self.attachments))
        header = struct.pack("<II", 1, 0) + bytes(8) + GUID + GUID + struct.pack("<IQQIQ", 0, dir_pos, meta_pos, 0, att_pos)
        self.data[32:32 + len(header)] = header
        path.write_bytes(bytes(self.data))
        # Following parts: own file GUID, the master's as primary GUID.
        for k, data in self.parts.items():
            if k:
                header = struct.pack("<II", 1, 0) + bytes(8) + GUID + bytes([k] * 16) + struct.pack("<IQQIQ", k, 0, 0, 0, 0)
                data[32:32 + len(header)] = header
                path.with_name(f"{path.stem} ({k}){path.suffix}").write_bytes(bytes(data))


def xml(scale_um, channels, scenes=(), bits=None, sizes=None, pixel_type="Gray16"):
    ch = "".join(f'<Channel Id="Channel:{i}" Name="{n}"><Color>{c}</Color></Channel>' for i, (n, c) in enumerate(channels))
    sc = "".join(f'<Scene Index="{i}" Name="{n}"/>' for i, n in scenes)
    b = f"<ComponentBitCount>{bits}</ComponentBitCount>" if bits else ""
    # Image sizes as ZEN writes them (Bio-Formats relies on them).
    b += "".join(f"<Size{k}>{v}</Size{k}>" for k, v in (sizes or {}).items()) + f"<PixelType>{pixel_type}</PixelType>"
    return (f'<ImageDocument><Metadata><Information><Image>{b}<Dimensions><Channels>{ch}</Channels>'
            f'<S><Scenes>{sc}</Scenes></S></Dimensions></Image></Information><Scaling><Items>'
            f'<Distance Id="X"><Value>{scale_um * 1e-6}</Value></Distance><Distance Id="Y"><Value>{scale_um * 1e-6}</Value></Distance>'
            f'</Items></Scaling></Metadata></ImageDocument>')


def tile(w, h, fmt, spp, f, bgr=False):
    """Pixel bytes of a tile; `f(x, y, k)` gives sample k (R, G, B order)."""
    out = bytearray()
    for y in range(h):
        for x in range(w):
            order = [2, 1, 0] if bgr else range(spp)
            for k in order:
                out += struct.pack("<" + fmt, f(x, y, k))
    return bytes(out)


def truth(file, series, level, planes, fmt):
    """`planes`: list of planes in t, z order, each a list of rows of pixels (channel tuples)."""
    data = bytearray()
    for plane in planes:
        for row in plane:
            for px in row:
                for v in px:
                    data += struct.pack("<" + fmt, v)
    TRUTH.append((file, series, level, f"{zlib.crc32(bytes(data)):08x}"))


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    # A: BGR24 mosaic, two overlapping raw tiles with negative origin; a 2x
    # pyramid subblock whose valid-pixel mask hides its right half.
    w = Writer()
    tiles = [(-4, 3, 0), (6, 5, 1)]  # x, y, m
    tw, th = 12, 8
    for x0, y0, m in tiles:
        px = tile(tw, th, "B", 3, lambda x, y, k, x0=x0, y0=y0, m=m: value(x + x0, y + y0, k=k, h=m), bgr=True)
        w.subblock(3, 0, [("X", x0, tw, tw), ("Y", y0, th, th), ("C", 0, 1, 1), ("M", m, 1, 1)], px)
    canvas_w, canvas_h = 6 + tw + 4, 5 + th - 3  # 22 x 10
    canvas = [[(0, 0, 0)] * canvas_w for _ in range(canvas_h)]
    for x0, y0, m in tiles:
        for y in range(th):
            for x in range(tw):
                canvas[y + y0 - 3][x + x0 + 4] = tuple(value(x + x0, y + y0, k=k, h=m) for k in range(3))
    truth("mosaic_bgr24.czi", 0, 0, [canvas], "B")
    # Pyramid: 11 x 5 stored for 22 x 10 logical; mask stride 2 bytes.
    sw, sh = 11, 5
    mask_rows = [bytes([0xFC, 0x00]) for _ in range(sh)]  # first 6 pixels valid
    payload = struct.pack("<4I", sw, sh, 0, 2) + b"".join(mask_rows)
    mask = bytes([0x67, 0xEA, 0xE3, 0xCB, 0xFC, 0x5B, 0x2B, 0x49, 0xA1, 0x6A, 0xEC, 0xE3, 0x78, 0x03, 0x14, 0x48]) + struct.pack("<i", len(payload)) + payload
    pyr = tile(sw, sh, "B", 3, lambda x, y, k: (x * 3 + y + k) % 200, bgr=True)
    w.subblock(3, 0, [("X", -4, canvas_w, sw), ("Y", 3, canvas_h, sh), ("C", 0, 1, 1)], pyr, pyramid=2, attachment=mask)
    level1 = [[tuple((x * 3 + y + k) % 200 for k in range(3)) if x < 6 else (0, 0, 0) for x in range(sw)] for y in range(sh)]
    truth("mosaic_bgr24.czi", 0, 1, [level1], "B")
    w.write(OUT / "mosaic_bgr24.czi", xml(0.5, [("Brightfield", "#FFFFFFFF")], sizes={"X": 22, "Y": 10, "C": 1, "M": 2}, pixel_type="Bgr24"))

    # B: gray16, two scenes, two channels, two z; scene 0 zstd1 with hi/lo
    # split, scene 1 plain zstd and an H index that varies (two series).
    w = Writer()
    W, H = 9, 4
    planes = {}
    for s, h_values in ((0, [0]), (1, [0, 1])):
        for hv in h_values:
            for z in range(2):
                for c in range(2):
                    f = lambda x, y, k, c=c, z=z, s=s, hv=hv: value(x, y, c=c, z=z, s=s, h=hv) * 250
                    raw = tile(W, H, "H", 1, f)
                    if s == 0:
                        comp, data = 6, bytes([3, 1, 1]) + zstd_raw(hilo(raw))
                    else:
                        comp, data = 5, zstd_raw(raw)
                    d = [("X", 100 * s, W, W), ("Y", 0, H, H), ("Z", z, 1, 1), ("C", c, 1, 1), ("S", s, 1, 1), ("H", hv, 1, 1)]
                    w.subblock(1, comp, d, data)
            plane_list = [[[tuple(value(x, y, c=c, z=z, s=s, h=hv) * 250 for c in range(2)) for x in range(W)] for y in range(H)] for z in range(2)]
            planes[(s, hv)] = plane_list
    for i, key in enumerate([(0, 0), (1, 0), (1, 1)]):
        truth("scenes_zstd.czi", i, 0, planes[key], "H")
    w.write(OUT / "scenes_zstd.czi", xml(0.25, [("DAPI", "#FF0000FF"), ("FITC", "#FF00FF00")], [(0, "A1"), (1, "B2")], bits=12,
                                     sizes={"X": 109, "Y": 4, "Z": 2, "C": 2, "S": 2, "H": 2}))

    # C: gray16 chunked LZ4 with hi/lo split, three time points and a
    # TimeStamps attachment.
    w = Writer()
    W, H = 10, 7
    plane_list = []
    for t in range(3):
        raw = tile(W, H, "H", 1, lambda x, y, k, t=t: value(x, y, t=t) * 3)
        w.subblock(1, 7, [("X", 0, W, W), ("Y", 0, H, H), ("T", t, 1, 1), ("C", 0, 1, 1)], chunked_lz4(raw, 48, True))
        plane_list.append([[(value(x, y, t=t) * 3,) for x in range(W)] for y in range(H)])
    truth("chunked_time.czi", 0, 0, plane_list, "H")
    w.attach("TimeStamps", "CZTIMS", struct.pack("<II", 32, 3) + struct.pack("<3d", 10.0, 12.5, 15.0))
    w.write(OUT / "chunked_time.czi", xml(1.0, [("GFP", "#FF00FF00")], sizes={"X": 10, "Y": 7, "T": 3, "C": 1}))

    # D: multi-file document, gray8, time point t in part t; TimeStamps in
    # part 1; a thumbnail entry naming part 7 (absent) stored in the master.
    w = Writer()
    W, H = 6, 5
    plane_list = []
    for t in range(3):
        w.subblock(0, 0, [("X", 0, W, W), ("Y", 0, H, H), ("T", t, 1, 1), ("C", 0, 1, 1)], tile(W, H, "B", 1, lambda x, y, k, t=t: value(x, y, t=t) % 256), part=t)
        plane_list.append([[(value(x, y, t=t) % 256,) for x in range(W)] for y in range(H)])
    truth("multipart.czi", 0, 0, plane_list, "B")
    w.attach("TimeStamps", "CZTIMS", struct.pack("<II", 32, 3) + struct.pack("<3d", 1.0, 2.0, 4.0), part=1)
    w.attach("Profile", "CZPRF", b"profile", part=7, stored_in=0)
    w.write(OUT / "multipart.czi", xml(1.0, [("Gray", "#FFFFFFFF")], sizes={"X": W, "Y": H, "T": 3, "C": 1}, pixel_type="Gray8"))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tlevel\tcrc32\n")
        for row in TRUTH:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

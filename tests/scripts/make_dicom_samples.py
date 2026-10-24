#!/usr/bin/env python3
"""Writes small synthetic DICOM files.

Encoding from DICOM PS3.5 and PS3.10 (doc/dicom/): Part 10 preamble and
meta information, explicit VR little endian (and implicit VR for the plain
image), sequences of undefined length, RLE (Annex G). Sets:

- wsi/: a brightfield whole-slide image (Series Instance UID shared):
  level 0 40 x 30 RGB, TILED_FULL, native, 16 x 16 tiles (partial edge
  tiles); level 1 20 x 15 TILED_SPARSE, RLE, the tile at (16, 0) missing
  (reads as white); a LABEL image. Pixel spacing 0.0005 mm.
- fluo/: a fluorescence whole-slide image, 16-bit MONOCHROME2, 2 optical
  paths (channels) x 2 focal planes, TILED_FULL, 12 x 10 tiles of 8 x 8.
- plain.dcm: a non-WSI image in implicit VR, 3 frames of 7 x 5 signed
  16-bit (frames read as z planes).

Also writes truth.tsv with the CRC-32 of the pixels in the layout of
tests/common/mod.rs `decode_level()`. Output in tests/data/dicom/ is
committed.

Usage: python3 -I tests/scripts/make_dicom_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "dicom"
LONG = {"OB", "OD", "OF", "OL", "OV", "OW", "SQ", "SV", "UC", "UN", "UR", "UT", "UV"}
SERIES = "1.2.826.0.1.3680043.10.1234.1"


def el(tag, vr, value):
    """Explicit VR little endian element; `value` bytes, str or list of datasets (SQ)."""
    g, e = tag >> 16, tag & 0xFFFF
    if vr == "SQ":
        body = b"".join(struct.pack("<HHI", 0xFFFE, 0xE000, 0xFFFFFFFF) + b"".join(item) + struct.pack("<HHI", 0xFFFE, 0xE00D, 0) for item in value)
        return struct.pack("<HH2sHI", g, e, b"SQ", 0, 0xFFFFFFFF) + body + struct.pack("<HHI", 0xFFFE, 0xE0DD, 0)
    if isinstance(value, str):
        value = value.encode()
        if len(value) % 2:
            value += b"\0" if vr == "UI" else b" "
    if vr in LONG:
        return struct.pack("<HH2sHI", g, e, vr.encode(), 0, len(value)) + value
    return struct.pack("<HH2sH", g, e, vr.encode(), len(value)) + value


def imp(tag, value):
    """Implicit VR little endian element."""
    if isinstance(value, str):
        value = value.encode()
        if len(value) % 2:
            value += b" "
    return struct.pack("<HHI", tag >> 16, tag & 0xFFFF, len(value)) + value


def us(*v):
    return struct.pack(f"<{len(v)}H", *v)


def ul(*v):
    return struct.pack(f"<{len(v)}I", *v)


def part10(ts, sop_uid, elements, sop_class="1.2.840.10008.5.1.4.1.1.77.1.6"):
    meta = [el(0x00020001, "OB", b"\0\1"), el(0x00020002, "UI", sop_class), el(0x00020003, "UI", sop_uid), el(0x00020010, "UI", ts)]
    body = b"".join(meta)
    return bytes(128) + b"DICM" + el(0x00020000, "UL", ul(len(body))) + body + b"".join(elements)


def encapsulated(frames):
    out = struct.pack("<HH2sHI", 0x7FE0, 0x0010, b"OB", 0, 0xFFFFFFFF) + struct.pack("<HHI", 0xFFFE, 0xE000, 0)
    for f in frames:
        if len(f) % 2:
            f += b"\0"
        out += struct.pack("<HHI", 0xFFFE, 0xE000, len(f)) + f
    return out + struct.pack("<HHI", 0xFFFE, 0xE0DD, 0)


def rle_frame(frame, spp, w, h):
    """RLE (Annex G): one segment per sample byte, literal runs of up to 128."""
    segs = []
    for s in range(spp):
        data = bytes(frame[i * spp + s] for i in range(w * h))
        enc = bytearray()
        for i in range(0, len(data), 128):
            chunk = data[i:i + 128]
            enc += bytes([len(chunk) - 1]) + chunk
        segs.append(bytes(enc))
    header = bytearray(64)
    struct.pack_into("<I", header, 0, spp)
    at = 64
    for i, s in enumerate(segs):
        struct.pack_into("<I", header, 4 + 4 * i, at)
        at += len(s)
    return bytes(header) + b"".join(segs)


def wsi_common(kind, w, h, tw, th, frames, spp=3, photometric="RGB", bits=8, organization="TILED_FULL", extra=()):
    image_type = f"DERIVED\\PRIMARY\\{kind}\\NONE"
    shared = [[el(0x00289110, "SQ", [[el(0x00180088, "DS", "0.002"), el(0x00280030, "DS", "0.0005\\0.0005")]])]]
    return [
        el(0x00080008, "CS", image_type), el(0x00080060, "CS", "SM"), el(0x0008002A, "DT", "20240102030405"),
        el(0x0020000E, "UI", SERIES), el(0x00209311, "CS", organization),
        el(0x00280002, "US", us(spp)), el(0x00280004, "CS", photometric), el(0x00280006, "US", us(0)) if spp == 3 else b"",
        el(0x00280008, "IS", str(frames)), el(0x00280010, "US", us(th)), el(0x00280011, "US", us(tw)),
        el(0x00280100, "US", us(bits)), el(0x00280101, "US", us(bits)), el(0x00280102, "US", us(bits - 1)), el(0x00280103, "US", us(0)),
        el(0x00480006, "UL", ul(w)), el(0x00480007, "UL", ul(h)),
        *extra,
        el(0x52009229, "SQ", shared),
    ]


def rgb(x, y, k, seed):
    return (x * 7 + y * 11 + k * 60 + seed) % 256


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "wsi").mkdir(parents=True)
    (OUT / "fluo").mkdir()
    truth = []

    # Brightfield level 0: 40 x 30, tiles 16 x 16 (3 x 2), native.
    W0, H0, T = 40, 30, 16
    full0 = [[tuple(rgb(x, y, k, 0) for k in range(3)) for x in range(W0)] for y in range(H0)]
    frames = []
    for ty in range(2):
        for tx in range(3):
            f = bytearray()
            for y in range(T):
                for x in range(T):
                    X, Y = tx * T + x, ty * T + y
                    f += bytes(full0[Y][X]) if X < W0 and Y < H0 else bytes(3)
            frames.append(bytes(f))
    els = wsi_common("VOLUME", W0, H0, T, T, len(frames)) + [el(0x7FE00010, "OB", b"".join(frames))]
    (OUT / "wsi" / "level0.dcm").write_bytes(part10("1.2.840.10008.1.2.1", SERIES + ".10", els))
    truth.append(("wsi/level0.dcm", "level0", b"".join(bytes(p) for row in full0 for p in row)))

    # Level 1: 20 x 15, TILED_SPARSE, RLE; tiles 16 x 16 (2 x 1), the second missing.
    W1, H1 = 20, 15
    full1 = [[tuple(rgb(x, y, k, 99) for k in range(3)) for x in range(W1)] for y in range(H1)]
    f = bytearray()
    for y in range(T):
        for x in range(T):
            f += bytes(full1[y][x]) if y < H1 else bytes(3)
    per_frame = [[el(0x0048021A, "SQ", [[el(0x0048021E, "SL", struct.pack("<i", 1)), el(0x0048021F, "SL", struct.pack("<i", 1))]])]]
    els = wsi_common("VOLUME", W1, H1, T, T, 1, organization="TILED_SPARSE") + [el(0x52009230, "SQ", per_frame), encapsulated([rle_frame(f, 3, T, T)])]
    (OUT / "wsi" / "level1.dcm").write_bytes(part10("1.2.840.10008.1.2.5", SERIES + ".11", els))
    lv1 = b"".join(bytes(full1[y][x]) if x < T else bytes([255, 255, 255]) for y in range(H1) for x in range(W1))
    truth.append(("wsi/level0.dcm", "level1", lv1))

    # Label: 6 x 4 native RGB.
    label = bytes(rgb(x, y, k, 50) for y in range(4) for x in range(6) for k in range(3))
    els = wsi_common("LABEL", 6, 4, 6, 4, 1) + [el(0x7FE00010, "OB", label)]
    (OUT / "wsi" / "label.dcm").write_bytes(part10("1.2.840.10008.1.2.1", SERIES + ".12", els))
    truth.append(("wsi/level0.dcm", "label", label))

    # Fluorescence: 12 x 10, tiles 8 x 8 (2 x 2), 2 focal planes, 2 optical paths, 16-bit.
    W, H, TS, Z = 12, 10, 8, 2
    paths = [("1", "DAPI", 405, 461), ("2", "FITC", 488, 520)]
    val = lambda x, y, c, z: (x * 31 + y * 17 + c * 5000 + z * 900) % 65536
    frames = []
    for c in range(2):
        for z in range(Z):
            for ty in range(2):
                for tx in range(2):
                    fr = bytearray()
                    for y in range(TS):
                        for x in range(TS):
                            X, Y = tx * TS + x, ty * TS + y
                            fr += struct.pack("<H", val(X, Y, c, z) if X < W and Y < H else 0)
                    frames.append(bytes(fr))
    optical = [[el(0x00220055, "FL", struct.pack("<f", ex)), el(0x00220057, "FL", struct.pack("<f", em)),
                el(0x00480106, "SH", pid), el(0x00480107, "ST", name)] for pid, name, ex, em in paths]
    extra = [el(0x00480105, "SQ", optical), el(0x00480302, "UL", ul(2)), el(0x00480303, "UL", ul(Z))]
    els = wsi_common("VOLUME", W, H, TS, TS, len(frames), spp=1, photometric="MONOCHROME2", bits=16, extra=extra) + [el(0x7FE00010, "OW", b"".join(frames))]
    (OUT / "fluo" / "level0.dcm").write_bytes(part10("1.2.840.10008.1.2.1", SERIES + ".2", els))
    px = b"".join(struct.pack("=H", val(x, y, c, z)) for z in range(Z) for y in range(H) for x in range(W) for c in range(2))
    truth.append(("fluo/level0.dcm", "level0", px))

    # Plain image: implicit VR, 3 frames of 7 x 5, signed 16-bit.
    pv = lambda x, y, f: (x * 300 - y * 700 + f * 1000) % 65536 - 32768
    data = b"".join(struct.pack("<h", pv(x, y, f)) for f in range(3) for y in range(5) for x in range(7))
    els = [imp(0x00080008, "ORIGINAL\\PRIMARY\\AXIAL"), imp(0x00080060, "CT"), imp(0x00280002, us(1)), imp(0x00280004, "MONOCHROME2"),
           imp(0x00280008, "3"), imp(0x00280010, us(5)), imp(0x00280011, us(7)), imp(0x00280030, "0.5\\0.25"),
           imp(0x00280100, us(16)), imp(0x00280101, us(16)), imp(0x00280102, us(15)), imp(0x00280103, us(1)), imp(0x7FE00010, data)]
    (OUT / "plain.dcm").write_bytes(part10("1.2.840.10008.1.2", "1.2.826.0.1.3680043.10.1234.3", els, sop_class="1.2.840.10008.5.1.4.1.1.2"))
    truth.append(("plain.dcm", "level0", b"".join(struct.pack("=h", pv(x, y, f)) for f in range(3) for y in range(5) for x in range(7))))

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\timage\tcrc32\n")
        for name, image, px in truth:
            f.write(f"{name}\t{image}\t{zlib.crc32(px):08x}\n")


if __name__ == "__main__":
    main()

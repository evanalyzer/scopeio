#!/usr/bin/env python3
"""Writes a small synthetic Olympus FluoView acquisition as OIF and OIB.

Layout from the OpenReadout OIF notes and oiffile (BSD-3-Clause)
(doc/olympus-oif-oib/): a main settings file (UTF-16LE INI with byte-order
mark, decimal commas), a folder `<name>.oif.files` with one uncompressed
16-bit TIFF per plane named `s_C001Z001T001.tif` and a `.pty` property file
for the first plane. The OIB packs the same files into a compound file
([MS-CFB] version 3: 512-byte sectors, small streams in the mini stream)
with `OibInfo.txt` naming the streams. 2 channels x 3 z x 2 t of 20 x 13. Also writes truth.tsv
with the CRC-32 of the pixels in the layout of tests/common/mod.rs
`decode_level()`. Output in tests/data/oif/ is committed.

Usage: python3 -I tests/scripts/make_oif_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "oif"
W, H, C, Z, T = 20, 13, 2, 3, 2


def value(x, y, c, z, t):
    return (x * 97 + y * 31 + c * 1000 + z * 211 + t * 3001) % 4096


def tiff(c, z, t):
    """Uncompressed little-endian 16-bit gray TIFF, one strip."""
    data = b"".join(struct.pack("<H", value(x, y, c, z, t)) for y in range(H) for x in range(W))
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(data))]
    ifd = struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    return b"II*\0" + struct.pack("<I", 8 + len(data)) + data + ifd


def utf16(text):
    return b"\xff\xfe" + text.replace("\n", "\r\n").encode("utf-16-le")


MAIN = """[Acquisition Parameters Common]
Acquisition Device="FV1000"
ImageCaputreDate='2021-03-04 05:06:07'
ScanMode="XYZT"
{AXES}[Axis Parameter Common]
AxisCount=4
AxisOrder="XYCZT"
[Channel 1 Parameters]
CH Name="CH1"
DyeName="DAPI"
EmissionWavelength=461
ExcitationWavelength=405
LightType="Fluorescence"
[Channel 2 Parameters]
CH Name="CH3"
DyeName="None"
EmissionWavelength=0
ExcitationWavelength=543
LightType="Fluorescence"
[File Info]
DataName="synthetic.oib"
[ProfileSaveInfo]
Name="COFAImages"
PtyFileNameE="synthetic.oif.files\\s_C002Z003T002.pty"
PtyFileNameS="synthetic.oif.files\\s_C001Z001T001.pty"
PtyFileNameT1="synthetic.oif.files\\s_%s.pty"
PtyFileNameT2="synthetic.oif.files\\s_C%03dZ%03dT%03d.pty"
Version="2.0.0.0"
[Reference Image Parameter]
HeightConvertValue=0,25.0
HeightUnit="um"
ValidBitCounts=12
WidthConvertValue=0,25.0
WidthUnit="um"
"""

AXES = [("X", "X", 20, "um", 0.0, 4.75, None), ("Y", "Y", 13, "um", 0.0, 3.0, None), ("C", "Ch", 2, "Ch", 1.0, 2.0, None),
        ("Z", "Z", 3, "nm", 0.0, 1500.0, 750), ("T", "T", 2, "ms", 0.0, 1000.0, 1000), ("A", "Angle", 0, "deg", 0.0, 0.0, None),
        ("L", "Lambda", 0, "nm", 400.0, 401.0, 1), ("P", "Point", 0, "mm", 0.0, 0.0, None), ("Q", "Unknown", 0, "Unknown", 0.0, 0.0, None)]


def axes_text():
    out = []
    for i, (code, name, size, unit, start, end, interval) in enumerate(AXES):
        out += [f"[Axis {i} Parameters Common]", "AbsolutePosition=0.0", f'AxisCode="{code}"', f'AxisName="{name}"',
                "CalibrateValueA=100.0", "CalibrateValueB=0.0", f"EndPosition={end}", f"GUI MaxSize={size}"]
        if interval is not None:
            out.append(f"Interval={interval}")
        out += [f"MaxSize={size}", f'PixUnit="{unit}"', f"StartPosition={start}", f'UnitName="{unit}"']
    return "\n".join(out) + "\n"


PTY = """[File Info]
DataName="{TIF}"
Path=".\\"
[Acquisition Parameters Common]
Magnification=60.0
ObjectiveLens NAValue=1,35.0
ObjectiveLens Name="UPLSAPO   60X O  NA:1.35"
[Image Parameters]
HeightConvertValue=0,125.0
HeightUnit="um"
ImageGroup="Normal"
ValidBitCounts=12
WidthConvertValue=0,125.0
WidthUnit="um"
"""


def main_text():
    return MAIN.replace("{AXES}", axes_text())


def members():
    """(file name, bytes) of the folder."""
    out = []
    for t in range(T):
        for z in range(Z):
            for c in range(C):
                out.append((f"s_C{c + 1:03d}Z{z + 1:03d}T{t + 1:03d}.tif", tiff(c, z, t)))
    # A property file for every plane, as FluoView writes them.
    for name, _ in list(out):
        out.append((name[:-4] + ".pty", utf16(PTY.replace("{TIF}", name))))
    return out


# --- Compound file writer ([MS-CFB], version 3) -------------------------------

SECTOR, MINI, CUTOFF = 512, 64, 4096
FREE, END, FATSECT, NOSTREAM = 0xFFFFFFFF, 0xFFFFFFFE, 0xFFFFFFFD, 0xFFFFFFFF


def cfb(root_streams, storage_name, storage_streams):
    """root_streams / storage_streams: [(name, bytes)]."""
    entries = [("Root Entry", 5, None)] + [(n, 2, d) for n, d in root_streams] + [(storage_name, 1, None)] + \
              [(n, 2, d) for n, d in storage_streams]
    storage_id = 1 + len(root_streams)
    sectors = []  # regular sector payloads
    fat = []

    def put_chain(data):
        start = len(sectors)
        n = max(1, -(-len(data) // SECTOR))
        for i in range(n):
            sectors.append(data[i * SECTOR:(i + 1) * SECTOR].ljust(SECTOR, b"\0"))
            fat.append(start + i + 1 if i + 1 < n else END)
        return start

    # Streams: large ones in sectors, small ones in the mini stream.
    mini = bytearray()
    minifat = []
    starts = {}
    for i, (_, typ, data) in enumerate(entries):
        if typ != 2:
            continue
        if len(data) >= CUTOFF:
            starts[i] = put_chain(data)
        else:
            start = len(mini) // MINI
            n = max(1, -(-len(data) // MINI))
            mini += data.ljust(n * MINI, b"\0")
            minifat += [start + k + 1 if k + 1 < n else END for k in range(n)]
            starts[i] = start
    mini_start = put_chain(bytes(mini)) if mini else END
    minifat_bytes = b"".join(struct.pack("<I", v) for v in minifat)
    minifat_start = put_chain(minifat_bytes.ljust(-(-len(minifat_bytes) // SECTOR) * SECTOR, b"\xff")) if minifat else END
    # Directory: siblings as a right-leaning chain (all black).
    def dirent(i):
        name, typ, data = entries[i]
        n = (name + "\0").encode("utf-16-le")
        left = right = child = NOSTREAM
        if i == 0:
            child = 1 if len(root_streams) else storage_id
        elif typ == 1:
            child = storage_id + 1 if storage_streams else NOSTREAM
        if 1 <= i < storage_id:
            right = i + 1  # root siblings, ending at the storage
        elif i > storage_id and i + 1 < len(entries):
            right = i + 1
        if i == 0:
            start, size = mini_start, len(mini)
        elif typ == 2:
            start, size = starts[i], len(data)
        else:
            start, size = 0, 0
        return (n.ljust(64, b"\0") + struct.pack("<HBB", len(n), typ, 1) + struct.pack("<III", left, right, child)
                + bytes(16) + bytes(4) + bytes(16) + struct.pack("<IQ", start, size))
    directory = b"".join(dirent(i) for i in range(len(entries)))
    directory = directory.ljust(-(-len(directory) // SECTOR) * SECTOR, b"\0")
    dir_start = put_chain(directory)
    # FAT sectors at the end, covering themselves.
    n_fat = 1
    while len(sectors) + n_fat > n_fat * (SECTOR // 4):
        n_fat += 1
    fat_ids = list(range(len(sectors), len(sectors) + n_fat))
    fat += [FATSECT] * n_fat
    fat += [FREE] * (n_fat * (SECTOR // 4) - len(fat))
    for k in range(n_fat):
        sectors.append(b"".join(struct.pack("<I", v) for v in fat[k * 128:(k + 1) * 128]))
    assert n_fat <= 109
    n_minifat = -(-len(minifat) * 4 // SECTOR)
    header = (b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + bytes(16) + struct.pack("<HHHHH", 0x3E, 3, 0xFFFE, 9, 6) + bytes(6)
              + struct.pack("<IIIIIIIII", 0, n_fat, dir_start, 0, CUTOFF, minifat_start, n_minifat, END, 0))
    difat = [fat_ids[k] if k < n_fat else FREE for k in range(109)]
    header += b"".join(struct.pack("<I", v) for v in difat)
    assert len(header) == 512, len(header)
    return header + b"".join(sectors)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    files = members()
    # OIF: main file and folder.
    folder = OUT / "synthetic.oif.files"
    folder.mkdir()
    (OUT / "synthetic.oif").write_bytes(utf16(main_text().replace("synthetic.oib", "synthetic.oif")))
    for name, data in files:
        (folder / name).write_bytes(data)
    # OIB: the same files as streams.
    info = ["[OibSaveInfo]", "Version=2.0.0.0", "MainFileName=Stream00100", "Compression=None",
            "Stream00100=synthetic.oif", "Storage00001=synthetic.oif.files"]
    storage = []
    for i, (name, data) in enumerate(files):
        sid = f"Stream{i + 1:05d}"
        info.append(f"{sid}=Storage00001/{name}")
        storage.append((sid, data))
    root = [("OibInfo.txt", utf16("\n".join(info) + "\n")), ("Stream00100", utf16(main_text()))]
    (OUT / "synthetic.oib").write_bytes(cfb(root, "Storage00001", storage))

    # Truth: planes t, z; channels interleaved per pixel.
    px = bytearray()
    for t in range(T):
        for z in range(Z):
            for y in range(H):
                for x in range(W):
                    for c in range(C):
                        px += struct.pack("=H", value(x, y, c, z, t))
    crc = f"{zlib.crc32(bytes(px)):08x}"
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tcrc32\n")
        f.write(f"synthetic.oib\t{crc}\nsynthetic.oif\t{crc}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Seeds fuzz/corpus/<target>/ with the small committed samples of
tests/data. Folder formats (OME-Zarr) are packed in the entry format of
`scopeio_fuzz::tree`: relative path, 0x00, u32 LE length, bytes.

Run from the repository root: python3 -I fuzz/seed.py
"""

import hashlib
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "tests" / "data"
CORPUS = ROOT / "fuzz" / "corpus"
MAX_SEED = 1 << 20

# Target -> file name suffixes of its seeds (lower case).
FILES = {
    "tiff": (".tif", ".tiff", ".btf", ".svs", ".qptiff", ".scn", ".bif", ".lsm", ".stk"),
    "ome_tiff": (".ome.tif", ".ome.tiff", ".ome.btf"),
    "ndpi": (".ndpi",),
    "nd2": (".nd2",),
    "czi": (".czi",),
    "vsi": (".vsi",),
    "lif": (".lif",),
    "ims": (".ims",),
    "dicom": (".dcm",),
    "zvi": (".zvi", ".oib", ".xrm", ".txrm", ".txm"),
    "avi": (".avi",),
    "amira": (".am",),
    "dcimg": (".dcimg",),
}


def add(target, data):
    out = CORPUS / target
    out.mkdir(parents=True, exist_ok=True)
    (out / hashlib.sha1(data).hexdigest()).write_bytes(data)


def pack(folder):
    """One OME-Zarr folder as a `tree` input, metadata first."""
    files = sorted((p for p in folder.rglob("*") if p.is_file()), key=lambda p: (p.name[0] not in ".z", str(p)))
    out = bytearray()
    for p in files:
        body = p.read_bytes()
        out += str(p.relative_to(folder)).encode() + b"\0" + struct.pack("<I", len(body)) + body
    return bytes(out)


def main():
    counts = dict.fromkeys(list(FILES) + ["zarr"], 0)
    for p in sorted(DATA.rglob("*")):
        if not p.is_file() or p.stat().st_size > MAX_SEED:
            continue
        name = p.name.lower()
        for target, suffixes in FILES.items():
            if name.endswith(suffixes):
                add(target, p.read_bytes())
                counts[target] += 1
    for folder in sorted(DATA.rglob("*.zarr")):
        data = pack(folder)
        if folder.is_dir() and len(data) <= MAX_SEED:
            add("zarr", data)
            counts["zarr"] += 1
    for target, n in counts.items():
        print(f"{target}: {n} seeds")
    return 0


if __name__ == "__main__":
    sys.exit(main())

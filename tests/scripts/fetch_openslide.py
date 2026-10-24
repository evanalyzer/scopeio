#!/usr/bin/env python3
"""Downloads selected OpenSlide test slides into tests/reference/openslide/.

Files come from https://openslide.cs.cmu.edu/download/openslide-testdata/
and are verified against the SHA-256 sums of its index.json. Their
licences are listed there (CC0-1.0 or "distributable"); the files are not
committed. MRXS archives are unpacked next to the zip.

Usage: python3 -I tests/scripts/fetch_openslide.py [--large] [names...]
"""

import argparse
import hashlib
import json
import sys
import urllib.request
import zipfile
from pathlib import Path

BASE = "https://openslide.cs.cmu.edu/download/openslide-testdata/"
OUT = Path(__file__).resolve().parent.parent / "reference" / "openslide"
DEFAULT = [
    "Aperio/CMU-1-Small-Region.svs", "Aperio/CMU-1.svs", "Aperio/CMU-1-JP2K-33005.svs", "Aperio/JP2K-33003-1.svs",
    "Hamamatsu/CMU-1.ndpi", "Hamamatsu/Hamamatsu-2.ndpi",
    "Mirax/CMU-1-Saved-1_16.zip", "Mirax/Mirax2-Fluorescence-2.zip", "Mirax/CMU-1-Saved-1_2.zip",
    "Leica/Leica-Fluorescence-1.scn", "Leica/Leica-1.scn",
    "Ventana/Ventana-1.bif",
]
# Large files with layouts the default set lacks (NDPI offsets above 4 GiB).
LARGE = ["Hamamatsu/Hamamatsu-1.ndpi", "Hamamatsu/OS-2.ndpi"]


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("names", nargs="*")
    ap.add_argument("--large", action="store_true", help="also fetch the multi-GB slides")
    args = ap.parse_args()
    with urllib.request.urlopen(BASE + "index.json", timeout=60) as r:
        index = json.load(r)
    names = args.names or DEFAULT + (LARGE if args.large else [])
    for name in names:
        info = index[name]
        dest = OUT / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        if dest.exists() and dest.stat().st_size == info["size"] and sha256(dest) == info["sha256"]:
            print(f"ok   {name}", file=sys.stderr)
        else:
            print(f"get  {name} ({info['size'] >> 20} MiB, {info['license']})", file=sys.stderr)
            part = dest.with_name(dest.name + ".part")
            urllib.request.urlretrieve(BASE + name, part)
            if sha256(part) != info["sha256"]:
                sys.exit(f"checksum mismatch: {name}")
            part.rename(dest)
        if dest.suffix == ".zip":
            target = dest.with_suffix("")
            if not target.exists():
                with zipfile.ZipFile(dest) as z:
                    # Refuse members that would leave the target folder.
                    for m in z.namelist():
                        if m.startswith("/") or ".." in Path(m).parts:
                            sys.exit(f"unsafe member {m} in {name}")
                    z.extractall(target)


if __name__ == "__main__":
    main()

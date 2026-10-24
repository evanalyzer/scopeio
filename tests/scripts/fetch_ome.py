#!/usr/bin/env python3
"""Downloads files from downloads.openmicroscopy.org/images/ into
tests/reference/ome/<same path>.

The size is checked against the server's Content-Length. Check each
folder's licence (readme.txt / COPYING) before use; the files are not
committed.

Usage: python3 -I tests/scripts/fetch_ome.py <path below images/>...
"""

import sys
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://downloads.openmicroscopy.org/images/"
OUT = Path(__file__).resolve().parent.parent / "reference" / "ome"


def main():
    for rel in sys.argv[1:]:
        dest = OUT / rel
        url = BASE + urllib.parse.quote(rel)
        with urllib.request.urlopen(urllib.request.Request(url, method="HEAD"), timeout=60) as r:
            size = int(r.headers["Content-Length"])
        if dest.exists() and dest.stat().st_size == size:
            print(f"ok   {rel}", file=sys.stderr)
            continue
        print(f"get  {rel} ({size >> 20} MiB)", file=sys.stderr)
        dest.parent.mkdir(parents=True, exist_ok=True)
        part = dest.with_name(dest.name + ".part")
        urllib.request.urlretrieve(url, part)
        if part.stat().st_size != size:
            sys.exit(f"size mismatch: {rel}")
        part.rename(dest)


if __name__ == "__main__":
    main()

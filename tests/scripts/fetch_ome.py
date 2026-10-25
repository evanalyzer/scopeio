#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Downloads files from downloads.openmicroscopy.org/images/ into
tests/reference/ome/<same path>.

The size is checked against the server's Content-Length. Check each
folder's licence (readme.txt / COPYING) before use; the files are not
committed.

Usage: python3 -I tests/scripts/fetch_ome.py <path below images/>...
       a path ending in / mirrors that folder recursively
"""

import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://downloads.openmicroscopy.org/images/"
OUT = Path(__file__).resolve().parent.parent / "reference" / "ome"


def listing(rel):
    """Files below folder `rel` (ending in /), recursively."""
    with urllib.request.urlopen(BASE + urllib.parse.quote(rel), timeout=60) as r:
        page = r.read().decode("utf-8", "replace")
    out = []
    for href in re.findall(r'href="([^"?][^"]*)"', page):
        if href.startswith("../") or href.startswith("/") or "://" in href:
            continue
        name = urllib.parse.unquote(href)
        out += listing(rel + name) if name.endswith("/") else [rel + name]
    return out


def fetch(rel):
    """Downloads one file unless present with the server's size."""
    dest = OUT / rel
    url = BASE + urllib.parse.quote(rel)
    with urllib.request.urlopen(urllib.request.Request(url, method="HEAD"), timeout=60) as r:
        size = int(r.headers["Content-Length"])
    if dest.exists() and dest.stat().st_size == size:
        print(f"ok   {rel}", file=sys.stderr)
        return
    print(f"get  {rel} ({size >> 20} MiB)", file=sys.stderr)
    dest.parent.mkdir(parents=True, exist_ok=True)
    part = dest.with_name(dest.name + ".part")
    urllib.request.urlretrieve(url, part)
    if part.stat().st_size != size:
        raise OSError(f"size mismatch: {rel}")
    part.rename(dest)


def main():
    rels = []
    for rel in sys.argv[1:]:
        rels += listing(rel) if rel.endswith("/") else [rel]
    failed = []
    for rel in rels:
        # Retry each file a few times; report failures at the end.
        for attempt in range(3):
            try:
                fetch(rel)
                break
            except OSError as e:
                if attempt == 2:
                    print(f"FAIL {rel}: {e}", file=sys.stderr)
                    failed.append(rel)
    if failed:
        sys.exit(f"{len(failed)} file(s) failed")


if __name__ == "__main__":
    main()

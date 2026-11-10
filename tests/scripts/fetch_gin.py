#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Downloads files of the CatalystNeuro optical-physiology test data
(https://gin.g-node.org/CatalystNeuro/ophys_testing_data) into
tests/reference/gin-ophys/<same path>.

The repository states no licence: contributors confirm they may publish
the files for building software on their formats. The files are
therefore not committed, only downloaded.

Usage: python3 -I tests/scripts/fetch_gin.py <path below the repository root>...
       a path ending in / mirrors that folder recursively
"""

import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

REPO = "https://gin.g-node.org/CatalystNeuro/ophys_testing_data"
OUT = Path(__file__).resolve().parent.parent / "reference" / "gin-ophys"
AGENT = {"User-Agent": "scopeio-tests"}


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=AGENT), timeout=120) as r:
        return r.read()


def listing(rel):
    """Files below folder `rel` (ending in /), recursively."""
    folder = rel.rstrip("/")
    page = get(f"{REPO}/src/main/{urllib.parse.quote(folder)}").decode("utf-8", "replace")
    prefix = "/CatalystNeuro/ophys_testing_data/"
    files, dirs = set(), set()
    for kind, href in re.findall(r'href="/CatalystNeuro/ophys_testing_data/(src|raw|media)/main/([^"?#]+)"', page):
        name = urllib.parse.unquote(href)
        if not name.startswith(folder + "/") or "/" in name[len(folder) + 1:]:
            continue
        (files if kind in ("raw", "media") else dirs).add(name)
    # Folder pages link files by src/ too; a name is a folder if its page
    # lists entries below it.
    out = []
    for name in sorted(dirs - files):
        sub = get(f"{REPO}/src/main/{urllib.parse.quote(name)}").decode("utf-8", "replace")
        if re.search(rf'href="{re.escape(prefix)}src/main/{re.escape(urllib.parse.quote(name))}/', sub):
            out += listing(name + "/")
        else:
            out.append(name)
    return out + sorted(files - dirs)


def fetch(rel):
    """Downloads one file unless present."""
    dest = OUT / rel
    if dest.exists() and dest.stat().st_size > 0:
        print(f"ok   {rel}", file=sys.stderr)
        return
    print(f"get  {rel}", file=sys.stderr)
    dest.parent.mkdir(parents=True, exist_ok=True)
    data = get(f"{REPO}/raw/main/{urllib.parse.quote(rel)}")
    if data.startswith(b"<!DOCTYPE html>"):
        raise OSError(f"not a file: {rel}")
    part = dest.with_name(dest.name + ".part")
    part.write_bytes(data)
    part.rename(dest)


def main():
    rels = []
    for rel in sys.argv[1:]:
        rels += listing(rel) if rel.endswith("/") else [rel]
    for rel in rels:
        fetch(rel)


if __name__ == "__main__":
    main()

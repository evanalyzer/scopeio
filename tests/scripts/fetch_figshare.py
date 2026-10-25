#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Downloads files of Figshare articles into tests/reference/figshare/<id>/.

Only articles under a CC BY or CC0 licence are fetched (the licence is
printed); the files are not committed. Sizes are checked against the
article listing.

Usage: python3 -I tests/scripts/fetch_figshare.py <article id>[:<file name>]...
       (without a file name: the first file with the extension given by
       --ext, default .zvi)
"""

import argparse
import json
import sys
import urllib.request
from pathlib import Path

API = "https://api.figshare.com/v2/articles/"
OUT = Path(__file__).resolve().parent.parent / "reference" / "figshare"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("items", nargs="+")
    ap.add_argument("--ext", default=".zvi")
    args = ap.parse_args()
    for item in args.items:
        article, _, wanted = item.partition(":")
        with urllib.request.urlopen(API + article, timeout=60) as r:
            info = json.load(r)
        licence = info.get("license", {}).get("name", "")
        if not (licence.startswith("CC BY") or licence.startswith("CC0")):
            print(f"skip {article}: licence {licence!r}", file=sys.stderr)
            continue
        files = [f for f in info.get("files", []) if (f["name"] == wanted if wanted else f["name"].lower().endswith(args.ext))]
        if not files:
            print(f"skip {article}: no matching file", file=sys.stderr)
            continue
        f = files[0]
        dest = OUT / article / f["name"]
        if dest.exists() and dest.stat().st_size == f["size"]:
            print(f"ok   {article}/{f['name']} ({licence})", file=sys.stderr)
            continue
        print(f"get  {article}/{f['name']} ({f['size'] >> 20} MiB, {licence})", file=sys.stderr)
        dest.parent.mkdir(parents=True, exist_ok=True)
        part = dest.with_name(dest.name + ".part")
        urllib.request.urlretrieve(f["download_url"], part)
        if part.stat().st_size != f["size"]:
            sys.exit(f"size mismatch: {article}/{f['name']}")
        part.rename(dest)


if __name__ == "__main__":
    main()

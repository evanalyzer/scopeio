#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Downloads an OME-Zarr store (NGFF 0.4/Zarr v2 or 0.5/Zarr v3) over HTTP.

Walks the metadata, then fetches every existing chunk (or shard) of the
selected resolution levels. Images, bioformats2raw series and plates
(with a limit on the number of wells) are followed; labels are skipped.

The data is not committed; it goes to tests/reference/zarr/ by default.

Usage:
  python3 -I tests/scripts/fetch_ome_zarr.py <url.zarr> [--out DIR]
      [--levels N] [--wells N]

Metadata of all levels is fetched; chunks only of the first N levels.
"""

import argparse
import json
import sys
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from itertools import product
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent


def get(url):
    """Body of `url`, or None if it does not exist."""
    try:
        with urllib.request.urlopen(url, timeout=60) as r:
            return r.read()
    except urllib.error.HTTPError as e:
        if e.code in (403, 404):
            return None
        raise


class Store:
    def __init__(self, url, out):
        self.url = url.rstrip("/")
        self.out = out
        self.version = None

    def fetch(self, key, required=False):
        dest = self.out / key
        if dest.exists():
            return dest.read_bytes()
        data = get(f"{self.url}/{key}")
        if data is None:
            if required:
                sys.exit(f"missing {self.url}/{key}")
            return None
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        return data

    def attrs(self, path):
        """Attributes of the group at `path` (v2 .zattrs or v3 zarr.json)."""
        prefix = f"{path}/" if path else ""
        if self.version in (None, 3):
            meta = self.fetch(prefix + "zarr.json")
            if meta is not None:
                self.version = 3
                return json.loads(meta).get("attributes", {}).get("ome", {})
        self.version = 2
        self.fetch(prefix + ".zgroup")
        meta = self.fetch(prefix + ".zattrs")
        return json.loads(meta) if meta else {}

    def array(self, path, chunks_too=True):
        """Downloads the metadata and, if `chunks_too`, all existing chunks of
        an array."""
        if self.version == 3:
            meta = json.loads(self.fetch(f"{path}/zarr.json", required=True))
            shape = meta["shape"]
            chunks = meta["chunk_grid"]["configuration"]["chunk_shape"]
            enc = meta.get("chunk_key_encoding", {"name": "default"})
            sep = enc.get("configuration", {}).get("separator", "/" if enc["name"] == "default" else ".")
            prefix = "c" + sep if enc["name"] == "default" else ""
        else:
            meta = json.loads(self.fetch(f"{path}/.zarray", required=True))
            shape, chunks = meta["shape"], meta["chunks"]
            sep = meta.get("dimension_separator", ".")
            prefix = ""
        if not chunks_too:
            return
        counts = [-(-s // c) for s, c in zip(shape, chunks)]
        keys = [f"{path}/{prefix}" + sep.join(map(str, idx)) for idx in product(*map(range, counts))]
        with ThreadPoolExecutor(16) as pool:
            done = sum(1 for d in pool.map(self.fetch, keys) if d is not None)
        print(f"{path}: {done}/{len(keys)} chunks", file=sys.stderr)

    def image(self, path, levels):
        attrs = self.attrs(path)
        for ms in attrs.get("multiscales", [])[:1]:
            # Metadata of every level; chunks of the first `levels` only, so
            # higher levels read as fill values.
            for i, ds in enumerate(ms["datasets"]):
                self.array(f"{path}/{ds['path']}" if path else ds["path"], i < levels)
        return attrs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("url")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--levels", type=int, default=99, help="resolution levels per image")
    ap.add_argument("--wells", type=int, default=4, help="wells of a plate")
    args = ap.parse_args()
    name = args.url.rstrip("/").split("/")[-1]
    out = args.out or ROOT / "tests" / "reference" / "zarr" / name
    store = Store(args.url, out)
    attrs = store.image("", args.levels)
    if "plate" in attrs:
        for well in attrs["plate"]["wells"][:args.wells]:
            for image in store.attrs(well["path"]).get("well", {}).get("images", []):
                store.image(f"{well['path']}/{image['path']}", args.levels)
    if "bioformats2raw.layout" in attrs:
        series = 0
        while store.fetch(f"{series}/.zattrs" if store.version == 2 else f"{series}/zarr.json"):
            store.image(str(series), args.levels)
            series += 1
        store.fetch("OME/METADATA.ome.xml")
    print(f"wrote {out}", file=sys.stderr)


if __name__ == "__main__":
    main()

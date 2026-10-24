#!/usr/bin/env python3
"""Downloads files of a Zenodo record into tests/reference/zenodo-<id>/.

Prints the record's licence and verifies each file's checksum from the
Zenodo API. Files are not committed (tests/reference is gitignored).

Usage: python3 -I tests/scripts/fetch_zenodo.py <record id> <file name>...
"""

import hashlib
import json
import sys
import urllib.request
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "reference"


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    record, names = sys.argv[1], sys.argv[2:]
    with urllib.request.urlopen(f"https://zenodo.org/api/records/{record}", timeout=60) as r:
        meta = json.load(r)
    print(f"record {record}: {meta['metadata']['title'][:70]} ({meta['metadata'].get('license', {}).get('id')})", file=sys.stderr)
    files = {f["key"]: f for f in meta["files"]}
    dest_dir = OUT / f"zenodo-{record}"
    dest_dir.mkdir(parents=True, exist_ok=True)
    (dest_dir / "record.json").write_text(json.dumps(meta, indent=1))
    for name in names:
        f = files[name]
        algo, want = f["checksum"].split(":", 1)
        dest = dest_dir / name
        if dest.exists() and dest.stat().st_size == f["size"]:
            print(f"ok   {name}", file=sys.stderr)
            continue
        print(f"get  {name} ({f['size'] >> 20} MiB)", file=sys.stderr)
        part = dest.with_name(dest.name + ".part")
        urllib.request.urlretrieve(f["links"]["self"], part)
        h = hashlib.new(algo)
        with open(part, "rb") as fh:
            for block in iter(lambda: fh.read(1 << 20), b""):
                h.update(block)
        if h.hexdigest() != want:
            sys.exit(f"checksum mismatch: {name}")
        part.rename(dest)


if __name__ == "__main__":
    main()

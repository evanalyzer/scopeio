#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Downloads camera raw samples from raw.pixls.us into
tests/reference/rawpixls/<id>.<ext>.

Only samples under CC0 are fetched (checked against the repository
listing); the files are not committed.

Usage: python3 -I tests/scripts/fetch_rawpixls.py <sample id>...
       (the number in https://raw.pixls.us/getfile.php/<id>/nice/...)
"""

import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

LISTING = "https://raw.pixls.us/json/getrepository.php"
OUT = Path(__file__).resolve().parent.parent / "reference" / "rawpixls"


def main():
    with urllib.request.urlopen(LISTING, timeout=60) as r:
        rows = json.load(r)["data"]
    urls = {}
    for row in rows:
        text = " ".join(map(str, row))
        for url in re.findall(r"https://raw.pixls.us/getfile.php/(\d+)/nice/([^'\"]+)", text):
            urls[url[0]] = (url[1], "publicdomain/zero" in text)
    OUT.mkdir(parents=True, exist_ok=True)
    for sample in sys.argv[1:]:
        if sample not in urls:
            print(f"skip {sample}: not in the listing", file=sys.stderr)
            continue
        name, cc0 = urls[sample]
        if not cc0:
            print(f"skip {sample}: not CC0", file=sys.stderr)
            continue
        dest = OUT / f"{sample}{Path(name).suffix.lower()}"
        if dest.exists():
            print(f"ok   {dest.name} ({name})", file=sys.stderr)
            continue
        print(f"get  {dest.name} ({name}, CC0)", file=sys.stderr)
        url = f"https://raw.pixls.us/getfile.php/{sample}/nice/{urllib.parse.quote(name)}"
        part = dest.with_name(dest.name + ".part")
        urllib.request.urlretrieve(url, part)
        part.rename(dest)


if __name__ == "__main__":
    main()

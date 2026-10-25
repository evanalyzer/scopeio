#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Measures CPU time and peak RAM of output options (examples/output_bench.rs).

Each run is a fresh process measured by tests/scripts/measure.rs (CPU time,
peak RSS, killed above MEASURE_MAX_RSS_MIB); the median of --repeat runs is
recorded. Results are written to tests/expected/output_bench.md: a new
table with --label (e.g. "before" / "after"), replacing an older table of
the same label.

Cases (files are downloaded by the tests/README.md commands, not committed):
- f32: full plane (3554 x 2540, uint16) of the vendor stitch of the OME
  sample Olympus-OIR/gh-4205/zenodo-13680725, z 26.
- rgb: a 4096 x 4096 region of OpenSlide's CMU-1.svs (JPEG tiles).

Usage: python3 -I tests/scripts/bench_output.py --label after
       [--binary target/release/examples/output_bench] [--repeat 5]
       [--scenarios native,native-convert,...]
"""

import argparse
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
MEASURE = ROOT / "target" / "release" / "measure"
MEASURE_SRC = ROOT / "tests" / "scripts" / "measure.rs"
OUT = ROOT / "tests" / "expected" / "output_bench.md"

OIR = "tests/reference/ome/Olympus-OIR/gh-4205/zenodo-13680725/Stitch_A01_G001.oir"
SVS = "tests/reference/openslide/Aperio/CMU-1.svs"
CASES = {
    "native": (OIR, "0,0,3554,2540", "26"),
    "native-convert": (OIR, "0,0,3554,2540", "26"),
    "f32-normalized": (OIR, "0,0,3554,2540", "26"),
    "f32-significant-bits": (OIR, "0,0,3554,2540", "26"),
    "f32-scaled": (OIR, "0,0,3554,2540", "26"),
    "rgb-3-sample-reads": (SVS, "20000,14000,4096,4096", "0"),
    "rgb-samples": (SVS, "20000,14000,4096,4096", "0"),
    "rgb-channel-chw": (SVS, "20000,14000,4096,4096", "0"),
}


def run(binary, scenario):
    path, region, z = CASES[scenario]
    start = time.perf_counter()
    p = subprocess.run([str(MEASURE), str(binary), scenario, str(ROOT / path), region, z],
                       capture_output=True, text=True)
    wall = time.perf_counter() - start
    m = re.search(r"@measure cpu_s=([\d.]+) maxrss_kib=(\d+)", p.stderr)
    if p.returncode != 0 or not m:
        sys.exit(f"{scenario} failed: {p.stderr.strip()[-300:]}")
    return float(m.group(1)), wall, int(m.group(2)) / 1024, p.stdout.strip().split("\t")[-1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--label", required=True)
    ap.add_argument("--binary", default=str(ROOT / "target" / "release" / "examples" / "output_bench"))
    ap.add_argument("--repeat", type=int, default=5)
    ap.add_argument("--scenarios", default=",".join(CASES))
    args = ap.parse_args()
    if not MEASURE.exists():
        subprocess.run(["rustc", "--edition", "2024", "-O", "-o", str(MEASURE), str(MEASURE_SRC)], check=True)
    rows = []
    for scenario in args.scenarios.split(","):
        runs = [run(args.binary, scenario) for _ in range(args.repeat)]
        cpu, wall, rss = (statistics.median(r[i] for r in runs) for i in range(3))
        rows.append((scenario, Path(CASES[scenario][0]).name, CASES[scenario][1], cpu, wall, rss, runs[0][3]))
        print(f"{scenario}: cpu {cpu:.3f} s, wall {wall:.3f} s, peak {rss:.1f} MiB", file=sys.stderr)
    table = [f"## {args.label}", "",
             f"{time.strftime('%Y-%m-%d')}, {platform.processor() or platform.machine()}, "
             f"{os.cpu_count()} threads, median of {args.repeat} runs, one process per run.", "",
             "| Scenario | File | Region | CPU s | Wall s | Peak RAM MiB | Checksum |",
             "|---|---|---|---:|---:|---:|---|"]
    table += [f"| {s} | {f} | {r} | {c:.3f} | {w:.3f} | {m:.1f} | {k} |" for s, f, r, c, w, m, k in rows]
    text = OUT.read_text() if OUT.exists() else "# Output options: CPU and RAM\n\nWritten by `tests/scripts/bench_output.py`.\n"
    # Replace an older table of the same label.
    text = re.sub(rf"\n## {re.escape(args.label)}\n.*?(?=\n## |\Z)", "", text, flags=re.S)
    OUT.write_text(text.rstrip("\n") + "\n\n" + "\n".join(table) + "\n")
    print(f"wrote {OUT}", file=sys.stderr)


if __name__ == "__main__":
    main()

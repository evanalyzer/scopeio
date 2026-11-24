#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Reference output of RosettaSciIO for CEOS Panta Rhei (.prz) and Gatan
DigitalMicrograph (.dm3, .dm4) files.

RosettaSciIO is GPL-3.0 and is treated like Bio-Formats: a black box.
It is installed only in tests/tools/rsciio (not committed, never
distributed), this script runs it as a separate process through its
documented public API (`rsciio.pantarhei.file_reader`,
`rsciio.digitalmicrograph.file_reader`, see the RosettaSciIO user guide), and only the results are committed. Its source
code must not be read, not even to debug a mismatch.

For each file of FORMAT (`prz` or `gatan`) this writes:

- one `rsciio` row per returned signal into tests/expected/FORMAT.tsv
  (replacing earlier `rsciio` rows; the Bio-Formats and bioformats-rs rows
  are kept), in the columns of tests/scripts/oracle.py, so
  `check_against_oracles(FORMAT)` compares pixels, shape, type and pixel
  size (signals that are not one image or a stack of images, e.g. spectra
  or RGB, are recorded as not compared);
- <dir>/<file name>.json with the axes, `metadata` and
  `original_metadata` of each signal, for the metadata comparison in
  tests/FORMAT.rs; <dir> is tests/expected/FORMAT_rsciio for the
  committed synthetic files and the uncommitted
  tests/reference/FORMAT_rsciio for all others (real samples, whose
  metadata stays out of the repository);
- CPU time and peak RAM of the read (this process, including the Python
  and NumPy start-up) into tests/expected/FORMAT_rsciio_bench.md.

Run it after `oracle.py FORMAT`, which rewrites FORMAT.tsv without the
`rsciio` rows.

Setup (once):

    python3 -m venv tests/tools/rsciio
    tests/tools/rsciio/bin/pip install rosettasciio==<version>

Usage: tests/tools/rsciio/bin/python -I tests/scripts/oracle_rsciio.py FORMAT FILE...
"""

import csv
import datetime
import json
import platform
import resource
import subprocess
import sys
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
# FORMAT -> RosettaSciIO reader module.
READERS = {"prz": "rsciio.pantarhei", "gatan": "rsciio.digitalmicrograph"}
EXPECTED = ROOT / "tests" / "expected"
COLUMNS = ["file", "tool", "series", "status", "channels", "width", "height", "size_z", "size_t", "pixel_type", "phys_x_nm",
           "phys_y_nm", "crc32", "grid", "crop", "ours_series"]
DTYPES = {"uint8": "U8", "int8": "I8", "uint16": "U16", "int16": "I16", "uint32": "U32", "int32": "I32", "float32": "F32",
          "float64": "F64", "bool": "U8"}
# Length units RosettaSciIO may report, to nm.
UNITS = {"m": 1e9, "mm": 1e6, "µm": 1e3, "um": 1e3, "nm": 1.0, "Å": 0.1, "A": 0.1, "pm": 1e-3}


def load_grid():
    """`grid()` of tests/scripts/oracle.py (same block means as the Rust side)."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from oracle import grid
    return grid


def plain(v):
    """JSON-safe copy of NumPy scalars, arrays and nested containers."""
    import numpy as np
    if isinstance(v, dict):
        return {str(k): plain(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [plain(x) for x in v]
    if isinstance(v, np.ndarray):
        return plain(v.tolist())
    if isinstance(v, np.generic):
        return plain(v.item())
    if isinstance(v, float) and v != v or v in (float("inf"), float("-inf")):
        return repr(v)  # NaN / infinity are not JSON
    if isinstance(v, (datetime.date, datetime.time)):
        return v.isoformat()
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return repr(v)


def axis_nm(axis):
    unit = str(axis.get("units") or "")
    scale = axis.get("scale")
    if unit not in UNITS or scale is None:
        return "-"
    return f"{float(scale) * UNITS[unit]:.3f}"


def rows_for(path, rel, grid, file_reader):
    import numpy as np
    signals = file_reader(str(path))
    rows, dumps = [], []
    for i, s in enumerate(signals):
        data = np.asarray(s["data"])
        axes = list(s.get("axes", []))
        nav = [a for a in axes if a.get("navigate")]
        sig = [a for a in axes if not a.get("navigate")]
        row = {c: "-" for c in COLUMNS}
        row.update(file=rel, tool="rsciio", series=str(i))
        dumps.append({"axes": plain(axes), "metadata": plain(s.get("metadata", {})),
                      "original_metadata": plain(s.get("original_metadata", {})), "shape": list(data.shape),
                      "dtype": str(data.dtype)})
        ptype = DTYPES.get(data.dtype.name)
        if data.ndim not in (2, 3) or ptype is None or len(sig) != 2:
            row["status"] = f"error: shape {data.shape} dtype {data.dtype} not compared"
            rows.append(row)
            continue
        # Signal axes are (y, x). A navigation axis is z for a length unit,
        # channels for an energy unit (spectrum images; interleaved like the
        # other references), else time.
        h, w = data.shape[-2:]
        n = data.shape[0] if data.ndim == 3 else 1
        unit = str(nav[0].get("units") or "") if nav else ""
        kind = "z" if unit in UNITS else "c" if unit in ("eV", "keV", "meV") else "t"
        if kind == "c":
            data = data.transpose(1, 2, 0)
        le = np.ascontiguousarray(data).astype(data.dtype.newbyteorder("<"), copy=False)
        pixels = le.astype(np.uint8).tobytes() if data.dtype == bool else le.tobytes()
        y_axis, x_axis = sig[0], sig[1]
        ch, planes = (n, 1) if kind == "c" else (1, n)
        row.update(status="ok", channels=str(ch), width=str(w), height=str(h), size_z=str(n if kind == "z" else 1),
                   size_t=str(n if kind == "t" else 1), pixel_type=ptype, phys_x_nm=axis_nm(x_axis),
                   phys_y_nm=axis_nm(y_axis), crc32=f"{zlib.crc32(pixels):08x}", grid=grid(pixels, ptype, ch, w, h, planes))
        if len(nav) > 1:
            row["status"] = f"error: {len(nav)} navigation axes not compared"
        rows.append(row)
    return rows, dumps


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in READERS:
        sys.exit(__doc__)
    fmt, files = sys.argv[1], [Path(f) for f in sys.argv[2:]]
    grid = load_grid()
    t0, c0 = time.perf_counter(), time.process_time()
    import importlib
    from importlib.metadata import version
    file_reader = importlib.import_module(READERS[fmt]).file_reader
    startup = (time.process_time() - c0, time.perf_counter() - t0)

    tsv = EXPECTED / f"{fmt}.tsv"
    kept = []
    if tsv.exists():
        with open(tsv, newline="") as f:
            kept = [r for r in csv.DictReader(f, delimiter="\t") if r["tool"] != "rsciio"]
    committed = ROOT / "tests" / "data"
    new, bench = [], []
    for path in files:
        rel = str(path.resolve().relative_to(ROOT))
        c, t = time.process_time(), time.perf_counter()
        try:
            rows, dumps = rows_for(path, rel, grid, file_reader)
        except Exception as e:  # recorded like a failing reference reader
            msg = " ".join(f"{type(e).__name__}: {e}".split())[:200]
            rows, dumps = [{**{k: "-" for k in COLUMNS}, "file": rel, "tool": "rsciio", "series": "0",
                            "status": f"error: {msg}"}], []
        bench.append((rel, path.stat().st_size, rows[0]["status"], time.process_time() - c, time.perf_counter() - t))
        new.extend(rows)
        out_dir = (EXPECTED if path.resolve().is_relative_to(committed) else ROOT / "tests" / "reference") / f"{fmt}_rsciio"
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"{path.name}.json").write_text(json.dumps(dumps, indent=1, ensure_ascii=False, sort_keys=True, allow_nan=False) + "\n")
        print(rel, rows[0]["status"])
    peak_mib = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024

    order = {}
    for r in kept + new:
        order.setdefault(r["file"], []).append(r)
    with open(tsv, "w", newline="") as f:
        w = csv.DictWriter(f, COLUMNS, delimiter="\t", lineterminator="\n", extrasaction="ignore")
        w.writeheader()
        for rs in order.values():
            w.writerows(rs)

    cpu_model = subprocess.run(["sh", "-c", "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2"], capture_output=True,
                               text=True).stdout.strip()
    with open(EXPECTED / f"{fmt}_rsciio_bench.md", "w") as f:
        f.write(f"# {fmt}: RosettaSciIO read times\n\n")
        f.write(f"Generated {datetime.date.today()} by `tests/scripts/oracle_rsciio.py`.\n\n")
        f.write(f"- Machine: {cpu_model}, {platform.system()} {platform.release()}\n")
        f.write(f"- RosettaSciIO {version('rosettasciio')}, Python {platform.python_version()}\n")
        f.write(f"- One process for all files; importing RosettaSciIO took {startup[0]:.3f} s CPU / {startup[1]:.3f} s wall.\n")
        f.write(f"- Peak RAM of the whole process (Python, NumPy, all files): {peak_mib:.1f} MiB.\n\n")
        f.write("| File | Size | Status | CPU s | Wall s |\n|---|---:|---|---:|---:|\n")
        for rel, size, status, cpu, wall in bench:
            f.write(f"| {rel} | {size // 1024} KiB | {status[:60]} | {cpu:.3f} | {wall:.3f} |\n")
    print(f"wrote {tsv}, the metadata dumps and {fmt}_rsciio_bench.md")


if __name__ == "__main__":
    main()

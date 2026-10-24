#!/usr/bin/env python3
"""Generates reference results and resource measurements for one format.

Runs Bio-Formats (bftools) and bioformats-rs strictly as separate processes,
converting each input to ICS, and records metadata and a CRC-32 of the pixel
data. Then measures CPU time and peak RAM of both and of scopeio's
`scopeio_info` example on the same files.

The tools are expected in tests/tools/ (not committed, see tests/README.md).
Outputs, both committed:
  tests/expected/<format>.tsv        reference values used by `cargo test`
  tests/expected/<format>_bench.md   CPU and RAM comparison

Usage: python3 -I tests/scripts/oracle.py <format> <file>... [--repeat N]
"""

import argparse
import array
import datetime
import re
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
TOOLS = ROOT / "tests" / "tools"
BFCONVERT = TOOLS / "bftools" / "bfconvert"
BFRS = TOOLS / "bioformats-rs" / "bin" / "bioformats-convert"
SCOPEIO = ROOT / "target" / "release" / "examples" / "scopeio_info"
MEASURE_SRC = ROOT / "tests" / "scripts" / "measure.rs"
MEASURE = ROOT / "target" / "release" / "measure"
EXPECTED = ROOT / "tests" / "expected"

COLUMNS = ["file", "tool", "series", "status", "channels", "width", "height", "size_z",
           "size_t", "pixel_type", "phys_x_nm", "phys_y_nm", "crc32", "grid", "crop", "ours_series"]

# Block means for comparing lossy data, see `grid()`.
GRID = 8
GRID_SAMPLES = 64
ARRAY_CODES = {"U8": "B", "I8": "b", "U16": "H", "I16": "h", "U32": "I", "I32": "i", "F32": "f",
               "F64": "d"}


def grid(pixels, pixel_type, channels, width, height, planes):
    """Means of GRID x GRID cells per channel, over all planes.

    Samples every `ceil(width / GRID_SAMPLES)`-th pixel in x and y, so the
    cost is bounded per plane. Must match `grid()` in tests/common/mod.rs.
    Returns `channels * GRID * GRID` values, channel-major, as text.
    """
    values = array.array(ARRAY_CODES[pixel_type])
    values.frombytes(pixels)
    sx = -(-width // GRID_SAMPLES)
    sy = -(-height // GRID_SAMPLES)
    sums = [0.0] * (channels * GRID * GRID)
    counts = [0] * (channels * GRID * GRID)
    plane_len = width * height * channels
    for p in range(planes):
        base = p * plane_len
        for y in range(0, height, sy):
            gy = y * GRID // height
            row = base + y * width * channels
            for x in range(0, width, sx):
                gx = x * GRID // width
                at = row + x * channels
                for c in range(channels):
                    i = (c * GRID + gy) * GRID + gx
                    sums[i] += values[at + c]
                    counts[i] += 1
    return ",".join(f"{s / n:.2f}" if n else "nan" for s, n in zip(sums, counts))


def run(cmd, measure=True):
    """Runs `cmd`, returning (exit code, output, CPU seconds, wall seconds, peak RSS KiB).

    CPU time and peak RSS come from the `measure` helper (see measure.rs);
    without `measure` they are 0.
    """
    start = time.perf_counter()
    proc = subprocess.run([str(MEASURE), *cmd] if measure else cmd, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT)
    wall = time.perf_counter() - start
    lines, cpu, rss = [], 0.0, 0
    for line in proc.stdout.decode(errors="replace").splitlines():
        m = re.fullmatch(r"@measure cpu_s=([\d.]+) maxrss_kib=(\d+)( killed=rss)?", line)
        if m:
            cpu, rss = float(m[1]), int(m[2])
            if m[3]:
                lines.append(f"killed: exceeded the RSS limit of {os.environ.get('MEASURE_MAX_RSS_MIB', 8192)} MiB")
        else:
            lines.append(line)
    return proc.returncode, "\n".join(lines), cpu, wall, rss


def parse_ics(path, int32_little=None, planar_colour=False):
    """Reads an ICS v2 file, returning its metadata and a CRC-32 of the pixels.

    Pixels are brought into one layout: channels interleaved per pixel, then
    x, y, z, t, native (little-endian) byte order.

    Quirks of the reference writers, observed on files with known content:
    the byte order labels of 32-bit integer data are unreliable; bfconvert
    writes them in the byte order of the source file, bioformats-rs in
    little-endian order. `int32_little` gives the actual order if known. bioformats-rs writes colour (3 or 4 channel) data plane
    by plane under an interleaved header (`planar_colour`).
    """
    data = path.read_bytes()
    end = data.find(b"\nend\n")
    header = data[:end].decode("latin-1")
    fields = {}
    for line in header.splitlines():
        parts = line.replace("\t", " ").split()
        if len(parts) >= 2:
            fields[" ".join(parts[:2])] = parts[2:]
    order = fields["layout order"]
    sizes = dict(zip(order, map(int, fields["layout sizes"])))
    bits = sizes.pop("bits")
    count = 1
    for v in sizes.values():
        count *= v
    pixels = data[len(data) - count * bits // 8:]
    fmt = fields.get("representation format", ["integer"])[0]
    signed = fields.get("representation sign", ["unsigned"])[0] == "signed"
    if fmt == "real":
        pixel_type = f"F{bits}"
    else:
        pixel_type = f"{'I' if signed else 'U'}{bits}"
    big_endian = fields.get("representation byte_order", ["1"])[0] != "1"
    if int32_little is not None and fmt == "integer" and bits == 32:
        big_endian = not int32_little
    values = array.array(ARRAY_CODES[pixel_type])
    values.frombytes(pixels)
    if big_endian and bits > 8:
        values.byteswap()
    axes = [a for a in order if a != "bits"]
    if planar_colour and sizes.get("ch", 1) > 1 and axes[0] == "ch":
        axes = axes[1:] + ["ch"]
    values = canonical_order(values, axes, sizes)
    pixels = values.tobytes()
    scale = dict(zip(order, fields.get("parameter scale", [])))
    units = dict(zip(order, fields.get("parameter units", [])))

    def nm(axis):
        value, unit = scale.get(axis), units.get(axis)
        if value is None or unit not in ("micrometers", "microns", "um"):
            return "-"
        value = float(value) * 1000.0
        # Writers put 1 for unknown sizes.
        return "-" if value == 1000.0 else f"{value:.3f}"

    ch, w, h = sizes.get("ch", 1), sizes.get("x", 1), sizes.get("y", 1)
    planes = count // max(1, ch * w * h)
    return {
        "channels": ch, "width": w, "height": h,
        "size_z": sizes.get("z", 1), "size_t": sizes.get("t", 1), "pixel_type": pixel_type,
        "phys_x_nm": nm("x"), "phys_y_nm": nm("y"), "crc32": f"{zlib.crc32(pixels):08x}",
        "grid": grid(pixels, pixel_type, ch, w, h, planes),
    }


def source_little(src):
    """Byte order of a TIFF source file, None for other formats."""
    if Path(src).is_dir():
        return None
    with open(src, "rb") as f:
        magic = f.read(2)
    return {b"II": True, b"MM": False}.get(magic)


def canonical_order(values, axes, sizes):
    """Reorders samples stored with axes `axes` (fastest first) to
    ch, x, y, z, t."""
    target = ["ch", "x", "y", "z", "t"]
    present = [a for a in axes if sizes.get(a, 1) > 1]
    if [a for a in target if a in present] == present:
        return values
    channels = sizes.get("ch", 1)
    if axes[-1] == "ch" and [a for a in axes[:-1] if a in present] == [a for a in target[1:] if a in present]:
        # Channel blocks one after another: interleave them.
        n = len(values) // channels
        out = array.array(values.typecode, bytes(len(values) * values.itemsize))
        for c in range(channels):
            out[c::channels] = values[c * n:(c + 1) * n]
        return out
    raise ValueError(f"unsupported ICS axis order {axes}")


def oracle(tool, src, tmp, expand=True, series=None, crop=None):
    """Converts `src` with `tool` and returns (rows, cpu, wall, rss).

    `series` and `crop` (x,y,w,h at full resolution) limit the conversion;
    bioformats-rs has no such options and is skipped then.
    """
    out_dir = Path(tempfile.mkdtemp(dir=tmp))
    if tool == "bioformats":
        cmd = [str(BFCONVERT), "-no-upgrade", "-overwrite", *(["-expand"] if expand else []),
               *(["-series", str(series)] if series is not None else []),
               *(["-crop", crop] if crop else []), str(src), str(out_dir / "out_%s.ics")]
    elif series is not None or crop:
        return [{"series": series or 0, "status": "skipped: no series or crop option"}], 0.0, 0.0, 0
    else:
        cmd = [str(BFRS), str(src), str(out_dir / "out.ics")]
    code, log, cpu, wall, rss = run(cmd)
    outputs = sorted(out_dir.glob("*.ics"), key=lambda p: (len(p.name), p.name))
    if code != 0 or not outputs:
        lines = log.strip().splitlines()
        reason = next((l for l in lines if "Exception" in l or "Error" in l),
                      lines[-1] if lines else f"exit {code}")
        reason = reason.replace('Exception in thread "main" ', "").replace(str(src), "<file>")
        return [{"series": 0, "status": "error: " + reason.replace("\t", " ")[:200]}], cpu, wall, rss
    rows = []
    for index, path in enumerate(outputs):
        number = series if series is not None else index
        try:
            row = {"series": number, "status": "ok",
                   **parse_ics(path, int32_little=source_little(src) if tool == "bioformats" else True,
                               planar_colour=tool == "bioformats-rs" and source_little(src) is not None)}
            if tool == "bioformats-rs":
                # Its ICS writer stores pixel sizes without unit conversion.
                row["phys_x_nm"] = row["phys_y_nm"] = "-"
            rows.append(row)
        except (ValueError, KeyError) as e:
            rows.append({"series": number, "status": f"error: unreadable output: {e}"})
    return rows, cpu, wall, rss


UNIT_NM = {"µm": 1e3, "um": 1e3, "nm": 1.0, "mm": 1e6, "cm": 1e7, "m": 1e9, "Å": 0.1}


def bf_physical_sizes(src):
    """Physical pixel sizes per series from Bio-Formats' OME-XML, in nanometres.

    The ICS scale line written by bfconvert does not follow its axis order,
    so the sizes are taken from `showinf -omexml-only` instead.
    """
    _, xml, *_ = run([str(TOOLS / "bftools" / "showinf"), "-no-upgrade", "-nopix", "-omexml-only",
                      str(src)], measure=False)
    sizes = []
    for pixels in re.findall(r"<Pixels\b[^>]*>", xml):
        attrs = dict(re.findall(r'(\w+)="([^"]*)"', pixels))

        def nm(axis):
            value = attrs.get(f"PhysicalSize{axis}")
            if value is None:
                return "-"
            unit = attrs.get(f"PhysicalSize{axis}Unit", "µm")
            return f"{float(value) * UNIT_NM.get(unit, float('nan')):.3f}"

        sizes.append((nm("X"), nm("Y")))
    return sizes


def ours(src, series=None, crop=None):
    opts = [*(["--series", str(series)] if series is not None else []), *(["--crop", crop] if crop else [])]
    code, out, cpu, wall, rss = run([str(SCOPEIO), "--no-checksum", *opts, str(src)])
    status = "ok" if code == 0 else "error: " + out.strip().splitlines()[-1][:200]
    return status, cpu, wall, rss


def tool_versions():
    _, bf, *_ = run([str(TOOLS / "bftools" / "showinf"), "-version"], measure=False)
    bf_version = next((l.split(":", 1)[1].strip() for l in bf.splitlines() if l.startswith("Version")),
                      "unknown")
    marker = TOOLS / "bioformats-rs" / ".crates.toml"
    rs_version = "unknown"
    if marker.exists():
        for line in marker.read_text().splitlines():
            if line.startswith('"bioformats '):
                rs_version = line.split()[1]
    return bf_version, rs_version


def machine():
    cpu = platform.processor() or platform.machine()
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    return f"{cpu}, {os.cpu_count()} threads, {platform.system()} {platform.release()}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("format")
    ap.add_argument("files", nargs="+", type=Path)
    ap.add_argument("--repeat", type=int, default=3, help="runs per tool for timing (median)")
    ap.add_argument("--no-expand", action="store_true",
                    help="keep palette indices in Bio-Formats output (formats returning indices)")
    ap.add_argument("--series", type=int, help="convert only this Bio-Formats series")
    ap.add_argument("--crop", help="convert only this region, x,y,w,h at full resolution")
    ap.add_argument("--ours-series",
                    help="scopeio series compared with --series (default: the same index), or "
                         "assoc:<kind> for an associated image (label, macro, thumbnail)")
    ap.add_argument("--keep-series", type=int,
                    help="record only this series of the reference output")
    ap.add_argument("--append", action="store_true", help="add to the existing reference files")
    ap.add_argument("--per-series", action="store_true",
                    help="record each Bio-Formats series from a conversion of that series alone "
                         "(bfconvert returns wrong pixels for later series of some multi-series files)")
    args = ap.parse_args()

    for tool in (BFCONVERT, BFRS):
        if not tool.exists():
            sys.exit(f"missing {tool}; see tests/README.md")
    subprocess.run(["cargo", "build", "--quiet", "--release", "--example", "scopeio_info"],
                   cwd=ROOT, check=True)
    subprocess.run(["rustc", "--edition", "2024", "-O", "-o", str(MEASURE), str(MEASURE_SRC)],
                   check=True)

    files = [f.resolve() for f in args.files]
    rows, bench = [], []
    with tempfile.TemporaryDirectory() as tmp:
        for src in files:
            rel = src.relative_to(ROOT).as_posix()
            print(rel, file=sys.stderr)
            stats = {}
            for tool in ("bioformats", "bioformats-rs"):
                runs = [oracle(tool, src, tmp, not args.no_expand, args.series, args.crop)
                        for _ in range(args.repeat)]
                phys = bf_physical_sizes(src) if tool == "bioformats" else []
                if tool == "bioformats" and args.per_series and args.series is None and len(runs[0][0]) > 1:
                    runs[0] = ([oracle(tool, src, tmp, not args.no_expand, r["series"])[0][0]
                                if r["status"] == "ok" else r for r in runs[0][0]], *runs[0][1:])
                for r in runs[0][0]:
                    if args.keep_series is not None and r["series"] != args.keep_series:
                        continue
                    if tool == "bioformats" and r["status"] == "ok":
                        x, y = phys[r["series"]] if r["series"] < len(phys) else ("-", "-")
                        r["phys_x_nm"], r["phys_y_nm"] = x, y
                    r["crop"] = args.crop or "-"
                    if args.ours_series is not None:
                        r["ours_series"] = args.ours_series
                    rows.append({"file": rel, "tool": tool, **r})
                stats[tool] = (runs[0][0][0]["status"], *(statistics.median(r[i] for r in runs)
                                                          for i in (1, 2, 3)))
            ours_series = args.ours_series if args.ours_series is not None else args.series
            if isinstance(ours_series, str) and not ours_series.isdigit():
                ours_series = None
            runs = [ours(src, ours_series, args.crop) for _ in range(args.repeat)]
            stats["scopeio"] = (runs[0][0], *(statistics.median(r[i] for r in runs) for i in (1, 2, 3)))
            size = sum(f.stat().st_size for f in src.rglob("*") if f.is_file()) if src.is_dir() else src.stat().st_size
            label = rel + (f" (series {args.series})" if args.series is not None else "") + \
                (f" crop {args.crop}" if args.crop else "")
            bench.append((label, size, stats))

    EXPECTED.mkdir(exist_ok=True)
    tsv = EXPECTED / f"{args.format}.tsv"
    append = args.append and tsv.exists()
    with open(tsv, "a" if append else "w") as f:
        if not append:
            f.write("\t".join(COLUMNS) + "\n")
        for r in rows:
            f.write("\t".join(str(r.get(c, "-")) for c in COLUMNS) + "\n")

    bf_version, rs_version = tool_versions()
    bench_md = EXPECTED / f"{args.format}_bench.md"
    if append and bench_md.exists():
        with open(bench_md, "a") as f:
            for rel, size, stats in bench:
                for tool in ("scopeio", "bioformats", "bioformats-rs"):
                    status, cpu, wall, rss = stats[tool]
                    status = "ok" if status == "ok" else status.replace("|", "/")[:60]
                    f.write(f"| {rel} | {size / 1024:.0f} KiB | {tool} | {status} | {cpu:.3f} | "
                            f"{wall:.3f} | {rss / 1024:.1f} |\n")
        print(f"appended to {tsv} and {bench_md.name}", file=sys.stderr)
        return
    with open(bench_md, "w") as f:
        f.write(f"# {args.format}: CPU and RAM compared with the reference readers\n\n")
        f.write(f"Generated {datetime.date.today()} by `tests/scripts/oracle.py`.\n\n")
        f.write(f"- Machine: {machine()}\n")
        f.write(f"- Bio-Formats {bf_version} (bftools, Java {platform_java()}), "
                f"bioformats-rs {rs_version}\n")
        f.write(f"- Median of {args.repeat} runs, each a fresh process.\n")
        f.write("- scopeio: `scopeio_info --no-checksum` example, reads every plane of every "
                "series.\n")
        f.write("- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own "
                "footprint of about 2 MiB is included. Runs above "
                f"{os.environ.get('MEASURE_MAX_RSS_MIB', 8192)} MiB RSS are killed.\n")
        f.write("- Reference readers: convert the file to ICS, which also writes the pixels "
                "to disk. Bio-Formats times include JVM start-up.\n\n")
        f.write("| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |\n")
        f.write("|---|---:|---|---|---:|---:|---:|\n")
        for rel, size, stats in bench:
            for tool in ("scopeio", "bioformats", "bioformats-rs"):
                status, cpu, wall, rss = stats[tool]
                status = "ok" if status == "ok" else status.replace("|", "/")[:60]
                f.write(f"| {rel} | {size / 1024:.0f} KiB | {tool} | {status} | {cpu:.3f} | "
                        f"{wall:.3f} | {rss / 1024:.1f} |\n")
    print(f"wrote {EXPECTED / (args.format + '.tsv')} and {args.format}_bench.md", file=sys.stderr)


def platform_java():
    try:
        out = subprocess.run(["java", "-version"], capture_output=True, text=True).stderr
        return out.split('"')[1]
    except (OSError, IndexError):
        return "unknown"


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic OME-Zarr stores covering the variants of the format.

Written from the OME-NGFF 0.4/0.5, Zarr v2 and Zarr v3 specifications in
doc/ome-zarr/. Also writes truth.tsv with the CRC-32 of each series in the
layout of tests/common/mod.rs `decode()` (channels interleaved per pixel,
planes ordered by z then t, native byte order). Output in tests/data/zarr/
is committed.

Usage: python3 -I tests/scripts/make_zarr_samples.py
"""

import gzip
import json
import shutil
import struct
import zlib
from itertools import product
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "zarr"
TRUTH = []


def crc32c(data):
    """CRC-32C (Castagnoli), as used by the Zarr v3 crc32c codec."""
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82F63B78 if crc & 1 else 0)
    return crc ^ 0xFFFFFFFF


def value(idx):
    """Deterministic sample value of element `idx` (a tuple of coordinates)."""
    n = 0
    for i, v in enumerate(idx):
        n = n * 131 + v * (17 + 6 * i)
    return (n * 2654435761 >> 7) % 60000


def write_json(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=1))


def elements(shape, start, chunk, order):
    """Coordinates of one chunk's elements in storage order (C or F)."""
    ranges = [range(s, s + c) for s, c in zip(start, chunk)]
    if order == "F":
        return [tuple(reversed(i)) for i in product(*reversed(ranges))]
    return list(product(*ranges))


def encode_chunk(shape, start, chunk, fmt, order, fill):
    vals = [value(i) if all(v < s for v, s in zip(i, shape)) else fill for i in elements(shape, start, chunk, order)]
    return struct.pack(fmt[0] + fmt[1] * len(vals), *vals)


def array_v2(path, shape, chunk, dtype="<u2", compressor=None, order="C", sep="/", missing=(), fill=0):
    fmt = (dtype[0] if dtype[0] in "<>" else "<", {"u2": "H", "u1": "B", "f4": "f"}[dtype[1:]])
    write_json(path / ".zarray", {
        "zarr_format": 2, "shape": shape, "chunks": chunk, "dtype": dtype, "order": order,
        "compressor": compressor, "fill_value": fill, "filters": None, "dimension_separator": sep,
    })
    counts = [-(-s // c) for s, c in zip(shape, chunk)]
    for idx in product(*map(range, counts)):
        if idx in missing:
            continue
        start = [i * c for i, c in zip(idx, chunk)]
        raw = encode_chunk(shape, start, chunk, fmt, order, fill)
        if compressor and compressor["id"] == "zlib":
            raw = zlib.compress(raw)
        elif compressor and compressor["id"] == "gzip":
            raw = gzip.compress(raw, mtime=0)
        key = path / sep.join(map(str, idx))
        key.parent.mkdir(parents=True, exist_ok=True)
        key.write_bytes(raw)


def array_v3(path, shape, chunk, inner=None, compress=None, crc=False, missing=(), fill=0, endian="little"):
    """uint16 array; `inner` enables sharding with inner chunk shape `inner`."""
    fmt = ("<" if endian == "little" else ">", "H")
    chunk_codecs = [{"name": "bytes", "configuration": {"endian": endian}}]
    if compress == "gzip":
        chunk_codecs.append({"name": "gzip", "configuration": {"level": 5}})
    if crc:
        chunk_codecs.append({"name": "crc32c"})
    codecs = chunk_codecs
    if inner:
        codecs = [{"name": "sharding_indexed", "configuration": {
            "chunk_shape": inner, "codecs": chunk_codecs,
            "index_codecs": [{"name": "bytes", "configuration": {"endian": "little"}}, {"name": "crc32c"}],
            "index_location": "end"}}]
    write_json(path / "zarr.json", {
        "zarr_format": 3, "node_type": "array", "shape": shape, "data_type": "uint16",
        "chunk_grid": {"name": "regular", "configuration": {"chunk_shape": chunk}},
        "chunk_key_encoding": {"name": "default", "configuration": {"separator": "/"}},
        "codecs": codecs, "fill_value": fill,
    })

    def encode(start, shp):
        raw = encode_chunk(shape, start, shp, fmt, "C", fill)
        if compress == "gzip":
            raw = gzip.compress(raw, mtime=0)
        if crc:
            raw += struct.pack("<I", crc32c(raw))
        return raw

    counts = [-(-s // c) for s, c in zip(shape, chunk)]
    for idx in product(*map(range, counts)):
        start = [i * c for i, c in zip(idx, chunk)]
        if inner:
            per = [c // i for c, i in zip(chunk, inner)]
            body, index = b"", b""
            for sub in product(*map(range, per)):
                if (idx, sub) in missing:
                    index += struct.pack("<QQ", 2**64 - 1, 2**64 - 1)
                    continue
                data = encode([s + i * n for s, i, n in zip(start, sub, inner)], inner)
                index += struct.pack("<QQ", len(body), len(data))
                body += data
            raw = body + index + struct.pack("<I", crc32c(index))
        else:
            if idx in missing:
                continue
            raw = encode(start, chunk)
        key = path / "c" / "/".join(map(str, idx))
        key.parent.mkdir(parents=True, exist_ok=True)
        key.write_bytes(raw)


def multiscales(axes, scales, version="0.4"):
    names = {"t": "time", "c": "channel"}
    return [{
        "version": version,
        "axes": [{"name": a, "type": names.get(a, "space"), **({"unit": "micrometer"} if a in "zyx" else {}),
                  **({"unit": "second"} if a == "t" else {})} for a in axes],
        "datasets": [{"path": str(i), "coordinateTransformations": [{"type": "scale", "scale": s}]}
                     for i, s in enumerate(scales)],
    }]


def record(name, series, axes, shape, fill=0, missing_fn=None, endian="little"):
    """Expected decoding of level 0: planes z then t, channels per pixel."""
    pos = {a: i for i, a in enumerate(axes)}
    size = lambda a: shape[pos[a]] if a in pos else 1
    out = bytearray()
    for t in range(size("t")):
        for z in range(size("z")):
            for y in range(size("y")):
                for x in range(size("x")):
                    for c in range(size("c")):
                        coord = {"t": t, "c": c, "z": z, "y": y, "x": x}
                        idx = tuple(coord[a] for a in axes)
                        v = fill if missing_fn and missing_fn(idx) else value(idx)
                        out += struct.pack("<H", v)
    TRUTH.append((name, series, size("c"), size("z"), size("t"), f"{zlib.crc32(bytes(out)):08x}"))


def image_v2(name, axes, shape, chunk, **kw):
    root = OUT / name
    write_json(root / ".zgroup", {"zarr_format": 2})
    scales = [[1.0] * (len(axes) - 2) + [0.5, 0.5], [1.0] * (len(axes) - 2) + [1.0, 1.0]]
    attrs = {"multiscales": multiscales(axes, scales)}
    if "c" in axes:
        attrs["omero"] = {"channels": [{"label": f"ch{i}", "color": ["FF0000", "00FF00", "0000FF"][i % 3]}
                                       for i in range(shape[axes.index("c")])]}
    write_json(root / ".zattrs", attrs)
    array_v2(root / "0", shape, chunk, **kw)
    half = [s if a not in "yx" else -(-s // 2) for a, s in zip(axes, shape)]
    array_v2(root / "1", half, chunk, dtype=kw.get("dtype", "<u2"))
    return root


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    # Zarr v2 / NGFF 0.4 variants.
    image_v2("v2_raw_cyx.zarr", "cyx", [2, 13, 11], [1, 5, 4])
    record("v2_raw_cyx.zarr", 0, "cyx", [2, 13, 11])
    image_v2("v2_zlib_tczyx.zarr", "tczyx", [2, 2, 3, 9, 7], [1, 1, 2, 4, 4], compressor={"id": "zlib", "level": 5})
    record("v2_zlib_tczyx.zarr", 0, "tczyx", [2, 2, 3, 9, 7])
    image_v2("v2_gzip_dot_sep.zarr", "zyx", [3, 8, 10], [2, 3, 3], compressor={"id": "gzip", "level": 5}, sep=".")
    record("v2_gzip_dot_sep.zarr", 0, "zyx", [3, 8, 10])
    image_v2("v2_fortran_be.zarr", "cyx", [3, 7, 6], [2, 4, 4], dtype=">u2", order="F")
    record("v2_fortran_be.zarr", 0, "cyx", [3, 7, 6])
    missing = {(0, 1, 0), (1, 0, 1)}
    image_v2("v2_missing_chunks.zarr", "cyx", [2, 8, 8], [1, 4, 4], missing=missing, fill=7)
    record("v2_missing_chunks.zarr", 0, "cyx", [2, 8, 8], fill=7,
           missing_fn=lambda i: (i[0], i[1] // 4, i[2] // 4) in missing)
    image_v2("v2_yx.zarr", "yx", [6, 9], [6, 9])
    record("v2_yx.zarr", 0, "yx", [6, 9])

    # Zarr v3 / NGFF 0.5 variants.
    for name, kw in [
        ("v3_gzip_crc.zarr", dict(compress="gzip", crc=True)),
        ("v3_shard_gzip.zarr", dict(inner=[1, 4, 4], compress="gzip", missing={((0, 0, 1), (0, 1, 0))}, fill=3)),
        ("v3_big_endian.zarr", dict(endian="big")),
    ]:
        root = OUT / name
        shape, chunk = [2, 12, 10], [1, 8, 8]
        write_json(root / "zarr.json", {"zarr_format": 3, "node_type": "group", "attributes": {
            "ome": {"version": "0.5", "multiscales": multiscales("cyx", [[1.0, 0.25, 0.25]], "0.5")}}})
        array_v3(root / "0", shape, chunk, **kw)
        missing_fn = None
        if "missing" in kw:
            missing_fn = lambda i: i[0] == 0 and i[1] // 8 == 0 and i[2] // 8 == 1 and (i[1] % 8) // 4 == 1 and (i[2] % 8) // 4 == 0
        record(name, 0, "cyx", shape, fill=kw.get("fill", 0), missing_fn=missing_fn)

    # Plate with two wells of two fields each.
    root = OUT / "plate.zarr"
    write_json(root / ".zgroup", {"zarr_format": 2})
    wells = [("B", "3", 1, 2), ("C", "1", 2, 0)]
    write_json(root / ".zattrs", {"plate": {
        "version": "0.4", "name": "test plate", "field_count": 2,
        "rows": [{"name": r} for r in "ABC"], "columns": [{"name": c} for c in "123"],
        "wells": [{"path": f"{r}/{c}", "rowIndex": ri, "columnIndex": ci} for r, c, ri, ci in wells]}})
    series = 0
    for r, c, _, _ in wells:
        write_json(root / r / c / ".zgroup", {"zarr_format": 2})
        write_json(root / r / c / ".zattrs", {"well": {"version": "0.4", "images": [{"path": "0"}, {"path": "1"}]}})
        for f in range(2):
            img = root / r / c / str(f)
            write_json(img / ".zgroup", {"zarr_format": 2})
            write_json(img / ".zattrs", {"multiscales": multiscales("yx", [[0.5, 0.5]])})
            array_v2(img / "0", [5, 6], [5, 6])
            record("plate.zarr", series, "yx", [5, 6])
            series += 1

    # bioformats2raw layout with two series.
    root = OUT / "bioformats2raw.zarr"
    write_json(root / ".zgroup", {"zarr_format": 2})
    write_json(root / ".zattrs", {"bioformats2raw.layout": 3})
    for s, shape in enumerate([[4, 5], [3, 7]]):
        img = root / str(s)
        write_json(img / ".zgroup", {"zarr_format": 2})
        write_json(img / ".zattrs", {"multiscales": multiscales("yx", [[1.0, 1.0]])})
        array_v2(img / "0", shape, shape)
        record("bioformats2raw.zarr", s, "yx", shape)
    (root / "OME").mkdir()
    (root / "OME" / "METADATA.ome.xml").write_text('<?xml version="1.0"?><OME xmlns="http://www.openmicroscopy.org/Schemas/OME/2016-06"/>')

    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tchannels\tsize_z\tsize_t\tcrc32\n")
        for row in TRUTH:
            f.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main()

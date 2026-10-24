#!/usr/bin/env python3
"""Writes a small synthetic Imaris 5 file (HDF5).

The HDF5 structures follow the HDF5 File Format Specification
(doc/hdf5/): superblock version 0, version 1 object headers, groups as
symbol tables (version 1 B-tree, symbol table node, local heap), version 1
attributes holding arrays of one-character strings (as Imaris writes
them), contiguous and chunked datasets (version 1 B-tree chunk index) with
the deflate + shuffle and LZ4 (32004) filters. The Imaris layout follows
doc/bitplane-imaris-ims/: DataSet/ResolutionLevel L/TimePoint T/Channel
C/Data and DataSetInfo. Two levels, two time points, two channels; level 0
of channel 1 is padded to whole chunks. Also writes truth.tsv with the
CRC-32 of each level in the layout of tests/common/mod.rs `decode_level()`.
Output in tests/data/ims/ is committed.

Usage: python3 -I tests/scripts/make_ims_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "ims"
UNDEF = 0xFFFFFFFFFFFFFFFF


def pad8(b):
    return b + bytes(-len(b) % 8)


class H5:
    def __init__(self):
        self.data = bytearray(bytes(96))  # superblock written last

    def put(self, b):
        at = len(self.data)
        self.data += pad8(b)
        return at

    # --- messages -------------------------------------------------------

    @staticmethod
    def dataspace(dims):
        return struct.pack("<BBBB4x", 1, len(dims), 0, 0) + b"".join(struct.pack("<Q", d) for d in dims)

    @staticmethod
    def datatype(kind, size):
        cls = {"u": 0x10, "f": 0x11, "s": 0x13}[kind]
        props = struct.pack("<HH", 0, 8 * size) if kind == "u" else b""
        if kind == "f":
            props = struct.pack("<HHBBBBI", 0, 32, 23, 8, 0, 23, 127)
            return struct.pack("<B3sI", cls, bytes([0x20, 0x1F, 0]), size) + props
        return struct.pack("<B3sI", cls, bytes(3), size) + props

    def attribute(self, name, text):
        n = name.encode() + b"\0"
        dt = self.datatype("s", 1)
        ds = self.dataspace([len(text)])
        body = struct.pack("<BBHHH", 1, 0, len(n), len(dt), len(ds)) + pad8(n) + pad8(dt) + pad8(ds) + text.encode()
        return (0x000C, body)

    def object(self, messages):
        """A version 1 object header; returns its address."""
        body = b""
        for typ, data in messages:
            data = pad8(data)
            body += struct.pack("<HHB3x", typ, len(data), 0) + data
        head = struct.pack("<BBHII4x", 1, 0, len(messages), 1, len(body))
        return self.put(head + body)

    # --- groups ---------------------------------------------------------

    def group(self, children, attrs=()):
        """A symbol-table group of (name, object address); returns its header."""
        children = sorted(children)
        heap = b"\0" * 8
        offsets = []
        for name, _ in children:
            offsets.append(len(heap))
            heap += pad8(name.encode() + b"\0")
        heap_data = self.put(heap)
        heap_at = self.put(b"HEAP" + bytes([0, 0, 0, 0]) + struct.pack("<QQQ", len(heap), UNDEF, heap_data))
        entries = b"".join(struct.pack("<QQII16x", off, addr, 0, 0) for off, (_, addr) in zip(offsets, children))
        entries += bytes(40 * (8 - len(children))) if len(children) < 8 else b""
        snod = self.put(b"SNOD" + struct.pack("<BBH", 1, 0, len(children)) + entries)
        last = offsets[-1] if offsets else 0
        tree = self.put(b"TREE" + struct.pack("<BBHQQ", 0, 0, 1, UNDEF, UNDEF) + struct.pack("<QQQ", 0, snod, last))
        msgs = [(0x0011, struct.pack("<QQ", tree, heap_at))]
        msgs += [self.attribute(k, v) for k, v in attrs]
        return self.object(msgs)

    # --- datasets -------------------------------------------------------

    def contiguous(self, dims, kind, size, raw):
        at = self.put(raw)
        layout = struct.pack("<BBQQ", 3, 1, at, len(raw))
        return self.object([(0x0001, self.dataspace(dims)), (0x0003, self.datatype(kind, size)), (0x0008, layout)])

    def chunked(self, dims, chunk, size, raw_chunks, filters):
        """`raw_chunks`: {(z, y, x) element offsets: encoded bytes}."""
        nd = len(dims)
        addrs = []
        for off, b in sorted(raw_chunks.items()):
            addrs.append((off, self.put(b), len(b)))
        keys = b""
        for off, at, n in addrs:
            keys += struct.pack("<II", n, 0) + b"".join(struct.pack("<Q", o) for o in off) + struct.pack("<Q", 0) + struct.pack("<Q", at)
        # Final key: beyond the last chunk.
        keys += struct.pack("<II", 0, 0) + b"".join(struct.pack("<Q", d) for d in dims) + struct.pack("<Q", 0)
        tree = self.put(b"TREE" + struct.pack("<BBHQQ", 1, 0, len(addrs), UNDEF, UNDEF) + keys)
        layout = struct.pack("<BBBQ", 3, 2, nd + 1, tree) + b"".join(struct.pack("<I", c) for c in chunk) + struct.pack("<I", size)
        pipe = struct.pack("<BB6x", 1, len(filters))
        for fid, client in filters:
            pipe += struct.pack("<HHHH", fid, 0, 0, len(client)) + b"".join(struct.pack("<I", c) for c in client)
            if len(client) % 2:
                pipe += bytes(4)
        return self.object([(0x0001, self.dataspace(dims)), (0x0003, self.datatype("u", size)), (0x0008, layout), (0x000B, pipe)])

    def finish(self, root):
        sb = b"\x89HDF\r\n\x1a\n" + bytes([0, 0, 0, 0, 0, 8, 8, 0]) + struct.pack("<HHI", 4, 16, 0)
        sb += struct.pack("<QQQQ", 0, UNDEF, len(self.data), UNDEF)
        sb += struct.pack("<QQII16x", 0, root, 0, 0)
        self.data[:len(sb)] = sb
        return bytes(self.data)


def value(c, t, z, y, x):
    return (c * 1000 + t * 333 + z * 71 + y * 13 + x * 3) % 4096


def shuffle(b, size):
    n = len(b) // size
    return bytes(b[i * size + k] for k in range(size) for i in range(n))


def lz4_filter(raw):
    """HDF5 LZ4 filter framing with one literal-only LZ4 block."""
    n = len(raw)
    lit = bytes([0xF0]) + bytes([255] * ((n - 15) // 255)) + bytes([(n - 15) % 255]) + raw if n >= 15 else bytes([n << 4]) + raw
    return struct.pack(">QI", n, n) + struct.pack(">I", len(lit)) + lit


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    h = H5()
    sizes = [(2, 6, 8), (1, 3, 4)]  # z, y, x per level
    truth = []
    levels = []
    for lv, (Z, Y, X) in enumerate(sizes):
        tps = []
        planes = {}
        for t in range(2):
            chans = []
            for c in range(2):
                vol = {(z, y, x): value(c, t, z, y, x) >> lv for z in range(Z) for y in range(Y) for x in range(X)}
                for z in range(Z):
                    planes[(c, t, z)] = vol
                if c == 0:
                    raw = b"".join(struct.pack("<H", vol[(z, y, x)]) for z in range(Z) for y in range(Y) for x in range(X))
                    data = h.contiguous([Z, Y, X], "u", 2, raw)
                else:
                    # Chunks of (1, 4, 4), stored volume padded to whole chunks.
                    cz, cy, cx = 1, 4, 4
                    SZ, SY, SX = Z, -(-Y // cy) * cy, -(-X // cx) * cx
                    chunks = {}
                    for gz in range(0, SZ, cz):
                        for gy in range(0, SY, cy):
                            for gx in range(0, SX, cx):
                                raw = b"".join(struct.pack("<H", vol.get((z, y, x), 0)) for z in range(gz, gz + cz) for y in range(gy, gy + cy) for x in range(gx, gx + cx))
                                chunks[(gz, gy, gx)] = zlib.compress(shuffle(raw, 2)) if lv == 0 else lz4_filter(raw)
                    filters = [(2, [2]), (1, [6])] if lv == 0 else [(32004, [])]
                    data = h.chunked([SZ, SY, SX], [cz, cy, cx], 2, chunks, filters)
                attrs = [("ImageSizeX", str(X)), ("ImageSizeY", str(Y)), ("ImageSizeZ", str(Z))]
                chans.append((f"Channel {c}", h.group([("Data", data)], attrs)))
            tps.append((f"TimePoint {t}", h.group(chans)))
        levels.append((f"ResolutionLevel {lv}", h.group(tps)))
        # decode_level layout: planes t, z; channels interleaved per pixel.
        # Every level has the z planes of level 0, each the nearest stored one.
        out = bytearray()
        Z0 = sizes[0][0]
        for t in range(2):
            for z0 in range(Z0):
                z = z0 * Z // Z0
                for y in range(Y):
                    for x in range(X):
                        for c in range(2):
                            out += struct.pack("<H", value(c, t, z, y, x) >> lv)
        truth.append((f"level{lv}", out))
    dataset = h.group(levels)
    image = h.group([], [("X", "8"), ("Y", "6"), ("Z", "2"), ("ExtMin0", "0"), ("ExtMax0", "4"), ("ExtMin1", "0"), ("ExtMax1", "3"),
                         ("ExtMin2", "-1"), ("ExtMax2", "2"), ("Unit", "um"), ("RecordingDate", "2021-07-08 09:10:11.000"),
                         ("Name", "C:/data/synthetic.lsm"), ("LensPower", "63"), ("NumericalAperture", "1.4")])
    ch0 = h.group([], [("Name", "DAPI"), ("Color", "0 0 1"), ("LSMEmissionWavelength", "461")])
    ch1 = h.group([], [("Name", "GFP"), ("Color", "0 1 0")])
    times = h.group([], [("TimePoint1", "2021-07-08 09:10:11.000"), ("TimePoint2", "2021-07-08 09:10:41.500")])
    info = h.group([("Image", image), ("Channel 0", ch0), ("Channel 1", ch1), ("TimeInfo", times)])
    root = h.group([("DataSet", dataset), ("DataSetInfo", info)], [("ImarisDataSet", "ImarisDataSet"), ("ImarisVersion", "5.5.0")])
    (OUT / "synthetic.ims").write_bytes(h.finish(root))
    with open(OUT / "truth.tsv", "w") as f:
        f.write("image\tcrc32\n")
        for name, px in truth:
            f.write(f"{name}\t{zlib.crc32(bytes(px)):08x}\n")


if __name__ == "__main__":
    main()

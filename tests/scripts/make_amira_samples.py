#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes small synthetic AmiraMesh files.

Layout from the Amira 3.1 manual (chapter "AmiraMesh Format", doc/amira/)
and T. Weinkauf's AmiraMesh notes (doc/amira/):

- le_ushort.am: BINARY-LITTLE-ENDIAN 2.1, ushort 7 x 5 x 3, uniform
  BoundingBox 0 3 0 2 0 4 (0.5 x 0.5 x 2 per voxel, no unit: um).
- be_short.am: BINARY (big endian) 2.1, short 2D lattice 7 x 5,
  Units { Coordinates "nm" }, BoundingBox 10 22 0 8 0 0 (2 nm pixels).
- ascii_float.am: ASCII 1.0, float 7 x 5 x 2, `= @1` declarations,
  comments, no "# Data section follows" line.
- vector3.am: `# AmiraMesh 3D BINARY-LITTLE-ENDIAN 2.1`, float[3] 7 x 5 x 2
  (three interleaved components: three channels).
- zip_int.am: int 7 x 5 x 4, @1(HxZip,<size>).
- rle_byte.am: byte 7 x 5 x 3, @1(HxByteRLE,<size>): runs and literals.
- rle_slice.am: the first slice of rle_byte.am alone (Bio-Formats decodes
  HxByteRLE of one slice only).
- zip_nosize.am: `# Avizo 3D BINARY 2.0` header as written by Avizo,
  @1(HxZip,0) (size not given), nested Parameters groups with their own
  BoundingBox and `&key "base64"` entries, CRLF line ends.
- labels.am: label field: Materials, Lattice { byte Labels } @1 and
  Lattice { byte Probability } @2(HxByteRLE,<size>): two fields.
- stacked_double.am: double 7 x 5 x 3, CoordType "stacked" with
  BoundingBoxXY and a Coordinates { float z } block @2 before @1 in the
  data section.

HxByteRLE as written here: a control byte n; n < 128: the next byte is
repeated n times; n >= 128: n - 128 literal bytes follow. Each z slice is
encoded on its own (Bio-Formats rejects packets across slices).

Also writes truth.tsv with the CRC-32 of each file's first series in the
layout of tests/common/mod.rs `decode_level()` (planes z by z, channels
interleaved, little endian; the two label fields are two channels). Output in tests/data/amira/ is committed.

Usage: python3 -I tests/scripts/make_amira_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "amira"
W, H = 7, 5


def rle(data):
    """HxByteRLE: runs of 3 or more equal bytes, literals otherwise."""
    out, i = bytearray(), 0
    while i < len(data):
        run = 1
        while i + run < len(data) and data[i + run] == data[i] and run < 127:
            run += 1
        if run >= 3:
            out += bytes([run, data[i]])
            i += run
            continue
        j = i
        while j < len(data) and j - i < 127:
            if j + 2 < len(data) and data[j] == data[j + 1] == data[j + 2]:
                break
            j += 1
        out += bytes([0x80 | (j - i)]) + data[i:j]
        i = j
    return bytes(out)


def rle_slices(data, slice_len):
    return b"".join(rle(data[i:i + slice_len]) for i in range(0, len(data), slice_len))


def ascii_values(values, per_line=7):
    lines = [" ".join(values[i:i + per_line]) for i in range(0, len(values), per_line)]
    return ("\n".join(lines) + "\n").encode()


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    truth = []

    # Little-endian ushort volume.
    v = [(x * 1000 + y * 77 + z * 9000) % 65536 for z in range(3) for y in range(H) for x in range(W)]
    le = struct.pack(f"<{len(v)}H", *v)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\n\n\ndefine Lattice 7 5 3\n\nParameters {\n"
            b"    Content \"7x5x3 ushort, uniform coordinates\",\n    BoundingBox 0 3 0 2 0 4,\n"
            b"    CoordType \"uniform\"\n}\n\nLattice { ushort Data } @1\n\n# Data section follows\n@1\n")
    (OUT / "le_ushort.am").write_bytes(head + le + b"\n")
    truth.append(("le_ushort.am", zlib.crc32(le)))

    # Big-endian signed short, 2D lattice, nm units.
    s = [(x * 900 - y * 3000 - 5000) for y in range(H) for x in range(W)]
    head = (b"# AmiraMesh BINARY 2.1\ndefine Lattice 7 5\nParameters {\n    Units {\n        Coordinates \"nm\"\n    }\n"
            b"    BoundingBox 10 22 0 8 0 0,\n    CoordType \"uniform\"\n}\nLattice { short Data } @1\n"
            b"# Data section follows\n@1\n")
    (OUT / "be_short.am").write_bytes(head + struct.pack(f">{len(s)}h", *s) + b"\n")
    truth.append(("be_short.am", zlib.crc32(struct.pack(f"<{len(s)}h", *s))))

    # ASCII float.
    f = [x * 0.5 - y * 0.25 + z * 10 for z in range(2) for y in range(H) for x in range(W)]
    head = (b"# AmiraMesh ASCII 1.0\n\n# Dimensions in x-, y-, and z-direction\ndefine Lattice 7 5 2\n\n"
            b"Parameters {\n    CoordType \"uniform\",\n    # BoundingBox is xmin xmax ymin ymax zmin zmax\n"
            b"    BoundingBox 0 6 0 4 0 1\n}\n\nLattice { float ScalarField } = @1\n\n@1\n")
    (OUT / "ascii_float.am").write_bytes(head + ascii_values([repr(x) for x in f]))
    truth.append(("ascii_float.am", zlib.crc32(struct.pack(f"<{len(f)}f", *f))))

    # Three interleaved float components.
    vec = [x + c * 100 + y * 10 + z * 1000 for z in range(2) for y in range(H) for x in range(W) for c in range(3)]
    raw = struct.pack(f"<{len(vec)}f", *vec)
    head = (b"# AmiraMesh 3D BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 2\nParameters {\n"
            b"    BoundingBox 0 6 0 4 0 1,\n    CoordType \"uniform\"\n}\nLattice { float[3] Vectors } @1\n"
            b"# Data section follows\n@1\n")
    (OUT / "vector3.am").write_bytes(head + raw + b"\n")
    truth.append(("vector3.am", zlib.crc32(raw)))

    # HxZip int with the compressed size.
    iv = [x * 100000 - y * 7 + z * 3 for z in range(4) for y in range(H) for x in range(W)]
    raw = struct.pack(f"<{len(iv)}i", *iv)
    z = zlib.compress(raw, 9)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 4\nParameters {\n    CoordType \"uniform\"\n}\n"
            b"Lattice { int Data } @1(HxZip,%d)\n# Data section follows\n@1\n" % len(z))
    (OUT / "zip_int.am").write_bytes(head + z + b"\n")
    truth.append(("zip_int.am", zlib.crc32(raw)))

    # HxByteRLE byte: runs of the background and literal gradients.
    b = bytes((0 if (x + y + z) % 4 else 200) if x < 4 else (x * 31 + y * 7 + z) % 256
              for z in range(3) for y in range(H) for x in range(W))
    r = rle_slices(b, W * H)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 3\nParameters {\n    BoundingBox 0 6 0 4 0 2,\n"
            b"    CoordType \"uniform\"\n}\nLattice { byte Data } @1(HxByteRLE,%d)\n# Data section follows\n@1\n" % len(r))
    (OUT / "rle_byte.am").write_bytes(head + r + b"\n")
    truth.append(("rle_byte.am", zlib.crc32(b)))
    one = b[:W * H]
    r1 = rle(one)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 1\nParameters {\n    CoordType \"uniform\"\n}\n"
            b"Lattice { byte Data } @1(HxByteRLE,%d)\n# Data section follows\n@1\n" % len(r1))
    (OUT / "rle_slice.am").write_bytes(head + r1 + b"\n")
    truth.append(("rle_slice.am", zlib.crc32(one)))

    # Avizo header, HxZip without size, nested groups, CRLF.
    bz = bytes((x * 40 + y * 9 + z * 3) % 256 for z in range(2) for y in range(H) for x in range(W))
    head = ("# Avizo 3D BINARY 2.0\r\n# CreationDate: Thu Nov  5 12:00:00 2026\r\n\r\ndefine Lattice 7 5 2\r\n\r\n"
            "Parameters {\r\n    Inner {\r\n        BoundingBox 0 70 0 50 0 20,\r\n        Note \"braces { in } a string\",\r\n"
            "        &Note \"AAAAAQAAAAYAQZJXAxOxQSA=\"\r\n    }\r\n    Content \"7x5x2 byte, uniform coordinates\",\r\n"
            "    BoundingBox 0 3 0 2 0 1,\r\n    CoordType \"uniform\"\r\n}\r\n\r\n"
            "Lattice { byte Data } @1(HxZip,0)\r\n\r\n# Data section follows\r\n@1\r\n").encode()
    (OUT / "zip_nosize.am").write_bytes(head + zlib.compress(bz) + b"\r\n")
    truth.append(("zip_nosize.am", zlib.crc32(bz)))

    # Label field with two byte fields.
    labels = bytes((0 if x < 3 else 4) for z in range(2) for y in range(H) for x in range(W))
    prob = bytes((255 if x < 3 else 100 + y * 20 + x) for z in range(2) for y in range(H) for x in range(W))
    rp = rle_slices(prob, W * H)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 2\nParameters {\n    Materials {\n"
            b"        Exterior {\n            Id 0\n        }\n        Something {\n            Id 4,\n"
            b"            Color 0.8 0.3 0.1\n        }\n    }\n    BoundingBox 0 6 0 4 0 1,\n    CoordType \"uniform\"\n}\n"
            b"Lattice { byte Labels } @1\nLattice { byte Probability } @2(HxByteRLE,%d)\n"
            b"# Data section follows\n@1\n" % len(rp))
    (OUT / "labels.am").write_bytes(head + labels + b"\n@2\n" + rp + b"\n")
    truth.append(("labels.am", zlib.crc32(bytes(v for pair in zip(labels, prob) for v in pair))))

    # Stacked double with a z coordinate block stored first.
    d = [x * 1.5 + y * 0.125 - z * 7 for z in range(3) for y in range(H) for x in range(W)]
    zs = struct.pack("<3f", 0.0, 0.75, 2.0)
    head = (b"# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 7 5 3\ndefine Coordinates 3\nParameters {\n"
            b"    CoordType \"stacked\",\n    BoundingBoxXY 0 1.5 0 1\n}\nLattice { double Values } @1\n"
            b"Coordinates { float z } @2\n# Data section follows\n@2\n")
    raw = struct.pack(f"<{len(d)}d", *d)
    (OUT / "stacked_double.am").write_bytes(head + zs + b"\n@1\n" + raw + b"\n")
    truth.append(("stacked_double.am", zlib.crc32(raw)))

    with open(OUT / "truth.tsv", "w") as out:
        out.write("file\tcrc32\n")
        for name, crc in truth:
            out.write(f"{name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

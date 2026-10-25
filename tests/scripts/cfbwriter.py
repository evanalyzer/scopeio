# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Minimal compound file (OLE2) writer for the synthetic samples.

Follows Microsoft [MS-CFB] version 3: 512-byte sectors, streams below 4096
bytes in the mini stream, one storage. Used by make_oif_samples.py and
make_zvi_samples.py.
"""

import struct

SECTOR, MINI, CUTOFF = 512, 64, 4096
FREE, END, FATSECT, NOSTREAM = 0xFFFFFFFF, 0xFFFFFFFE, 0xFFFFFFFD, 0xFFFFFFFF


def cfb(root_streams, storage_name, storage_streams):
    """root_streams / storage_streams: [(name, bytes)]; one storage."""
    return cfb_tree(dict(root_streams) | {storage_name: dict(storage_streams)})


def cfb_tree(tree):
    """`tree`: {name: bytes (stream) or dict (storage)}, insertion-ordered."""
    entries = [["Root Entry", 5, None, NOSTREAM, NOSTREAM]]  # name, type, data, right, child

    def add(children):
        ids = []
        for name, value in children.items():
            i = len(entries)
            entries.append([name, 1 if isinstance(value, dict) else 2, value, NOSTREAM, NOSTREAM])
            ids.append(i)
            if isinstance(value, dict) and value:
                entries[i][4] = add(value)
        # Siblings as a right-leaning chain (all black), in the directory
        # order of [MS-CFB] 2.6.4: shorter names first, then upper-case code units.
        ids.sort(key=lambda i: (len(entries[i][0]), entries[i][0].upper()))
        for a, b in zip(ids, ids[1:]):
            entries[a][3] = b
        return ids[0] if ids else NOSTREAM

    entries[0][4] = add(tree)
    sectors = []  # regular sector payloads
    fat = []

    def put_chain(data):
        start = len(sectors)
        n = max(1, -(-len(data) // SECTOR))
        for i in range(n):
            sectors.append(data[i * SECTOR:(i + 1) * SECTOR].ljust(SECTOR, b"\0"))
            fat.append(start + i + 1 if i + 1 < n else END)
        return start

    # Streams: large ones in sectors, small ones in the mini stream.
    mini = bytearray()
    minifat = []
    starts = {}
    for i, (_, typ, data, _, _) in enumerate(entries):
        if typ != 2:
            continue
        if len(data) >= CUTOFF:
            starts[i] = put_chain(data)
        else:
            start = len(mini) // MINI
            n = max(1, -(-len(data) // MINI))
            mini += data.ljust(n * MINI, b"\0")
            minifat += [start + k + 1 if k + 1 < n else END for k in range(n)]
            starts[i] = start
    mini_start = put_chain(bytes(mini)) if mini else END
    minifat_bytes = b"".join(struct.pack("<I", v) for v in minifat)
    minifat_start = put_chain(minifat_bytes.ljust(-(-len(minifat_bytes) // SECTOR) * SECTOR, b"\xff")) if minifat else END

    def dirent(i):
        name, typ, data, right, child = entries[i]
        n = (name + "\0").encode("utf-16-le")
        if i == 0:
            start, size = mini_start, len(mini)
        elif typ == 2:
            start, size = starts[i], len(data)
        else:
            start, size = 0, 0
        return (n.ljust(64, b"\0") + struct.pack("<HBB", len(n), typ, 1) + struct.pack("<III", NOSTREAM, right, child)
                + bytes(16) + bytes(4) + bytes(16) + struct.pack("<IQ", start, size))
    directory = b"".join(dirent(i) for i in range(len(entries)))
    directory = directory.ljust(-(-len(directory) // SECTOR) * SECTOR, b"\0")
    dir_start = put_chain(directory)
    # FAT sectors at the end, covering themselves.
    n_fat = 1
    while len(sectors) + n_fat > n_fat * (SECTOR // 4):
        n_fat += 1
    fat_ids = list(range(len(sectors), len(sectors) + n_fat))
    fat += [FATSECT] * n_fat
    fat += [FREE] * (n_fat * (SECTOR // 4) - len(fat))
    for k in range(n_fat):
        sectors.append(b"".join(struct.pack("<I", v) for v in fat[k * 128:(k + 1) * 128]))
    assert n_fat <= 109
    n_minifat = -(-len(minifat) * 4 // SECTOR)
    header = (b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + bytes(16) + struct.pack("<HHHHH", 0x3E, 3, 0xFFFE, 9, 6) + bytes(6)
              + struct.pack("<IIIIIIIII", 0, n_fat, dir_start, 0, CUTOFF, minifat_start, n_minifat, END, 0))
    difat = [fat_ids[k] if k < n_fat else FREE for k in range(109)]
    header += b"".join(struct.pack("<I", v) for v in difat)
    assert len(header) == 512, len(header)
    return header + b"".join(sectors)

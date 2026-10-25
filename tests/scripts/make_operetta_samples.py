#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Harmony export (Operetta / Opera Phenix).

Layout from the public exports on the OME sample server: `Index.idx.xml`
(plates, wells, one `Image` entry per plane: URL, Row, Col, FieldID,
PlaneID, TimepointID, ChannelID, resolution, positions, channel settings)
and one uncompressed 16-bit TIFF per plane. As in older exports, the URLs
point to the acquisition server (`http://server/ODA/Images/<guid>/<file>`)
and the files sit in `<guid>/` next to the measurement folder. 2 wells x
2 fields, 2 channels x 3 planes x 2 timepoints of 10 x 8; plane
(well C03, field 1, channel 2, plane 3, timepoint 1) has no file and reads
as zeros. Also writes truth.tsv with the CRC-32 of each series in the
layout of tests/common/mod.rs `decode_level()`. Output in
tests/data/operetta/ is committed.

Usage: python3 -I tests/scripts/make_operetta_samples.py
"""

import shutil
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "data" / "operetta"
GUID = "0a1b2c3d-0000-4000-8000-123456789abc"
W, H, C, Z, T = 10, 8, 2, 3, 2
WELLS = [(3, 3), (3, 4)]
FIELDS = [1, 2]
MISSING = ((3, 3), 1, 2, 3, 1)  # well, field, channel, plane, timepoint (1-based ids)


def value(x, y, well, field, c, z, t):
    return (x * 37 + y * 101 + well * 5000 + field * 900 + c * 3000 + z * 333 + t * 7000) % 65536


def tiff16(data):
    entries = [(256, 3, 1, W), (257, 3, 1, H), (258, 3, 1, 16), (259, 3, 1, 1), (262, 3, 1, 1),
               (273, 4, 1, 8), (277, 3, 1, 1), (278, 3, 1, H), (279, 4, 1, len(data))]
    ifd = struct.pack("<H", len(entries)) + b"".join(struct.pack("<HHII", *e) for e in entries) + struct.pack("<I", 0)
    return b"II*\0" + struct.pack("<I", 8 + len(data)) + data + ifd


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    images_dir = OUT / "Plate1__2024-01-01T00_00_00-Measurement 1" / "Images"
    images_dir.mkdir(parents=True)
    (OUT / GUID).mkdir()
    entries = []
    truth = []
    for wi, (row, col) in enumerate(WELLS):
        for field in FIELDS:
            px = bytearray()
            for t in range(1, T + 1):
                for z in range(1, Z + 1):
                    planes = {}
                    for c in range(1, C + 1):
                        name = f"r{row:02d}c{col:02d}f{field:02d}p{z:02d}-ch{c}sk{t}fk1fl1.tiff"
                        data = b"".join(struct.pack("<H", value(x, y, wi, field, c, z, t)) for y in range(H) for x in range(W))
                        missing = ((row, col), field, c, z, t) == MISSING
                        if not missing:
                            (OUT / GUID / name).write_bytes(tiff16(data))
                        planes[c] = None if missing else data
                        entries.append(
                            f"<Image Version=\"1\"><id>{row:02d}{col:02d}K{t}F{field}P{z}R{c}</id><State>Ok</State>"
                            f"<URL>http://server/ODA/Images/{GUID}/{name}</URL><Row>{row}</Row><Col>{col}</Col>"
                            f"<FieldID>{field}</FieldID><PlaneID>{z}</PlaneID><TimepointID>{t}</TimepointID><ChannelID>{c}</ChannelID>"
                            f"<ChannelName>{['DAPI', 'Alexa 488'][c - 1]}</ChannelName><ChannelType>Fluorescence</ChannelType>"
                            f"<ImageResolutionX Unit=\"m\">6.5E-07</ImageResolutionX><ImageResolutionY Unit=\"m\">6.5E-07</ImageResolutionY>"
                            f"<ImageSizeX>{W}</ImageSizeX><ImageSizeY>{H}</ImageSizeY>"
                            f"<PositionX Unit=\"m\">{field * 1e-4}</PositionX><PositionY Unit=\"m\">0</PositionY>"
                            f"<PositionZ Unit=\"m\">{z * 2e-6}</PositionZ><MeasurementTimeOffset Unit=\"s\">{(t - 1) * 30.0}</MeasurementTimeOffset>"
                            f"<MainExcitationWavelength Unit=\"nm\">{[375, 488][c - 1]}</MainExcitationWavelength>"
                            f"<MainEmissionWavelength Unit=\"nm\">{[456, 525][c - 1]}</MainEmissionWavelength>"
                            f"<ObjectiveMagnification Unit=\"\">20</ObjectiveMagnification><ObjectiveNA Unit=\"\">0.4</ObjectiveNA>"
                            f"<ExposureTime Unit=\"s\">0.05</ExposureTime></Image>"
                        )
                    # decode_level layout: channels interleaved per pixel.
                    for i in range(W * H):
                        for c in range(1, C + 1):
                            px += planes[c][2 * i:2 * i + 2] if planes[c] else b"\0\0"
            truth.append((f"well {chr(64 + row)}{col:02d} field {field}", zlib.crc32(bytes(px))))
    wells = "".join(f'<Well><id>{r:02d}{c:02d}</id><Row>{r}</Row><Col>{c}</Col></Well>' for r, c in WELLS)
    xml = ('﻿<?xml version="1.0" encoding="utf-8"?><EvaluationInputData Version="1" xmlns="http://www.perkinelmer.com/PEHH/HarmonyV5">'
           '<Plates><Plate><PlateID>Plate1</PlateID><MeasurementStartTime>2024-01-01T00:00:00+00:00</MeasurementStartTime>'
           f'<Name>Plate1</Name><PlateRows>8</PlateRows><PlateColumns>12</PlateColumns></Plate></Plates><Wells>{wells}</Wells>'
           f'<Images>{"".join(entries)}</Images></EvaluationInputData>')
    (images_dir / "Index.idx.xml").write_text(xml, encoding="utf-8")
    with open(OUT / "truth.tsv", "w") as f:
        f.write("series\tname\tcrc32\n")
        for i, (name, crc) in enumerate(truth):
            f.write(f"{i}\tPlate1 {name}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

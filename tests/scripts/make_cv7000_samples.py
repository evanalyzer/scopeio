#!/usr/bin/env python3
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
"""Writes a small synthetic Yokogawa CV7000 measurement.

Layout from the public CV7000 / CV8000 plates (idr0088, idr0093,
cpg0016): plate file (.wpi), MeasurementDetail.mrf, measurement setting
(.mes), MeasurementData.mlf with one MeasurementRecord per TIFF file.

plate/: 8 x 12 plate, 2 fields, 2 time points. Action 1 takes channels 1
and 2 in 2-D, action 2 a 3-plane brightfield stack of channel 3 (z step
2 µm). Wells B03 and C05; C05 field 2 has only an error record (blank
series); D01 is listed but none of its files exist (no series); the
record of B03 field 2, t 2, z 2 names a missing file (zeros).

Also writes truth.tsv with the CRC-32 of each series in the layout of
tests/common/mod.rs `decode_level()` (2-D channels repeated at every z).
Output in tests/data/cv7000/ is committed.

Usage: python3 -I tests/scripts/make_cv7000_samples.py
"""

import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_micromanager_samples import W, H, plain_tiff  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "data" / "cv7000"
NS = 'xmlns:bts="http://www.yokogawa.co.jp/BTS/BTSSchema/1.0"'
# (action, channel number, z planes)
CHANNELS = [(1, 1, 1), (1, 2, 1), (2, 3, 3)]
WELLS = {"B03": (2, 3), "C05": (3, 5), "D01": (4, 1)}
MISSING = ("B03", 2, 2, 2, 3)  # well, field, t, z, channel


def value(x, y, c, z, t, s):
    return (x * 1000 + y * 77 + c * 5000 + z * 300 + t * 11 + s * 2000) % 65536


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    d = OUT / "plate"
    d.mkdir(parents=True)
    (d / "synth.wpi").write_text(f'''﻿<?xml version="1.0" encoding="utf-8"?>
<bts:WellPlate bts:Version="1.0" bts:Name="synth" bts:Columns="12" bts:Rows="8" {NS}>
  <bts:Description>Synthetic</bts:Description>
</bts:WellPlate>
''', encoding="utf-8")
    chans = "".join(f'''
  <bts:MeasurementChannel bts:Ch="{ch}" bts:HorizontalPixelDimension="0.65" bts:VerticalPixelDimension="0.65" bts:CameraNumber="1" bts:InputBitDepth="16" bts:InputLevel="1000" bts:HorizontalPixels="{W}" bts:VerticalPixels="{H}" bts:FilterWheelPosition="1" bts:FilterPosition="1" bts:ShadingCorrectionSource="" />'''
                    for _, ch, _ in CHANNELS)
    (d / "MeasurementDetail.mrf").write_text(f'''﻿<?xml version="1.0" encoding="utf-8"?>
<bts:MeasurementDetail bts:Version="1.0" bts:Title="synth" bts:OperatorName="synth" bts:Application="" bts:BeginTime="2026-11-04T10:11:12.5+01:00" bts:EndTime="2026-11-04T10:30:00.0+01:00" bts:MeasurementSettingFileName="synth.mes" bts:ColumnCount="12" bts:RowCount="8" bts:TimePointCount="2" bts:FieldCount="2" bts:ZCount="3" bts:TargetSystem="CV7000" {NS}>
  <bts:MeasurementSamplePlate bts:Name="synth" bts:WellPlateFileName="synth.wpi" />{chans}
</bts:MeasurementDetail>
''', encoding="utf-8")
    (d / "synth.mes").write_text(f'''﻿<?xml version="1.0" encoding="utf-8"?>
<bts:MeasurementSetting bts:Version="1.0" bts:Columns="12" bts:Rows="8" {NS}>
  <bts:Timelapse>
    <bts:Timeline bts:Name="Time Line 1" bts:InitialTime="0" bts:Period="60" bts:Interval="60">
      <bts:ActionList bts:RunMode="sequential">
        <bts:ActionAcquire bts:XOffset="0" bts:YOffset="0" bts:ZOffset="-3">
          <bts:Ch>1</bts:Ch>
          <bts:Ch>2</bts:Ch>
        </bts:ActionAcquire>
        <bts:ActionAcquireBF3D bts:XOffset="0" bts:YOffset="0" bts:TopDistance="2" bts:BottomDistance="-2" bts:SliceLength="2">
          <bts:Ch>3</bts:Ch>
        </bts:ActionAcquireBF3D>
      </bts:ActionList>
    </bts:Timeline>
  </bts:Timelapse>
  <bts:LightSourceList>
    <bts:LightSource bts:Name="405nm" bts:Type="Laser" bts:WaveLength="405" bts:Power="100" />
    <bts:LightSource bts:Name="488nm" bts:Type="Laser" bts:WaveLength="488" bts:Power="100" />
    <bts:LightSource bts:Name="Lamp" bts:Type="Lamp" bts:WaveLength="0" bts:Power="30" />
  </bts:LightSourceList>
  <bts:ChannelList>
    <bts:Channel bts:Ch="1" bts:Target="DNA" bts:ObjectiveID="20000" bts:Objective="20x" bts:Magnification="20" bts:MethodID="1" bts:Method="Fluorescence" bts:FilterID="1" bts:Fluorophore="" bts:CameraType="AndorZyla" bts:InputLevel="1000" bts:MinLevel="0" bts:MaxLevel="1" bts:Acquisition="BP445/45" bts:ExposureTime="100" bts:Binning="2" bts:Color="#FF002FFF" bts:PinholeDiameter="50" bts:Kind="ConfocalFluorescence">
      <bts:LightSourceName>405nm</bts:LightSourceName>
    </bts:Channel>
    <bts:Channel bts:Ch="2" bts:Target="ER" bts:ObjectiveID="20000" bts:Objective="20x" bts:Magnification="20" bts:MethodID="1" bts:Method="Fluorescence" bts:FilterID="1" bts:Fluorophore="" bts:CameraType="AndorZyla" bts:InputLevel="1000" bts:MinLevel="0" bts:MaxLevel="1" bts:Acquisition="BP525/50" bts:ExposureTime="50" bts:Binning="2" bts:Color="#FF00FFA1" bts:Kind="ConfocalFluorescence">
      <bts:LightSourceName>488nm</bts:LightSourceName>
    </bts:Channel>
    <bts:Channel bts:Ch="3" bts:Target="BF" bts:ObjectiveID="20000" bts:Objective="20x" bts:Magnification="20" bts:MethodID="1" bts:Method="Fluorescence" bts:FilterID="1" bts:Fluorophore="" bts:CameraType="AndorZyla" bts:InputLevel="1000" bts:MinLevel="0" bts:MaxLevel="1" bts:Acquisition="BP525/50" bts:ExposureTime="20" bts:Binning="2" bts:Color="#FFFFFFFF" bts:Kind="Brightfield">
      <bts:LightSourceName>Lamp</bts:LightSourceName>
    </bts:Channel>
  </bts:ChannelList>
</bts:MeasurementSetting>
''', encoding="utf-8")

    records, truth = [], []
    series = 0
    for well, (row, col) in WELLS.items():
        for field in (1, 2):
            if well == "C05" and field == 2:
                records.append(f'<bts:MeasurementRecord bts:Type="ERR" bts:Time="2026-11-04T10:20:00.000+01:00" bts:Column="{col}" bts:Row="{row}" bts:TimePoint="1" bts:FieldIndex="2" bts:TimelineIndex="1" bts:X="0" bts:Y="0">AF Error at {well} No. 2</bts:MeasurementRecord>')
                if well != "D01":
                    truth.append((series, zlib.crc32(bytes(W * H * 3 * 2 * 3 * 2))))
                    series += 1
                continue
            planes = {}
            for t in (1, 2):
                for c, (action, ch, zs) in enumerate(CHANNELS):
                    for z in range(1, zs + 1):
                        name = f"synth_{well}_T{t:04d}F{field:03d}L01A{action:02d}Z{z:02d}C{ch:02d}.tif"
                        sec = 10 + field * 5 + t * 60 + c + z * 0.25
                        records.append(f'<bts:MeasurementRecord bts:Type="IMG" bts:Time="2026-11-04T10:{int(sec // 60):02d}:{sec % 60:06.3f}+01:00" bts:Column="{col}" bts:Row="{row}" bts:TimePoint="{t}" bts:FieldIndex="{field}" bts:ZIndex="{z}" bts:TimelineIndex="1" bts:ActionIndex="{action}" bts:Action="{"2D" if zs == 1 else "BF3D"}" bts:X="{100.0 * field}" bts:Y="-50.0" bts:Z="{-3.0 + 2.0 * (z - 1)}" bts:Ch="{ch}">{name}</bts:MeasurementRecord>')
                        if well == "D01" or (well, field, t, z, ch) == MISSING:
                            continue
                        px = [value(x, y, c, z, t, series) for y in range(H) for x in range(W)]
                        (d / name).write_bytes(plain_tiff(px, "H", 16))
                        planes[(c, z, t)] = px
            if well == "D01":
                continue
            out = bytearray()
            for t in (1, 2):
                for z in (1, 2, 3):
                    for i in range(W * H):
                        for c, (_, _, zs) in enumerate(CHANNELS):
                            p = planes.get((c, 1 if zs == 1 else z, t))
                            out += struct.pack("<H", p[i] if p else 0)
            truth.append((series, zlib.crc32(out)))
            series += 1
    (d / "MeasurementData.mlf").write_text('<?xml version="1.0" encoding="utf-8"?>\n'
                                           f'<bts:MeasurementData bts:Version="1.0" {NS}>\n'
                                           + "\n".join(records) + "\n</bts:MeasurementData>\n", encoding="utf-8")
    with open(OUT / "truth.tsv", "w") as f:
        f.write("file\tseries\tcrc32\n")
        for s, crc in truth:
            f.write(f"plate/synth.wpi\t{s}\t{crc:08x}\n")


if __name__ == "__main__":
    main()

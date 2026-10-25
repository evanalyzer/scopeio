//! # zvi
//!
//! Zeiss AxioVision ZVI reader.
//!
//! A ZVI file is a compound file. Each plane is a stream
//! `Image/Item(n)/Contents`: typed values, then a raw-image header (marker
//! `0x10002000`, width, height, depth, bytes per pixel, pixel format, valid
//! bits) and the samples; `Image/Item(n)/Tags/Contents` holds its z, channel
//! and time indices and channel settings, `Image/Tags/Contents` the image
//! settings (scale, objective, date). Typed values carry OLE Automation
//! `VARENUM` type codes ([MS-OAUT]). Distinct stored indices, sorted, become
//! the z, c and t positions; items differing in their further position
//! indices form separate series. Colour samples are stored blue first and
//! returned as RGB. Planes are read by positioned reads of their streams.
//!
//! Provenance: OpenReadout ZVI notes (MIT OR Apache-2.0) in
//! `doc/zeiss-zvi/`; Microsoft [MS-CFB] and [MS-OAUT]; own analysis of
//! public sample files (Figshare, CC BY 4.0 / CC0); checked against
//! Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::cfb::{self, Cfb};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, PixelType, Plane,
    PlaneMeta, Region, Result, SeriesMeta, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Zvi, "Zeiss AxioVision ZVI", "1").extensions(&["zvi"]);

const IMAGE_MARKER: u32 = 0x1000_2000;
const IMAGE_HEADER_LEN: u64 = 28;
/// Where the raw-image header is looked for.
const HEADER_SEARCH: u64 = 64 << 10;
const TAG_Z_INDEX: i64 = 2819;
const TAG_C_INDEX: i64 = 2820;
const TAG_T_INDEX: i64 = 2821;
const POSITION_TAGS: [i64; 3] = [2822, 2823, 2827];
const TAG_CHANNEL_NAME: i64 = 1284;
const TAG_CHANNEL_COLOR: i64 = 1282;
const TAG_EXPOSURE_MS: i64 = 2564;
const TAG_EXCITATION_NM: i64 = 0x100_0110;
const TAG_EMISSION_NM: i64 = 0x100_0111;
const TAG_SCALE: [(i64, i64); 3] = [(769, 770), (772, 773), (775, 776)];
const UNIT_MICROMETRE: i64 = 76;
const TAG_ACQUIRED: i64 = 1025;
const TAG_RELATIVE_TIME: i64 = 300;
const TAG_OBJECTIVE_NAME: i64 = 2049;
const TAG_OBJECTIVE_MAGNIFICATION: i64 = 2076;
const TAG_OBJECTIVE_NA: i64 = 2077;
const TAG_FILE_NAME: i64 = 1553;

/// Returns `true` for a compound file with ZVI image items.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    cfb::matches(head) && Cfb::open(path).is_ok_and(|c| c.stream("Image/Item(0)/Contents").is_some())
}

/// A typed value.
#[derive(Debug, Clone, PartialEq)]
enum TagValue {
    Empty,
    Int(i64),
    Float(f64),
    Text(String),
    Bytes,
}

impl TagValue {
    fn f64(&self) -> Option<f64> {
        match self {
            TagValue::Int(v) => Some(*v as f64),
            TagValue::Float(v) => Some(*v),
            _ => None,
        }
    }
}

/// Reads one typed value at `*pos`; `None` when truncated or unknown.
fn read_value(b: &[u8], pos: &mut usize) -> Option<TagValue> {
    let code = u16::from_le_bytes(b.get(*pos..*pos + 2)?.try_into().ok()?);
    *pos += 2;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = b.get(*pos..*pos + n)?;
        *pos += n;
        Some(s)
    };
    let int = |s: &[u8]| -> i64 {
        match s.len() {
            1 => s[0] as i8 as i64,
            2 => i16::from_le_bytes([s[0], s[1]]) as i64,
            4 => i32::from_le_bytes(s.try_into().expect("4 bytes")) as i64,
            _ => i64::from_le_bytes(s.try_into().expect("8 bytes")),
        }
    };
    Some(match code {
        0 | 1 => TagValue::Empty,
        2 | 11 => TagValue::Int(int(take(2)?)),
        3 | 22 => TagValue::Int(int(take(4)?)),
        16 => TagValue::Int(int(take(1)?)),
        17 => TagValue::Int(take(1)?[0] as i64),
        18 => TagValue::Int(u16::from_le_bytes(take(2)?.try_into().ok()?) as i64),
        19 | 23 => TagValue::Int(u32::from_le_bytes(take(4)?.try_into().ok()?) as i64),
        20 | 21 => TagValue::Int(int(take(8)?)),
        4 => TagValue::Float(f32::from_le_bytes(take(4)?.try_into().ok()?) as f64),
        5 | 7 => TagValue::Float(f64::from_le_bytes(take(8)?.try_into().ok()?)),
        6 | 64 => TagValue::Int(int(take(8)?)),
        10 => TagValue::Int(int(take(4)?)),
        9 | 13 | 14 | 72 => {
            take(16)?;
            TagValue::Bytes
        }
        8 => {
            let n = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            let s = take(n)?;
            let units: Vec<u16> = s.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            TagValue::Text(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string())
        }
        30 | 31 | 65..=71 => {
            let n = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            take(n)?;
            TagValue::Bytes
        }
        _ => return None,
    })
}

/// A tag list: (id, value) pairs.
fn parse_tags(b: &[u8]) -> Vec<(i64, TagValue)> {
    let mut pos = 0;
    let mut out = Vec::new();
    // Version, count, then (value, id, attribute) entries.
    let (Some(_), Some(TagValue::Int(count))) = (read_value(b, &mut pos), read_value(b, &mut pos)) else { return out };
    for _ in 0..count.clamp(0, 1 << 16) {
        let (Some(v), Some(TagValue::Int(id)), Some(_)) = (read_value(b, &mut pos), read_value(b, &mut pos), read_value(b, &mut pos)) else { break };
        out.push((id, v));
    }
    out
}

fn tag(tags: &[(i64, TagValue)], id: i64) -> Option<&TagValue> {
    tags.iter().find(|(i, _)| *i == id).map(|(_, v)| v)
}

/// ISO-8601 local time of an OLE date (days since 1899-12-30).
fn ole_date(days: f64) -> Option<String> {
    if !(1.0..2_958_465.0).contains(&days) {
        return None;
    }
    let secs = ((days - 25569.0) * 86400.0).round() as i64;
    let (d, s) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = d + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some(format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60))
}

/// One plane stream.
#[derive(Debug)]
struct Item {
    sectors: Option<Vec<u32>>,
    stream: usize,
    data: u64,
    width: u32,
    height: u32,
    bpp: u32,
    tags: Vec<(i64, TagValue)>,
}

/// One series: planes by `c + size_c * (z + size_z * t)` (item index).
#[derive(Debug)]
struct Series {
    planes: Vec<Option<usize>>,
    times: Vec<Option<f64>>,
}

/// Zeiss AxioVision ZVI file.
#[derive(Debug)]
pub(super) struct Zvi {
    cfb: Cfb,
    items: Vec<Item>,
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Zvi {
    /// Opens a ZVI file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for broken containers or items and
    /// [`Error::UnsupportedVariant`] for sample layouts not handled.
    pub fn open(path: &Path) -> Result<Self> {
        let cfb = Cfb::open(path)?;
        let read_small = |name: &str| -> Option<Vec<u8>> {
            let s = cfb.stream(name)?;
            (s.size < 16 << 20).then(|| cfb.read(s).ok()).flatten()
        };
        let image_tags = read_small("Image/Tags/Contents").map(|b| parse_tags(&b)).unwrap_or_default();
        let mut items = Vec::new();
        for n in 0..1 << 20 {
            let name = format!("Image/Item({n})/Contents");
            let Some(index) = cfb.streams().iter().position(|s| s.path.eq_ignore_ascii_case(&name)) else { break };
            let stream = &cfb.streams()[index];
            let sectors = cfb.sectors(stream)?;
            let head_len = stream.size.min(HEADER_SEARCH) as usize;
            let head = match &sectors {
                Some(ids) => {
                    let mut b = vec![0u8; head_len];
                    cfb.read_at(ids, 0, &mut b)?;
                    b
                }
                None => cfb.read(stream)?,
            };
            // The raw-image header whose samples fit the stream.
            let found = (0..head.len().saturating_sub(IMAGE_HEADER_LEN as usize)).step_by(1).find_map(|at| {
                let u = |i: usize| u32::from_le_bytes(head[at + 4 * i..at + 4 * i + 4].try_into().expect("4 bytes"));
                if u(0) != IMAGE_MARKER {
                    return None;
                }
                let (w, h, bpp) = (u(1), u(2), u(4));
                let data = at as u64 + IMAGE_HEADER_LEN;
                let need = w as u64 * h as u64 * bpp as u64;
                (w > 0 && h > 0 && bpp > 0 && data + need <= stream.size).then_some((data, w, h, bpp))
            });
            let Some((data, width, height, bpp)) = found else {
                return Err(Error::Corrupt(format!("ZVI item {n} without raw-image header")));
            };
            let tags = read_small(&format!("Image/Item({n})/Tags/Contents")).map(|b| parse_tags(&b)).unwrap_or_default();
            items.push(Item { sectors, stream: index, data, width, height, bpp, tags });
        }
        if items.is_empty() {
            return Err(Error::Corrupt("ZVI without image items".into()));
        }
        let index = |it: &Item, id: i64| tag(&it.tags, id).and_then(TagValue::f64).map_or(0, |v| v as i64);
        // Series by further position indices.
        let mut groups: BTreeMap<[i64; 3], Vec<usize>> = BTreeMap::new();
        for (i, it) in items.iter().enumerate() {
            groups.entry(POSITION_TAGS.map(|t| index(it, t))).or_default().push(i);
        }
        let image_f64 = |id: i64| tag(&image_tags, id).and_then(TagValue::f64);
        let image_text = |id: i64| match tag(&image_tags, id) {
            Some(TagValue::Text(t)) if !t.is_empty() => Some(t.clone()),
            _ => None,
        };
        let size = |(scale, unit): (i64, i64)| -> Option<f64> {
            (image_f64(unit)? as i64 == UNIT_MICROMETRE).then(|| image_f64(scale)).flatten().filter(|v| *v > 0.0).map(|v| v * 1e3)
        };
        let mut series = Vec::new();
        let mut metas = Vec::new();
        for list in groups.into_values() {
            let first = &items[list[0]];
            if list.iter().any(|&i| (items[i].width, items[i].height, items[i].bpp) != (first.width, first.height, first.bpp)) {
                return Err(Error::UnsupportedVariant("ZVI items of different sizes".into()));
            }
            let (pixel_type, spp) = match first.bpp {
                1 => (PixelType::U8, 1),
                2 => (PixelType::U16, 1),
                3 => (PixelType::U8, 3),
                6 => (PixelType::U16, 3),
                b => return Err(Error::UnsupportedVariant(format!("ZVI with {b} bytes per pixel"))),
            };
            let distinct = |id: i64| -> Vec<i64> { list.iter().map(|&i| index(&items[i], id)).collect::<BTreeSet<_>>().into_iter().collect() };
            let (zs, cs, ts) = (distinct(TAG_Z_INDEX), distinct(TAG_C_INDEX), distinct(TAG_T_INDEX));
            let (size_c, size_z, size_t) = (cs.len(), zs.len(), ts.len());
            let mut planes = vec![None; size_c * size_z * size_t];
            let mut times = vec![None; planes.len()];
            for &i in &list {
                let it = &items[i];
                let pos = |v: &[i64], id: i64| v.iter().position(|x| *x == index(it, id)).expect("distinct value");
                let p = pos(&cs, TAG_C_INDEX) + size_c * (pos(&zs, TAG_Z_INDEX) + size_z * pos(&ts, TAG_T_INDEX));
                planes[p].get_or_insert(i);
                times[p] = tag(&it.tags, TAG_RELATIVE_TIME).and_then(TagValue::f64).map(|d| d * 86400.0);
            }
            // Channels by stored channel index; the image tags fill gaps.
            let channels: Vec<ChannelMeta> = cs
                .iter()
                .map(|&c| {
                    let it = list.iter().map(|&i| &items[i]).find(|it| index(it, TAG_C_INDEX) == c).expect("channel item");
                    let f = |id: i64| tag(&it.tags, id).and_then(TagValue::f64).or_else(|| image_f64(id));
                    let name = match tag(&it.tags, TAG_CHANNEL_NAME) {
                        Some(TagValue::Text(t)) if !t.is_empty() => Some(t.clone()),
                        _ => image_text(TAG_CHANNEL_NAME),
                    };
                    let color = f(TAG_CHANNEL_COLOR).map(|v| {
                        let v = v as u32;
                        [(v & 0xFF) as u8, (v >> 8 & 0xFF) as u8, (v >> 16 & 0xFF) as u8, 255]
                    });
                    ChannelMeta {
                        name,
                        color: if spp == 1 { color } else { None },
                        exposure_s: f(TAG_EXPOSURE_MS).filter(|v| *v > 0.0).map(|v| v / 1e3),
                        excitation_wavelength_nm: f(TAG_EXCITATION_NM).filter(|v| *v > 0.0),
                        emission_wavelength_nm: f(TAG_EMISSION_NM).filter(|v| *v > 0.0),
                        ..Default::default()
                    }
                })
                .collect();
            let time_increment_s = (size_t > 1)
                .then(|| {
                    let at = |t: usize| times.get(size_c * size_z * t).copied().flatten();
                    Some((at(size_t - 1)? - at(0)?) / (size_t - 1) as f64).filter(|v| *v > 0.0)
                })
                .flatten();
            let date = tag(&first.tags, TAG_ACQUIRED).or_else(|| tag(&image_tags, TAG_ACQUIRED)).and_then(TagValue::f64).and_then(ole_date);
            let objective = ObjectiveMeta {
                model: image_text(TAG_OBJECTIVE_NAME),
                magnification: image_f64(TAG_OBJECTIVE_MAGNIFICATION).filter(|v| *v > 0.0),
                numerical_aperture: image_f64(TAG_OBJECTIVE_NA).filter(|v| *v > 0.0),
                ..Default::default()
            };
            metas.push(SeriesMeta {
                name: image_text(TAG_FILE_NAME),
                acquisition_date: date,
                size_c: if spp == 3 { 1 } else { size_c as u32 },
                size_z: size_z as u32,
                size_t: size_t as u32,
                pixel_type,
                samples_per_pixel: spp,
                levels: vec![LevelMeta { size_x: first.width as u64, size_y: first.height as u64, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: size(TAG_SCALE[0]), y_nm: size(TAG_SCALE[1]), z_nm: size(TAG_SCALE[2]).filter(|_| size_z > 1) },
                time_increment_s,
                channels: if spp == 3 { vec![ChannelMeta { color: None, ..channels.into_iter().next().unwrap_or_default() }] } else { channels },
                objective: (objective != ObjectiveMeta::default()).then_some(objective),
                ..Default::default()
            });
            if spp == 3 && size_c > 1 {
                return Err(Error::UnsupportedVariant("ZVI colour items in several channels".into()));
            }
            series.push(Series { planes, times });
        }
        let mut original = OriginalMeta::default();
        for (id, v) in &image_tags {
            let text = match v {
                TagValue::Int(i) => i.to_string(),
                TagValue::Float(f) => f.to_string(),
                TagValue::Text(t) => t.clone(),
                _ => continue,
            };
            original.pairs.push((format!("Image tag {id}"), text));
        }
        Ok(Zvi { cfb, items, series, metas, original })
    }
}

impl Backend for Zvi {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let m = &self.metas[series];
        let index = (plane.c.min(m.size_c - 1) + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        Ok(PlaneMeta { delta_t_s: self.series[series].times.get(index).copied().flatten(), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let m = &self.metas[series];
        let index = (plane.c + m.size_c * (plane.z + m.size_z * plane.t)) as usize;
        let (w, h) = (region.width as usize, region.height as usize);
        let Some(item) = self.series[series].planes[index].map(|i| &self.items[i]) else {
            for y in 0..h {
                sink.write_with(y, 0, w, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        let bpp = item.bpp as usize;
        let whole = match &item.sectors {
            None => Some(self.cfb.read(&self.cfb.streams()[item.stream])?),
            Some(_) => None,
        };
        scratch::with(w * bpp, |row| {
            for y in 0..h {
                let at = item.data + ((region.y as usize + y) * item.width as usize + region.x as usize) as u64 * bpp as u64;
                match (&item.sectors, &whole) {
                    (Some(ids), _) => self.cfb.read_at(ids, at, row)?,
                    (None, Some(b)) => row.copy_from_slice(b.get(at as usize..at as usize + row.len()).ok_or_else(|| Error::Corrupt("ZVI item truncated".into()))?),
                    _ => unreachable!("sectors or whole stream"),
                }
                // Colour samples are stored blue first.
                if m.samples_per_pixel == 3 {
                    let s = bpp / 3;
                    for px in row.chunks_exact_mut(bpp) {
                        for k in 0..s {
                            px.swap(k, 2 * s + k);
                        }
                    }
                }
                sink.write_with(y, 0, w, |d| {
                    d.copy_from_slice(row);
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_tags_and_dates() {
        // Version, count 1, then (i32 1513, id 515, attribute 0x88).
        let b = [3, 0, 0, 0x10, 0, 0x20, 3, 0, 1, 0, 0, 0, 3, 0, 0xE9, 5, 0, 0, 3, 0, 3, 2, 0, 0, 3, 0, 0x88, 0, 0, 0];
        assert_eq!(parse_tags(&b), vec![(515, TagValue::Int(1513))]);
        let mut pos = 0;
        let text = [8, 0, 4, 0, 0, 0, b'A', 0, b'B', 0];
        assert_eq!(read_value(&text, &mut pos), Some(TagValue::Text("AB".into())));
        assert_eq!(ole_date(45021.6436).as_deref(), Some("2023-04-05T15:26:47"));
    }
}

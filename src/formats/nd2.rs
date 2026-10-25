//! # nd2
//!
//! Nikon NIS-Elements ND2 reader (modern chunk container, versions 2.x and
//! 3.x).
//!
//! Frames are stored as chunks `ImageDataSeq|N!`, uncompressed or
//! zlib-compressed, with all channels interleaved per pixel. The experiment
//! loops (XY positions, time, Z) map frame numbers to series (one per XY
//! position), time points and z-slices. Colour planes are stored blue first
//! and returned as RGB. The legacy JPEG 2000 container of NIS-Elements 2.x
//! is not supported.
//!
//! Provenance: OpenReadout ND2 format notes (MIT OR Apache-2.0) and nd2-rs
//! data structure notes (MIT, aligned with the BSD-3 `nd2` Python package)
//! in `doc/nikon-nd2/`; own analysis of sample files. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

pub(super) mod meta;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use meta::Value;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta, PhysicalSize,
    PixelType, Plane, PlaneMeta, Position, Region, Result, SeriesMeta, codecs, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Nd2, "Nikon ND2", "1");

const CHUNK_MAGIC: u32 = 0x0ABE_CEDA;
const FILE_SIGNATURE: &[u8] = b"ND2 FILE SIGNATURE CHUNK NAME01!";
/// Largest metadata chunk read, against corrupt files.
const MAX_META: u64 = 256 << 20;

/// Returns `true` if `head` starts with the modern ND2 signature chunk.
pub(super) fn matches(head: &[u8]) -> bool {
    head.len() >= 48 && u32::from_le_bytes(head[..4].try_into().expect("4 bytes")) == CHUNK_MAGIC && head[16..].starts_with(FILE_SIGNATURE)
}

/// Kind of an experiment loop (`eType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    T,
    P,
    Z,
}

#[derive(Debug, Clone)]
struct Loop {
    axis: Axis,
    count: u64,
    period_ms: Option<f64>,
    z_step_um: Option<f64>,
    positions: Vec<[Option<f64>; 3]>,
}

/// Nikon ND2 file.
#[derive(Debug)]
pub(super) struct Nd2 {
    file: File,
    len: u64,
    chunks: HashMap<String, (u64, u64)>,
    metas: Vec<SeriesMeta>,
    /// Counted loops, outermost first.
    loops: Vec<Loop>,
    frames: u64,
    width: u32,
    height: u32,
    row_bytes: usize,
    /// Samples per pixel of a frame (all channels).
    components: usize,
    compressed: bool,
    pixel_type: PixelType,
    /// Acquisition time of each frame, ms since the start.
    frame_times: Vec<f64>,
}

impl Nd2 {
    /// Opens an ND2 file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid chunk map or metadata and
    /// [`Error::UnsupportedVariant`] for lossy frames or unknown sample
    /// types.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let chunks = chunk_map(&file, len)?;
        let mut nd2 = Self {
            file,
            len,
            chunks,
            metas: Vec::new(),
            loops: Vec::new(),
            frames: 0,
            width: 0,
            height: 0,
            row_bytes: 0,
            components: 1,
            compressed: false,
            pixel_type: PixelType::U8,
            frame_times: Vec::new(),
        };
        let attrs = nd2.metadata(&["ImageAttributesLV!", "ImageAttributes!"])?.ok_or_else(|| Error::Corrupt("ND2 without image attributes".into()))?;
        let a = attrs.get("SLxImageAttributes").unwrap_or(&attrs);
        let int = |k: &str| a.int(k).unwrap_or(0);
        nd2.width = int("uiWidth") as u32;
        nd2.height = int("uiHeight") as u32;
        nd2.components = int("uiComp").max(1) as usize;
        let bits = int("uiBpcInMemory");
        nd2.pixel_type = match (int("ePixelType"), bits) {
            (2, 32) => PixelType::F32,
            (_, 8) => PixelType::U8,
            (_, 16) => PixelType::U16,
            (_, 32) => PixelType::U32,
            (t, b) => return Err(Error::UnsupportedVariant(format!("ND2 pixel type {t} with {b} bits"))),
        };
        let min_row = nd2.width as usize * nd2.components * nd2.pixel_type.bytes();
        nd2.row_bytes = (int("uiWidthBytes") as usize).max(min_row);
        nd2.compressed = match a.int("eCompression") {
            Some(0) => true,
            Some(1) => return Err(Error::UnsupportedVariant("lossy ND2 compression".into())),
            _ => false,
        };
        if nd2.width == 0 || nd2.height == 0 {
            return Err(Error::Corrupt("ND2 image size 0".into()));
        }
        let stored = nd2.chunks.keys().filter(|k| k.starts_with("ImageDataSeq|")).count() as u64;
        nd2.frames = match int("uiSequenceCount") {
            0 => stored,
            n => (n as u64).min(stored),
        };

        let experiment = nd2.metadata(&["ImageMetadataLV!", "ImageMetadata!"])?;
        nd2.loops = experiment.as_ref().map(experiment_loops).unwrap_or_default();
        fit_loops(&mut nd2.loops, nd2.frames);
        let frame0 = nd2.metadata(&["ImageMetadataSeqLV|0!", "ImageMetadataSeq|0!"])?;
        let calibration = nd2.metadata(&["ImageCalibrationLV|0!", "ImageCalibration|0!"])?;
        nd2.frame_times = nd2.frame_times();
        nd2.metas = nd2.build_metas(frame0.as_ref(), calibration.as_ref())?;
        Ok(nd2)
    }

    /// Payload of the first existing chunk of `names`.
    fn chunk(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(&(offset, len)) = self.chunks.get(name) else { return Ok(None) };
        let (start, data_len) = chunk_payload(&self.file, self.len, offset)?;
        let data_len = data_len.min(len.max(data_len));
        if data_len > MAX_META {
            return Err(Error::Corrupt(format!("ND2 chunk {name} too large")));
        }
        let mut buf = vec![0u8; data_len as usize];
        read_exact_at(&self.file, &mut buf, start)?;
        Ok(Some(buf))
    }

    /// Decoded metadata of the first existing chunk of `names`.
    fn metadata(&self, names: &[&str]) -> Result<Option<Value>> {
        for name in names {
            if let Some(data) = self.chunk(name)? {
                let v = if name.contains("LV") { meta::decode_lv(&data)? } else { meta::decode_xml(&data)? };
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    /// Frame times from `CustomData|AcqTimesCache!` (f64 ms per frame).
    fn frame_times(&self) -> Vec<f64> {
        let Ok(Some(data)) = self.chunk("CustomData|AcqTimesCache!") else { return Vec::new() };
        data.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes"))).collect()
    }

    /// Number of positions, time points and z-slices.
    fn sizes(&self) -> (u64, u64, u64) {
        let product = |axis: Axis| self.loops.iter().filter(|l| l.axis == axis).map(|l| l.count).product::<u64>();
        let counted: u64 = self.loops.iter().map(|l| l.count).product();
        // Frames not described by loops are time points.
        let extra = if self.loops.is_empty() { self.frames.max(1) } else { (self.frames / counted.max(1)).max(1) };
        let t = product(Axis::T) * if self.loops.is_empty() || counted < self.frames { extra } else { 1 };
        (product(Axis::P), t.max(1), product(Axis::Z))
    }

    /// Frame number of (position, z, t).
    fn frame_index(&self, p: u64, z: u64, t: u64) -> u64 {
        if self.loops.is_empty() {
            return t;
        }
        // Split each axis value over its loops (outer loops vary slower).
        let mut rem = [(Axis::P, p), (Axis::Z, z), (Axis::T, t)];
        let mut coords = vec![0u64; self.loops.len()];
        for (i, l) in self.loops.iter().enumerate().rev() {
            let r = rem.iter_mut().find(|(a, _)| *a == l.axis).expect("axis");
            coords[i] = r.1 % l.count;
            r.1 /= l.count;
        }
        coords.iter().zip(&self.loops).fold(0, |acc, (c, l)| acc * l.count + c)
    }

    fn build_metas(&self, frame0: Option<&Value>, calibration: Option<&Value>) -> Result<Vec<SeriesMeta>> {
        let pic = frame0.map(|v| v.get("SLxPictureMetadata").unwrap_or(v));
        // Channels: picture planes, else one gray channel per component.
        let planes: Vec<&Value> = pic
            .and_then(|p| p.get("sPicturePlanes"))
            .and_then(|pp| pp.get("sPlaneNew").or_else(|| pp.get("sPlane")))
            .map(|v| v.items().iter().map(|(_, p)| p).collect())
            .unwrap_or_default();
        let mut channel_samples = Vec::new();
        let mut channels = Vec::new();
        let mut first = 0usize;
        for p in &planes {
            let n = p.int("uiCompCount").unwrap_or(1).max(1) as usize;
            channel_samples.push((first, n));
            first += n;
            let color = p.int("uiColor").map(|c| [c as u8, (c >> 8) as u8, (c >> 16) as u8, 255]);
            channels.push(ChannelMeta {
                name: p.text("sDescription").or_else(|| p.text("sOpticalConfigName")),
                color,
                ..Default::default()
            });
        }
        if first != self.components {
            channel_samples = (0..self.components).map(|c| (c, 1)).collect();
            channels = vec![ChannelMeta::default(); self.components];
        }
        let spp = channel_samples[0].1;
        if channel_samples.iter().any(|&(_, n)| n != spp) || !matches!(spp, 1 | 3) {
            return Err(Error::UnsupportedVariant("ND2 with mixed gray and colour planes".into()));
        }
        let um = pic
            .and_then(|p| p.float("dCalibration"))
            .or_else(|| calibration.and_then(|c| c.get("SLxCalibration").unwrap_or(c).float("dCalibration")))
            .filter(|v| *v > 0.0);
        let objective = pic.and_then(|p| {
            let mag = p.float("dObjectiveMag").filter(|m| *m > 0.0);
            let name = p.text("wsObjectiveName");
            (mag.is_some() || name.is_some()).then(|| ObjectiveMeta {
                model: name,
                magnification: mag,
                numerical_aperture: p.float("dObjectiveNA").filter(|v| *v > 0.0),
                ..Default::default()
            })
        });
        let date = pic.and_then(|p| p.float("dTimeAbsolute")).and_then(julian_to_iso);
        let z_step = self.loops.iter().find(|l| l.axis == Axis::Z).and_then(|l| l.z_step_um).map(|v| v.abs() * 1e3);
        let period = self.loops.iter().find(|l| l.axis == Axis::T).and_then(|l| l.period_ms).filter(|p| *p > 0.0).map(|p| p / 1e3);
        let positions: Vec<[Option<f64>; 3]> = self.loops.iter().find(|l| l.axis == Axis::P).map(|l| l.positions.clone()).unwrap_or_default();
        let (np, nt, nz) = self.sizes();
        let mut metas = Vec::new();
        for p in 0..np.max(1) {
            let pos = positions.get(p as usize);
            metas.push(SeriesMeta {
                name: (np > 1).then(|| format!("Position {}", p + 1)),
                acquisition_date: date.clone(),
                size_c: channel_samples.len() as u32,
                size_z: nz as u32,
                size_t: nt as u32,
                pixel_type: self.pixel_type,
                samples_per_pixel: spp as u8,
                levels: vec![LevelMeta { size_x: self.width as u64, size_y: self.height as u64, tile: None, downsample: 1.0 }],
                physical_size: PhysicalSize { x_nm: um.map(|v| v * 1e3), y_nm: um.map(|v| v * 1e3), z_nm: z_step },
                time_increment_s: period,
                channels: channels.clone(),
                objective: objective.clone(),
                stage_position: pos.map(|p| Position { x_nm: p[0].map(|v| v * 1e3), y_nm: p[1].map(|v| v * 1e3), z_nm: p[2].map(|v| v * 1e3) }),
                ..Default::default()
            });
        }
        Ok(metas)
    }

    fn channel_samples(&self) -> Vec<(usize, usize)> {
        let spp = self.metas[0].samples_per_pixel as usize;
        (0..self.metas[0].size_c as usize).map(|c| (c * spp, spp)).collect()
    }
}

/// Reads the chunk map at the end of the file: name, offset, length.
fn chunk_map(file: &File, len: u64) -> Result<HashMap<String, (u64, u64)>> {
    let mut tail = [0u8; 8];
    if len < 112 {
        return Err(Error::Corrupt("ND2 file too short".into()));
    }
    read_exact_at(file, &mut tail, len - 8)?;
    let map_offset = u64::from_le_bytes(tail);
    let (start, data_len) = chunk_payload(file, len, map_offset)?;
    if data_len > MAX_META {
        return Err(Error::Corrupt("ND2 chunk map too large".into()));
    }
    let mut data = vec![0u8; data_len as usize];
    read_exact_at(file, &mut data, start)?;
    let mut map = HashMap::new();
    let mut pos = 0;
    while let Some(end) = data[pos..].iter().position(|&b| b == b'!') {
        let name = String::from_utf8_lossy(&data[pos..pos + end + 1]).into_owned();
        pos += end + 1;
        let Some(entry) = data.get(pos..pos + 16) else { break };
        let offset = u64::from_le_bytes(entry[..8].try_into().expect("8 bytes"));
        let size = u64::from_le_bytes(entry[8..].try_into().expect("8 bytes"));
        pos += 16;
        if name.starts_with("ND2 CHUNK MAP SIGNATURE") {
            break;
        }
        map.insert(name, (offset, size));
    }
    Ok(map)
}

/// Start and length of the payload of the chunk at `offset`.
fn chunk_payload(file: &File, len: u64, offset: u64) -> Result<(u64, u64)> {
    let mut head = [0u8; 16];
    if offset.checked_add(16).is_none_or(|e| e > len) {
        return Err(Error::Corrupt(format!("ND2 chunk at {offset} beyond the end")));
    }
    read_exact_at(file, &mut head, offset)?;
    if u32::from_le_bytes(head[..4].try_into().expect("4 bytes")) != CHUNK_MAGIC {
        return Err(Error::Corrupt(format!("ND2 chunk at {offset} without magic")));
    }
    let name_len = u32::from_le_bytes(head[4..8].try_into().expect("4 bytes")) as u64;
    let data_len = u64::from_le_bytes(head[8..16].try_into().expect("8 bytes"));
    let start = offset + 16 + name_len;
    if start.checked_add(data_len).is_none_or(|e| e > len) {
        return Err(Error::Corrupt(format!("ND2 chunk at {offset} extends beyond the end")));
    }
    Ok((start, data_len))
}

/// Counted loops of the experiment tree, outermost first.
fn experiment_loops(exp: &Value) -> Vec<Loop> {
    let mut loops = Vec::new();
    let mut node = exp.get("SLxExperiment").unwrap_or(exp);
    for _ in 0..16 {
        let Some(kind) = node.int("eType") else { break };
        let Some(params) = node.get("uLoopPars") else { break };
        let count = params.int("uiCount").unwrap_or(0).max(0) as u64;
        // Spectral loops (6) have no frame count; NE time loops (8) count
        // their periods.
        if kind == 0 || count == 0 && kind != 8 && kind != 6 {
            break;
        }
        match kind {
            // Spectral loop: channels are interleaved within each frame.
            6 => {}
            2 => {
                let valid = validity(node.get("pItemValid"));
                let points: Vec<&Value> = params.get("Points").map(|p| p.items().iter().map(|(_, v)| v).collect()).unwrap_or_default();
                let mut positions = Vec::new();
                for (i, p) in points.iter().enumerate() {
                    if valid.get(i).copied().unwrap_or(true) {
                        positions.push([p.float("dPosX"), p.float("dPosY"), p.float("dPosZ")]);
                    }
                }
                let n = if points.is_empty() { count } else { positions.len() as u64 };
                if n > 0 {
                    loops.push(Loop { axis: Axis::P, count: n, period_ms: None, z_step_um: None, positions });
                }
            }
            4 => {
                let step = params.float("dZStep").filter(|s| *s != 0.0).or_else(|| {
                    let (lo, hi) = (params.float("dZLow")?, params.float("dZHigh")?);
                    (count > 1).then(|| (hi - lo).abs() / (count - 1) as f64)
                });
                loops.push(Loop { axis: Axis::Z, count, period_ms: None, z_step_um: step, positions: Vec::new() });
            }
            8 => {
                // NE time loop: valid periods add their counts.
                let valid = validity(params.get("pPeriodValid"));
                let periods: Vec<&Value> = params.get("pPeriod").map(|p| p.items().iter().map(|(_, v)| v).collect()).unwrap_or_default();
                let n: u64 = periods
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| valid.get(*i).copied().unwrap_or(true))
                    .map(|(_, p)| p.int("uiCount").unwrap_or(0).max(0) as u64)
                    .sum();
                let period = periods.first().and_then(|p| p.float("dPeriod"));
                if n > 0 {
                    loops.push(Loop { axis: Axis::T, count: n, period_ms: period, z_step_um: None, positions: Vec::new() });
                }
            }
            _ => loops.push(Loop { axis: Axis::T, count, period_ms: params.float("dPeriod"), z_step_um: None, positions: Vec::new() }),
        }
        // Continue with the nested loop: the child itself, or its first child.
        let Some(next) = node.get("ppNextLevelEx") else { break };
        node = if next.int("eType").is_some() {
            next
        } else {
            match next.items().iter().map(|(_, v)| v).find(|v| v.int("eType").is_some()) {
                Some(v) => v,
                None => break,
            }
        };
    }
    loops
}

/// Validity flags from a byte array or a list of booleans.
fn validity(v: Option<&Value>) -> Vec<bool> {
    match v {
        Some(Value::Bytes(b)) => b.iter().map(|&x| x != 0).collect(),
        Some(Value::Map(items)) => items.iter().map(|(_, v)| v.as_i64().unwrap_or(1) != 0).collect(),
        _ => Vec::new(),
    }
}

/// Cuts the outermost loop if the loops describe more frames than stored.
fn fit_loops(loops: &mut Vec<Loop>, frames: u64) {
    let inner: u64 = loops.iter().skip(1).map(|l| l.count).product();
    if let Some(first) = loops.first_mut()
        && first.count * inner > frames && inner > 0 {
            first.count = frames.div_ceil(inner).max(1);
        }
    loops.retain(|l| l.count > 0);
}

/// ISO 8601 of a Julian day number (UTC); `None` outside 1900-2100.
pub(super) fn julian_to_iso(jdn: f64) -> Option<String> {
    let ms = ((jdn - 2_440_587.5) * 86_400_000.0).round();
    let secs = (ms / 1000.0).floor() as i64;
    if !(-2_208_988_800..=4_102_444_800).contains(&secs) {
        return None;
    }
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    Some(format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60))
}

impl Backend for Nd2 {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let frame = self.frame_index(series as u64, plane.z as u64, plane.t as u64) as usize;
        Ok(PlaneMeta { delta_t_s: self.frame_times.get(frame).map(|ms| ms / 1e3), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        let mut names: Vec<&String> = self.chunks.keys().filter(|k| !k.starts_with("ImageDataSeq|")).collect();
        names.sort();
        meta.pairs.push(("Chunks".into(), names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        if let Some(text) = self.metadata(&["ImageTextInfoLV!", "ImageTextInfo!"])? {
            flatten(&text, "", &mut meta.pairs);
        }
        Ok(meta)
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let frame = self.frame_index(series as u64, plane.z as u64, plane.t as u64);
        if frame >= self.frames {
            return Err(Error::Corrupt(format!("ND2 frame {frame} missing ({} stored)", self.frames)));
        }
        let (&(offset, _), name) = (
            self.chunks.get(&format!("ImageDataSeq|{frame}!")).ok_or_else(|| Error::Corrupt(format!("ND2 frame {frame} missing")))?,
            frame,
        );
        let _ = name;
        let (start, data_len) = chunk_payload(&self.file, self.len, offset)?;
        // The frame starts with an f64 time stamp.
        let start = start + 8;
        let data_len = data_len.saturating_sub(8);
        let (first, n) = self.channel_samples()[plane.c as usize];
        let bytes = self.pixel_type.bytes();
        let px = self.components * bytes;
        let (x, y0, w, h) = (region.x as usize, region.y as usize, region.width as usize, region.height as usize);
        let frame_len = self.row_bytes * self.height as usize;
        let rgb = n == 3;
        let copy_row = |src: &[u8], dst: &mut [u8]| {
            for (i, d) in dst.chunks_exact_mut(n * bytes).enumerate() {
                let s = &src[(x + i) * px + first * bytes..][..n * bytes];
                if rgb {
                    // Stored blue first.
                    for k in 0..3 {
                        d[k * bytes..(k + 1) * bytes].copy_from_slice(&s[(2 - k) * bytes..(3 - k) * bytes]);
                    }
                } else {
                    d.copy_from_slice(s);
                }
            }
        };
        if self.compressed {
            return scratch::with(data_len as usize, |src| {
                read_exact_at(&self.file, src, start)?;
                scratch::with(frame_len, |frame_buf| {
                    let got = codecs::deflate(src, frame_buf, true)?;
                    if got < frame_len {
                        return Err(Error::Corrupt(format!("ND2 frame decodes to {got} bytes, expected {frame_len}")));
                    }
                    for r in 0..h {
                        let row = &frame_buf[(y0 + r) * self.row_bytes..][..self.row_bytes];
                        sink.write_with(r, 0, w, |dst| {
                            copy_row(row, dst);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
            });
        }
        if (frame_len as u64) > data_len {
            return Err(Error::Corrupt(format!("ND2 frame {frame} shorter than its image")));
        }
        let single = self.components == n && !rgb;
        if single {
            // One gray channel: rows go straight into the destination.
            for r in 0..h {
                let at = start + ((y0 + r) * self.row_bytes + x * px) as u64;
                sink.write_with(r, 0, w, |dst| read_exact_at(&self.file, dst, at))?;
            }
            return Ok(());
        }
        let span = (x + w) * px;
        scratch::with(span, |row| {
            for r in 0..h {
                read_exact_at(&self.file, &mut row[..span], start + ((y0 + r) * self.row_bytes) as u64)?;
                sink.write_with(r, 0, w, |dst| {
                    copy_row(row, dst);
                    Ok(())
                })?;
            }
            Ok(())
        })
    }
}

/// Flattens text and number leaves into `key/subkey` pairs.
fn flatten(v: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
    for (k, child) in v.items() {
        let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}/{k}") };
        match child {
            Value::Map(_) => flatten(child, &key, out),
            Value::Text(s) if !s.trim().is_empty() => out.push((key, s.trim().to_string())),
            Value::Int(i) => out.push((key, i.to_string())),
            Value::Float(f) => out.push((key, f.to_string())),
            _ => {}
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(axis: Axis, count: u64) -> Loop {
        Loop { axis, count, period_ms: None, z_step_um: None, positions: Vec::new() }
    }

    #[test]
    fn frame_index_is_row_major_over_loops() {
        let mut nd2 = Nd2 {
            file: File::open(file!()).unwrap(),
            len: 0,
            chunks: HashMap::new(),
            metas: Vec::new(),
            loops: vec![lp(Axis::T, 3), lp(Axis::P, 4), lp(Axis::Z, 5)],
            frames: 60,
            width: 1,
            height: 1,
            row_bytes: 1,
            components: 1,
            compressed: false,
            pixel_type: PixelType::U8,
            frame_times: Vec::new(),
        };
        // frame = t * 20 + p * 5 + z
        assert_eq!(nd2.frame_index(2, 3, 1), 20 + 10 + 3);
        assert_eq!(nd2.sizes(), (4, 3, 5));
        nd2.loops.clear();
        nd2.frames = 7;
        assert_eq!(nd2.sizes(), (1, 7, 1));
        assert_eq!(nd2.frame_index(0, 0, 6), 6);
    }

    #[test]
    fn julian_dates() {
        // 2017-06-06T09:15:06.980Z
        assert_eq!(julian_to_iso(2_457_910.885_497_454).as_deref(), Some("2017-06-06T09:15:06Z"));
        assert_eq!(julian_to_iso(397.7), None);
    }

    #[test]
    fn loops_are_cut_to_stored_frames() {
        let mut loops = vec![lp(Axis::T, 10), lp(Axis::Z, 4)];
        fit_loops(&mut loops, 21);
        assert_eq!(loops[0].count, 6);
    }
}

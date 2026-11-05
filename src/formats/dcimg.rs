//! # dcimg
//!
//! Hamamatsu DCIMG reader (`.dcimg`): one camera stream of HCImage or
//! other DCAM-API applications, its frames as time points.
//!
//! File header (little endian): `DCIMG\0\0\0`, u32 version at 8 (7: packed
//! layout, 0x01000000: framed layout), u32 session count at 0x20, u32
//! frame count at 0x24, u32 session header offset at 0x28, u64 file size at
//! 0x30. Only the first session is read.
//!
//! Packed layout (version 7), session header fields relative to it: frame
//! count 0x20, bytes per pixel 0x24, width 0x2c, row bytes 0x30, height
//! 0x34, frame bytes 0x38, data offset 0x44, data size (u64) 0x48. Frames
//! follow each other; the footer after the data (u32 7, u64 offset of a
//! second structure at +8) holds per-frame u32 counters (+0x30), u32
//! seconds + u32 microseconds time stamps (+0x40) and stored pixels
//! (+0x58, their offset in a frame at +0x64 and size at +0x68).
//!
//! Framed layout (version 0x01000000): frame count 0x3c, bytes per pixel
//! 0x40, width 0x48, height 0x4c, row bytes 0x50, frame bytes 0x54, data
//! offset (u64) 0x60, frame stride 0x74, trailer bytes 0x7c and a table of
//! 16-byte entries (u32 kind, u32 size, u64 offset from header + 0xa0) at
//! 0xf0 up to the first block. Each frame is followed by a trailer: u32
//! counter, u32 seconds, u32 microseconds and, in 32-byte trailers, the
//! stored pixels at +12. The 248-byte block holds the camera model (+0x40),
//! `S/N:` serial (+0x90) and the sensor sub-array (four u16 at +0xc8); a
//! 56-byte kind-4 block announces stored pixels (size at +12, offset in the
//! frame at +16).
//!
//! Stored pixels: ORCA-Flash4.0 cameras overwrite the first pixels of one
//! row (the sensor's centre row) with `0, 65535, 0, 65535` and store their
//! real values per frame; each frame's own values are put back.
//!
//! Planes are returned bottom row first, as Bio-Formats and bioformats-rs
//! return them (the `dcimg` Python package returns the stored order).
//!
//! Provenance: OpenReadout's DCIMG format description and provenance log
//! (MIT OR Apache-2.0, `doc/hamamatsu-dcimg/`; derived from hex dumps and
//! the `dcimg` Python package, MIT); own hex dumps of the OME / Zenodo
//! samples (CC BY 4.0); checked against Bio-Formats and bioformats-rs as
//! black boxes. No GPL code or Hamamatsu SDK consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PixelType, Plane, PlaneMeta, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Dcimg, "Hamamatsu DCIMG", "1").extensions(&["dcimg"]);

const MAGIC: &[u8; 8] = b"DCIMG\0\0\0";
const VERSION_PACKED: u32 = 7;
const VERSION_FRAMED: u32 = 0x0100_0000;
/// Most frames accepted, against corrupt counts.
const MAX_FRAMES: u64 = 1 << 26;
/// Bytes of rows read with one call.
const READ_CHUNK: usize = 4 << 20;

/// Returns `true` for a DCIMG file header.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(MAGIC)
}

/// Overwritten pixels of one row and where each frame's real values are.
#[derive(Debug, Clone, Copy)]
struct Stored {
    row: u64,
    column: u64,
    count: u64,
    /// Values of frame `i` at `at + i * step`.
    at: u64,
    step: u64,
}

/// One frame's camera counter and time stamp.
#[derive(Debug, Clone, Copy)]
struct Stamp {
    counter: u32,
    seconds: u32,
    micros: u32,
}

impl Stamp {
    /// Microseconds since `earlier` (Unix seconds as f64 would lose
    /// digits).
    fn micros_since(&self, earlier: &Stamp) -> i64 {
        (self.seconds as i64 - earlier.seconds as i64) * 1_000_000 + self.micros as i64 - earlier.micros as i64
    }
}

/// An opened DCIMG file.
#[derive(Debug)]
pub(super) struct Dcimg {
    file: File,
    /// Offset of frame 0 and between frames.
    first: u64,
    stride: u64,
    row_bytes: u64,
    bytes: usize,
    height: u64,
    stored: Option<Stored>,
    stamps: Vec<Stamp>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4).map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    b.get(at..at + 8).map(|v| u64::from_le_bytes(v.try_into().expect("8 bytes")))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|v| u16::from_le_bytes([v[0], v[1]]))
}

/// NUL-terminated text.
fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

/// ISO 8601 UTC of Unix seconds with microseconds.
fn iso8601(seconds: u32, micros: u32) -> String {
    let days = (seconds / 86_400) as i64;
    let rem = seconds % 86_400;
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{micros:06}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

impl Dcimg {
    /// Opens a DCIMG file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid or truncated headers and
    /// [`Error::UnsupportedVariant`] for other versions or sample sizes.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut head = vec![0u8; len.min(64 << 10) as usize];
        read_exact_at(&file, &mut head, 0)?;
        let bad = |what: &str| Error::Corrupt(format!("DCIMG header: {what}"));
        if !matches(&head) {
            return Err(bad("signature"));
        }
        let version = u32_at(&head, 8).ok_or_else(|| bad("version"))?;
        let hb = u32_at(&head, 0x28).ok_or_else(|| bad("session offset"))? as usize;
        let s = |off: usize| u32_at(&head, hb + off).ok_or_else(|| bad("session header beyond the file"));
        let s64 = |off: usize| u64_at(&head, hb + off).ok_or_else(|| bad("session header beyond the file"));
        let mut original = OriginalMeta::default();
        original.pairs.push(("Version".into(), format!("{version:#x}")));
        let (frames, bytes, width, height, row_bytes, frame_bytes, data, stride, layout) = match version {
            VERSION_PACKED => {
                let frame_bytes = s(0x38)? as u64;
                (s(0x20)?, s(0x24)?, s(0x2c)?, s(0x34)?, s(0x30)? as u64, frame_bytes, s(0x44)? as u64, frame_bytes, "packed")
            }
            VERSION_FRAMED => (s(0x3c)?, s(0x40)?, s(0x48)?, s(0x4c)?, s(0x50)? as u64, s(0x54)? as u64, s64(0x60)?, s(0x74)? as u64, "framed"),
            v => return Err(Error::UnsupportedVariant(format!("DCIMG version {v:#x}"))),
        };
        original.pairs.push(("FrameLayout".into(), layout.into()));
        let pixel_type = match bytes {
            1 => PixelType::U8,
            2 => PixelType::U16,
            b => return Err(Error::UnsupportedVariant(format!("DCIMG with {b} bytes per pixel"))),
        };
        let (width, height) = (width as u64, height as u64);
        if width == 0 || height == 0 || row_bytes < width * bytes as u64 || frame_bytes < row_bytes * height || stride < frame_bytes {
            return Err(bad(&format!("frames of {width} x {height}, {row_bytes} bytes per row, {frame_bytes} per frame, stride {stride}")));
        }
        let first = (hb as u64).checked_add(data).ok_or_else(|| bad("data offset"))?;
        // Frames inside the file (a truncated file keeps its complete ones).
        let fit = len.saturating_sub(first).saturating_sub(frame_bytes) / stride + u64::from(len >= first + frame_bytes);
        let frames = (frames as u64).min(fit).min(MAX_FRAMES);
        if frames == 0 {
            return Err(Error::Corrupt("DCIMG without a complete frame".into()));
        }

        let bytes_u = bytes as u64;
        let at_row = |off: u64, size: u64, at: u64, step: u64| {
            let (row, column) = (off / row_bytes, off % row_bytes / bytes_u);
            let count = (size / bytes_u).min(width.saturating_sub(column));
            (row < height && count > 0).then_some(Stored { row, column, count, at, step })
        };
        let mut stamps = Vec::new();
        let mut stored = None;
        if version == VERSION_PACKED {
            // Footer: counters, time stamps and stored pixels.
            let footer = (hb as u64).checked_add(s64(0x48)?).filter(|&f| f + 8 <= len);
            let mut f = [0u8; 16];
            if let Some(footer) = footer
                && read_exact_at(&file, &mut f, footer).is_ok()
                && u32_at(&f, 0) == Some(7)
            {
                let second = footer.saturating_add(u64_at(&f, 8).unwrap_or(0));
                let mut sec = [0u8; 0x70];
                if read_exact_at(&file, &mut sec, second).is_ok() {
                    let rel = |o: usize| u64_at(&sec, o).map(|v| footer.saturating_add(v));
                    let table = |at: Option<u64>, size: u64| -> Option<Vec<u8>> {
                        let at = at?;
                        let n = frames.checked_mul(size)?;
                        if at.checked_add(n)? > len {
                            return None;
                        }
                        let mut b = vec![0u8; n as usize];
                        read_exact_at(&file, &mut b, at).ok()?;
                        Some(b)
                    };
                    let counters = table(rel(0x30), 4);
                    let times = table(rel(0x40), 8);
                    if let (Some(c), Some(t)) = (counters, times) {
                        stamps = (0..frames as usize)
                            .map(|i| Stamp {
                                counter: u32_at(&c, 4 * i).unwrap_or(0),
                                seconds: u32_at(&t, 8 * i).unwrap_or(0),
                                micros: u32_at(&t, 8 * i + 4).unwrap_or(0),
                            })
                            .collect();
                    }
                    let (off, size) = (u32_at(&sec, 0x64).unwrap_or(0) as u64, u64_at(&sec, 0x68).unwrap_or(0));
                    if let Some(at) = rel(0x58).filter(|_| size > 0 && size <= 1 << 16)
                        && at.saturating_add(frames.saturating_mul(size)) <= len
                    {
                        stored = at_row(off, size, at, size);
                    }
                }
            }
        } else {
            let trailer = s(0x7c)? as u64;
            // Block table: 16-byte entries from 0xf0 up to the first block.
            let base = hb + 0xa0;
            let mut end = usize::MAX;
            let mut at = hb + 0xf0;
            let (mut camera, mut announce) = (None, None);
            while at + 16 <= head.len() && at < end {
                let (kind, size, off) = (u32_at(&head, at).unwrap_or(0), u32_at(&head, at + 4).unwrap_or(0), u64_at(&head, at + 8).unwrap_or(0));
                at += 16;
                if kind == 0 && size == 0 {
                    continue;
                }
                let Some(block) = usize::try_from(off).ok().and_then(|o| o.checked_add(base)) else { break };
                end = end.min(block);
                match (kind, size) {
                    (_, 248) => camera = Some(block),
                    (4, 56) => announce = Some(block),
                    _ => {}
                }
            }
            if let Some(c) = camera.and_then(|c| head.get(c..c + 248)) {
                original.pairs.push(("CameraModel".into(), text(&c[0x40..0x60])));
                original.pairs.push(("CameraSerial".into(), text(&c[0x90..0xa0]).trim_start_matches("S/N:").trim().to_string()));
                for (k, o) in [("Version1", 0x00), ("Version2", 0x20), ("Version3", 0x80)] {
                    original.pairs.push((format!("Camera{k}"), text(&c[o..o + 0x20])));
                }
                let sub: Vec<u16> = (0..4).filter_map(|i| u16_at(c, 0xc8 + 2 * i)).collect();
                if let [x, w, y, h] = sub[..] {
                    original.pairs.push(("SubArray".into(), format!("x {x} width {w} y {y} height {h}")));
                    if w as u64 >= width && width > 0 {
                        original.pairs.push(("Binning".into(), (w as u64 / width).to_string()));
                    }
                }
            }
            if let Some(a) = announce.and_then(|a| head.get(a..a + 56))
                && trailer >= 12 + u32_at(a, 12).unwrap_or(0) as u64
            {
                let (size, off) = (u32_at(a, 12).unwrap_or(0) as u64, u32_at(a, 16).unwrap_or(0) as u64);
                stored = at_row(off, size, first + frame_bytes + 12, stride);
            }
            // Trailers: counter and time stamp of every frame.
            if trailer >= 12 && stride >= frame_bytes + 12 {
                let mut t = [0u8; 12];
                for i in 0..frames {
                    if read_exact_at(&file, &mut t, first + i * stride + frame_bytes).is_err() {
                        break;
                    }
                    stamps.push(Stamp {
                        counter: u32_at(&t, 0).unwrap_or(0),
                        seconds: u32_at(&t, 4).unwrap_or(0),
                        micros: u32_at(&t, 8).unwrap_or(0),
                    });
                }
            }
        }
        if let Some(st) = &stored {
            original.pairs.push(("StoredPixels".into(), format!("row {} column {} count {}", st.row, st.column, st.count)));
        }
        if let (Some(a), Some(b)) = (stamps.first(), stamps.last()) {
            original.pairs.push(("FrameCounters".into(), format!("{} - {}", a.counter, b.counter)));
        }
        let plausible = stamps.first().is_some_and(|s| s.seconds > 0);
        let time_increment_s = match (stamps.first(), stamps.last()) {
            (Some(a), Some(b)) if plausible && stamps.len() > 1 && b.micros_since(a) > 0 => {
                Some(b.micros_since(a) as f64 / (stamps.len() - 1) as f64 / 1e6)
            }
            _ => None,
        };
        let meta = SeriesMeta {
            name: path.file_stem().map(|n| n.to_string_lossy().into_owned()),
            acquisition_date: stamps.first().filter(|_| plausible).map(|s| iso8601(s.seconds, s.micros)),
            size_c: 1,
            size_z: 1,
            size_t: frames as u32,
            pixel_type,
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }],
            time_increment_s,
            channels: vec![ChannelMeta::default()],
            ..Default::default()
        };
        if !plausible {
            stamps.clear();
        }
        Ok(Dcimg { file, first, stride, row_bytes, bytes: bytes as usize, height, stored, stamps, metas: vec![meta], original })
    }
}

impl Backend for Dcimg {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        let delta = match (self.stamps.first(), self.stamps.get(plane.t as usize)) {
            (Some(a), Some(b)) => Some(b.micros_since(a) as f64 / 1e6),
            _ => None,
        };
        Ok(PlaneMeta { delta_t_s: delta, ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let e = self.bytes;
        let frame = self.first + plane.t as u64 * self.stride;
        let (w, rows) = (region.width as usize, region.height as usize);
        // This frame's real values of the overwritten pixels. Rows are
        // returned bottom row first (stored row `height - 1 - y`).
        let flip = |y: u64| self.height - 1 - y;
        let mut fix = [0u8; 64];
        let stored = match self.stored {
            Some(st)
                if (region.y..region.y + rows as u64).contains(&flip(st.row))
                    && st.column < region.x + w as u64
                    && region.x < st.column + st.count =>
            {
                let n = (st.count as usize * e).min(fix.len());
                read_exact_at(&self.file, &mut fix[..n], st.at + plane.t as u64 * st.step)?;
                Some(st)
            }
            _ => None,
        };
        // Bands of whole rows, each read with one call.
        let row_bytes = self.row_bytes as usize;
        let band = (READ_CHUNK / row_bytes).clamp(1, rows);
        scratch::with(band * row_bytes, |buf| {
            for r0 in (0..rows).step_by(band) {
                let n = band.min(rows - r0);
                // Stored rows of output rows r0 .. r0 + n, lowest first.
                let low = flip(region.y + (r0 + n - 1) as u64);
                let span = (n - 1) * row_bytes + w * e;
                read_exact_at(&self.file, &mut buf[..span], frame + low * self.row_bytes + region.x * e as u64)?;
                for r in 0..n {
                    let stored_row = flip(region.y + (r0 + r) as u64);
                    let src = &mut buf[(stored_row - low) as usize * row_bytes..][..w * e];
                    if let Some(st) = stored.filter(|st| st.row == stored_row) {
                        for k in 0..st.count {
                            let x = st.column + k;
                            if (region.x..region.x + w as u64).contains(&x) {
                                let at = (x - region.x) as usize * e;
                                src[at..at + e].copy_from_slice(&fix[k as usize * e..(k as usize + 1) * e]);
                            }
                        }
                    }
                    sink.write_with(r0 + r, 0, w, |d| {
                        d.copy_from_slice(src);
                        if e > 1 && cfg!(target_endian = "big") {
                            d.chunks_exact_mut(e).for_each(<[u8]>::reverse);
                        }
                        Ok(())
                    })?;
                }
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
    fn formats_unix_time() {
        assert_eq!(iso8601(1_511_904_855, 664_884), "2017-11-28T21:34:15.664884Z");
        assert_eq!(iso8601(1_725_555_197, 0), "2024-09-05T16:53:17.000000Z");
        assert_eq!(iso8601(951_782_400, 1), "2000-02-29T00:00:00.000001Z");
    }
}

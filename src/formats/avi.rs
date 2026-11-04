//! # avi
//!
//! AVI (Audio Video Interleave) reader: video streams of RIFF `AVI ` files,
//! including OpenDML (AVI 2.0) files larger than 1 GB.
//!
//! Container: the `RIFF 'AVI '` chunk holds `LIST 'hdrl'` (`avih` main
//! header; one `LIST 'strl'` per stream with `strh` stream header, `strf`
//! format — a `BITMAPINFOHEADER` with colour table for video —, `strn`
//! name and the OpenDML `indx` super index), `LIST 'INFO'` tags,
//! `LIST 'movi'` with the data chunks (`##db` / `##dc`, possibly grouped in
//! `LIST 'rec '`) and the `idx1` index. OpenDML files continue in
//! `RIFF 'AVIX'` chunks with further `movi` lists. Frames are located by
//! the `indx` / `ix##` standard indexes, else `idx1` (offsets relative to
//! the `movi` list or absolute, detected by checking the chunk id at the
//! offset), else a scan of all `movi` lists. A zero-size video chunk is a
//! dropped frame and repeats the previous frame. Each video stream is a
//! series with one time point per frame; audio is ignored.
//!
//! Codecs (from `strf.biCompression`, not `strh.fccHandler`): uncompressed
//! 8-bit palette, 16-, 24- and 32-bit (also fourccs `DIB `, `RGB `,
//! `RAW `), 8-bit gray `Y800` / `GREY` / `Y8  ` (top-down, rows not
//! padded), MSRLE (`BI_RLE8`, `BI_RLE4`, `mrle`; delta frames applied on
//! the previous frame from the last key frame) and Motion JPEG (`MJPG`,
//! `JPEG`, `BI_JPEG`; frames without Huffman tables get the standard tables
//! of ITU-T T.81 Annex K.3). Gray palettes give one sample, colour
//! palettes and true colour RGB; 32-bit alpha is kept only when the first
//! frame has alpha values other than all 0 or all 255. Other codecs
//! (inter-frame codecs such as H.264, MPEG-4, Cinepak) are not supported.
//!
//! Provenance: Microsoft "AVI RIFF File Reference" (AVIMAINHEADER,
//! AVISTREAMHEADER, AVIOLDINDEX, AVISUPERINDEX, AVISTDINDEX, `doc/avi/`),
//! OpenDML AVI File Format Extensions 1.02 (`doc/avi/`), RIFF
//! (`doc/riff/`), BITMAPINFOHEADER and bitmap compression pages
//! (`doc/bmp/`), ITU-T T.81 (JPEG) Annex K; own analysis of sample files;
//! checked against Bio-Formats and bioformats-rs as black boxes. No GPL
//! code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::dib::{self, BI_BITFIELDS, BI_RGB, BI_RLE4, BI_RLE8, Pixels, packed_masks, parse_header};
use crate::codecs::jpeg::{self, Color};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PixelType, Plane, Region, Result, SeriesMeta, scratch};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Avi, "AVI (Audio Video Interleave)", "1").extensions(&["avi"]);

/// Most frames of one stream, against corrupt indexes.
const MAX_FRAMES: usize = 1 << 24;
/// Largest frame decoded (bytes of samples).
const MAX_FRAME_BYTES: u64 = 1 << 31;
/// Deepest LIST nesting followed.
const MAX_DEPTH: usize = 8;
/// `idx1` / `ix##` key frame flag.
const AVIIF_KEYFRAME: u32 = 0x10;
const BI_JPEG: u32 = 4;

/// Returns `true` for a RIFF AVI file.
pub(super) fn matches(head: &[u8]) -> bool {
    head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"AVI "
}

fn fourcc(v: u32) -> String {
    v.to_le_bytes().iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' }).collect()
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2).map_or(0, |v| u16::from_le_bytes([v[0], v[1]]))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4).map_or(0, |v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    b.get(at..at + 8).map_or(0, |v| u64::from_le_bytes(v.try_into().expect("8 bytes")))
}

/// Standard Huffman tables of ITU-T T.81 Annex K.3 as a tables-only JPEG
/// stream (SOI, DHT, EOI), for Motion JPEG frames that omit them.
fn standard_tables() -> Vec<u8> {
    const DC_LUMA_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
    const DC_CHROMA_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
    const DC_VALUES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    const AC_LUMA_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d];
    const AC_LUMA_VALUES: [u8; 162] = [
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1,
        0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26,
        0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56,
        0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85,
        0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa,
        0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
        0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9,
        0xfa,
    ];
    const AC_CHROMA_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
    const AC_CHROMA_VALUES: [u8; 162] = [
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42,
        0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19,
        0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55,
        0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8,
        0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4,
        0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9,
        0xfa,
    ];
    let tables: [(u8, &[u8; 16], &[u8]); 4] = [
        (0x00, &DC_LUMA_BITS, &DC_VALUES),
        (0x10, &AC_LUMA_BITS, &AC_LUMA_VALUES),
        (0x01, &DC_CHROMA_BITS, &DC_VALUES),
        (0x11, &AC_CHROMA_BITS, &AC_CHROMA_VALUES),
    ];
    let mut out = vec![0xFF, 0xD8];
    for (class_id, bits, values) in tables {
        let len = 2 + 1 + 16 + values.len();
        out.extend([0xFF, 0xC4, (len >> 8) as u8, len as u8, class_id]);
        out.extend_from_slice(bits);
        out.extend_from_slice(values);
    }
    out.extend([0xFF, 0xD9]);
    out
}

/// Whether a JPEG stream has a DHT segment before its first scan.
fn has_huffman_tables(data: &[u8]) -> bool {
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        match marker {
            0xC4 => return true,
            0xDA | 0xD9 => return false,
            0xFF => i += 1,
            0x01 | 0xD0..=0xD8 => i += 2,
            _ => i += 2 + u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize,
        }
    }
    false
}

/// Component count of a JPEG stream from its frame header.
fn jpeg_components(data: &[u8]) -> Option<u8> {
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        match data[i + 1] {
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return data.get(i + 9).copied(),
            0xDA | 0xD9 => return None,
            0xFF => i += 1,
            0x01 | 0xD0..=0xD8 => i += 2,
            _ => i += 2 + u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize,
        }
    }
    None
}

/// How the frames of a stream are coded.
#[derive(Debug)]
enum Codec {
    /// Uncompressed DIB rows of `stride` bytes.
    Raw { pixels: Pixels, stride: usize, top_down: bool },
    /// MSRLE palette indices, rendered with the stream palette.
    Rle { bits: u8, pixels: Pixels },
    /// Motion JPEG, `samples` 1 (gray) or 3 (RGB).
    Jpeg { samples: u8 },
}

/// One frame: data offset and size in the file, key frame flag.
#[derive(Debug, Clone, Copy)]
struct Frame {
    offset: u64,
    size: u32,
    key: bool,
}

/// One video stream.
#[derive(Debug)]
struct Stream {
    codec: Codec,
    frames: Vec<Frame>,
    width: usize,
    height: usize,
    /// Output samples per pixel.
    samples: usize,
}

/// Streams found in `hdrl` before the frames are indexed.
struct StreamInfo {
    kind: u32,
    strh: Vec<u8>,
    strf: Vec<u8>,
    name: Option<String>,
    indx: Option<Vec<u8>>,
}

/// The last decoded frame: (series, frame, samples).
type FrameCache = Mutex<Option<(usize, usize, Arc<Vec<u8>>)>>;

/// AVI file.
#[derive(Debug)]
pub(super) struct Avi {
    file: File,
    streams: Vec<Stream>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
    /// Last decoded frame: (series, frame, samples).
    cache: FrameCache,
}

/// Reader of the RIFF chunk tree.
struct Riff<'a> {
    file: &'a File,
    len: u64,
}

impl Riff<'_> {
    /// Chunk header at `at`: (id, size, data start).
    fn header(&self, at: u64) -> Option<(u32, u32, u64)> {
        if at.checked_add(8)? > self.len {
            return None;
        }
        let mut h = [0u8; 8];
        read_exact_at(self.file, &mut h, at).ok()?;
        Some((u32_at(&h, 0), u32_at(&h, 4), at + 8))
    }

    /// Bytes of a chunk, at most `max`.
    fn read(&self, at: u64, size: u32, max: usize) -> Result<Vec<u8>> {
        let n = (size as u64).min(self.len.saturating_sub(at)).min(max as u64) as usize;
        let mut b = vec![0u8; n];
        read_exact_at(self.file, &mut b, at)?;
        Ok(b)
    }

    /// The list type (fourcc after `LIST` / `RIFF`) at `at`.
    fn list_type(&self, at: u64) -> u32 {
        let mut t = [0u8; 4];
        if read_exact_at(self.file, &mut t, at).is_err() {
            return 0;
        }
        u32::from_le_bytes(t)
    }
}

const fn id(s: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*s)
}

/// Stream number of a data chunk id `##xx`.
fn stream_of(ckid: u32) -> Option<usize> {
    let b = ckid.to_le_bytes();
    let d = |c: u8| (c as char).to_digit(16);
    Some((d(b[0])? * 16 + d(b[1])?) as usize)
}

/// Whether a chunk id names a video frame (`##db` or `##dc`).
fn is_video_chunk(ckid: u32) -> bool {
    let b = ckid.to_le_bytes();
    &b[2..] == b"db" || &b[2..] == b"dc"
}

/// Everything collected while walking the file.
#[derive(Default)]
struct Walk {
    avih: Vec<u8>,
    streams: Vec<StreamInfo>,
    info: Vec<(String, String)>,
    /// (start of the `movi` fourcc, end) of each movi list.
    movi: Vec<(u64, u64)>,
    idx1: Option<Vec<u8>>,
    riff_count: usize,
}

fn walk(r: &Riff, start: u64, end: u64, depth: usize, w: &mut Walk) -> Result<()> {
    let mut at = start;
    while let Some((ck, size, data)) = r.header(at) {
        if at + 8 > end {
            break;
        }
        let next = data.saturating_add(size as u64 + (size as u64 & 1));
        let chunk_end = data.saturating_add(size as u64).min(end);
        match ck {
            c if c == id(b"RIFF") || c == id(b"LIST") => {
                let t = r.list_type(data);
                if c == id(b"RIFF") {
                    w.riff_count += 1;
                }
                if t == id(b"movi") {
                    w.movi.push((data, chunk_end));
                } else if t == id(b"strl") {
                    let mut s = StreamInfo { kind: 0, strh: Vec::new(), strf: Vec::new(), name: None, indx: None };
                    let mut sub = data + 4;
                    while let Some((sck, ssize, sdata)) = r.header(sub) {
                        if sub + 8 > chunk_end {
                            break;
                        }
                        match &sck.to_le_bytes() {
                            b"strh" => s.strh = r.read(sdata, ssize, 256)?,
                            b"strf" => s.strf = r.read(sdata, ssize, 64 << 10)?,
                            b"strn" => s.name = Some(String::from_utf8_lossy(&r.read(sdata, ssize, 4096)?).trim_end_matches('\0').to_string()),
                            b"indx" => s.indx = Some(r.read(sdata, ssize, 16 << 20)?),
                            _ => {}
                        }
                        sub = sdata.saturating_add(ssize as u64 + (ssize as u64 & 1));
                    }
                    s.kind = u32_at(&s.strh, 0);
                    w.streams.push(s);
                } else if t == id(b"INFO") {
                    let mut sub = data + 4;
                    while let Some((ick, isize, idata)) = r.header(sub) {
                        if sub + 8 > chunk_end {
                            break;
                        }
                        let text = String::from_utf8_lossy(&r.read(idata, isize, 64 << 10)?).trim_end_matches('\0').to_string();
                        w.info.push((format!("INFO {}", fourcc(ick)), text));
                        sub = idata.saturating_add(isize as u64 + (isize as u64 & 1));
                    }
                } else if depth < MAX_DEPTH && (c == id(b"RIFF") || t == id(b"hdrl") || t == id(b"odml")) {
                    walk(r, data + 4, chunk_end, depth + 1, w)?;
                }
            }
            c if c == id(b"avih") => w.avih = r.read(data, size, 256)?,
            c if c == id(b"idx1") => w.idx1 = Some(r.read(data, size, 1 << 30)?),
            _ => {}
        }
        if next <= at {
            break;
        }
        at = next;
    }
    Ok(())
}

/// Frames of every stream from `idx1`; `None` if it does not match the file.
fn frames_from_idx1(r: &Riff, idx1: &[u8], movi: (u64, u64)) -> Option<Vec<Vec<Frame>>> {
    let entries: Vec<(u32, u32, u64, u32)> =
        idx1.chunks_exact(16).map(|e| (u32_at(e, 0), u32_at(e, 4), u32_at(e, 8) as u64, u32_at(e, 12))).collect();
    let first = entries.iter().find(|e| e.0 != id(b"rec ") && stream_of(e.0).is_some())?;
    // Offsets point at the chunk header, relative to the `movi` fourcc or
    // absolute: the chunk id there tells which.
    let check = |base: u64| r.header(base + first.2).is_some_and(|(ck, _, _)| ck == first.0);
    let base = if check(movi.0) {
        movi.0
    } else if check(0) {
        0
    } else {
        return None;
    };
    let mut out: Vec<Vec<Frame>> = Vec::new();
    for &(ck, flags, off, size) in &entries {
        let (Some(s), true) = (stream_of(ck), is_video_chunk(ck)) else { continue };
        if s >= 100 {
            continue;
        }
        if out.len() <= s {
            out.resize_with(s + 1, Vec::new);
        }
        let at = base + off + 8;
        if at + size as u64 > r.len {
            return None;
        }
        out[s].push(Frame { offset: at, size, key: flags & AVIIF_KEYFRAME != 0 });
    }
    Some(out)
}

/// Frames of one stream from its OpenDML super index and standard indexes.
fn frames_from_indx(r: &Riff, indx: &[u8]) -> Option<Vec<Frame>> {
    let longs = u16_at(indx, 0) as usize;
    let index_type = indx.get(3).copied()?;
    let n = u32_at(indx, 4) as usize;
    let mut frames = Vec::new();
    // A standard index chunk body: base offset, then (offset, size) pairs.
    let chunks = |std: &[u8], frames: &mut Vec<Frame>| -> Option<()> {
        let count = u32_at(std, 4) as usize;
        let base = u64_at(std, 12);
        for e in std.get(24..)?.chunks_exact(8).take(count) {
            let (off, raw) = (u32_at(e, 0) as u64, u32_at(e, 4));
            let size = raw & 0x7FFF_FFFF;
            let at = base.checked_add(off)?;
            if at + size as u64 > r.len || frames.len() >= MAX_FRAMES {
                return None;
            }
            frames.push(Frame { offset: at, size, key: raw & 0x8000_0000 == 0 });
        }
        Some(())
    };
    match index_type {
        // AVI_INDEX_OF_INDEXES: entries (offset, size, duration) of ix## chunks.
        0 => {
            if longs != 4 {
                return None;
            }
            for e in indx.get(24..)?.chunks_exact(16).take(n) {
                let (off, size) = (u64_at(e, 0), u32_at(e, 8));
                let (_, csize, data) = r.header(off)?;
                if size < 8 {
                    return None;
                }
                let body = r.read(data, csize, 64 << 20).ok()?;
                chunks(&body, &mut frames)?;
            }
        }
        // AVI_INDEX_OF_CHUNKS directly in `indx`.
        1 => chunks(indx, &mut frames)?,
        _ => return None,
    }
    Some(frames)
}

/// Frames of every stream from a scan of the `movi` lists.
fn frames_from_scan(r: &Riff, movi: &[(u64, u64)]) -> Vec<Vec<Frame>> {
    let mut out: Vec<Vec<Frame>> = Vec::new();
    fn scan(r: &Riff, start: u64, end: u64, depth: usize, out: &mut Vec<Vec<Frame>>) {
        let mut at = start;
        while let Some((ck, size, data)) = r.header(at) {
            if data + size as u64 > end {
                break;
            }
            if ck == id(b"LIST") && depth < MAX_DEPTH {
                scan(r, data + 4, data + size as u64, depth + 1, out);
            } else if let Some(s) = stream_of(ck).filter(|&s| s < 100 && is_video_chunk(ck)) {
                if out.len() <= s {
                    out.resize_with(s + 1, Vec::new);
                }
                if out[s].len() < MAX_FRAMES {
                    out[s].push(Frame { offset: data, size, key: true });
                }
            }
            let next = data + size as u64 + (size as u64 & 1);
            if next <= at {
                break;
            }
            at = next;
        }
    }
    for &(start, end) in movi {
        scan(r, start + 4, end, 0, &mut out);
    }
    out
}

impl Avi {
    /// Opens an AVI file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid files and
    /// [`Error::UnsupportedVariant`] when no video stream has a supported
    /// codec.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let r = Riff { file: &file, len };
        let mut head = [0u8; 12];
        read_exact_at(&file, &mut head, 0).map_err(|_| Error::Corrupt("AVI shorter than its RIFF header".into()))?;
        if !matches(&head) {
            return Err(Error::Corrupt("not a RIFF AVI file".into()));
        }
        let mut w = Walk::default();
        walk(&r, 0, len, 0, &mut w)?;
        let mut original = OriginalMeta::default();
        let avih = &w.avih;
        let main = [
            ("MicroSecPerFrame", 0),
            ("MaxBytesPerSec", 4),
            ("PaddingGranularity", 8),
            ("Flags", 12),
            ("TotalFrames", 16),
            ("InitialFrames", 20),
            ("Streams", 24),
            ("SuggestedBufferSize", 28),
            ("Width", 32),
            ("Height", 36),
        ];
        if avih.len() >= 40 {
            for (k, at) in main {
                original.pairs.push((format!("avih {k}"), u32_at(avih, at).to_string()));
            }
        }
        original.pairs.push(("RIFF chunks".into(), w.riff_count.to_string()));
        original.pairs.extend(w.info.iter().cloned());

        // Frame indexes: OpenDML per stream, else idx1, else a scan.
        let idx1 = w.idx1.as_deref().zip(w.movi.first().copied()).and_then(|(i, m)| frames_from_idx1(&r, i, m));
        let mut scanned: Option<Vec<Vec<Frame>>> = None;
        let mut streams = Vec::new();
        let mut metas = Vec::new();
        let mut unsupported = Vec::new();
        for (n, s) in w.streams.iter().enumerate() {
            if s.kind != id(b"vids") {
                continue;
            }
            let p = |k: &str, v: String| (format!("stream {n} {k}"), v);
            let strh = &s.strh;
            original.pairs.extend([
                p("fccType", fourcc(u32_at(strh, 0))),
                p("fccHandler", fourcc(u32_at(strh, 4))),
                p("Flags", u32_at(strh, 8).to_string()),
                p("Scale", u32_at(strh, 20).to_string()),
                p("Rate", u32_at(strh, 24).to_string()),
                p("Start", u32_at(strh, 28).to_string()),
                p("Length", u32_at(strh, 32).to_string()),
            ]);
            if let Some(name) = &s.name {
                original.pairs.push(p("strn", name.clone()));
            }
            if s.strf.len() < 40 {
                return Err(Error::Corrupt(format!("AVI stream {n} without a bitmap header")));
            }
            let mut strf = s.strf.clone();
            strf.resize(strf.len().max(124), 0);
            let mut h = parse_header(&strf)?;
            // The header size field is not trusted beyond the chunk.
            h.size = h.size.clamp(40, s.strf.len() as u32);
            original.pairs.extend(dib::original_meta(&h, h.height < 0).into_iter().map(|(k, v)| p(&format!("strf {k}"), v)));
            original.pairs.push(p("strf Compression fourcc", fourcc(h.compression)));
            let codec = match self::codec(&h, &s.strf) {
                Ok(c) => c,
                Err(e) => {
                    unsupported.push(e);
                    continue;
                }
            };
            if h.width <= 0 || h.height == 0 || h.height == i32::MIN {
                return Err(Error::Corrupt(format!("AVI stream {n} with size {}x{}", h.width, h.height)));
            }
            let (width, height) = (h.width as usize, h.height.unsigned_abs() as usize);
            let frames = match s.indx.as_deref().and_then(|i| frames_from_indx(&r, i)) {
                Some(f) => f,
                None => match idx1.as_ref() {
                    Some(all) => all.get(n).cloned().unwrap_or_default(),
                    None => scanned.get_or_insert_with(|| frames_from_scan(&r, &w.movi)).get(n).cloned().unwrap_or_default(),
                },
            };
            // Dropped frames (empty chunks) repeat the previous frame.
            let mut frames = frames;
            for i in 1..frames.len() {
                if frames[i].size == 0 {
                    frames[i] = Frame { key: false, ..frames[i - 1] };
                }
            }
            if frames.is_empty() {
                return Err(Error::Corrupt(format!("AVI stream {n} without frames")));
            }
            let mut stream = Stream { codec, frames, width, height, samples: 0 };
            stream.samples = match &stream.codec {
                Codec::Raw { pixels, .. } | Codec::Rle { pixels, .. } => dib::samples_per_pixel(pixels) as usize,
                Codec::Jpeg { samples } => *samples as usize,
            };
            if let Codec::Jpeg { samples } = &mut stream.codec {
                // Gray or colour from the first frame's frame header.
                let f = stream.frames.iter().find(|f| f.size > 0).copied();
                if let Some(f) = f {
                    let head = r.read(f.offset, f.size, 64 << 10)?;
                    if jpeg_components(&head) == Some(1) {
                        *samples = 1;
                    }
                }
                stream.samples = *samples as usize;
            }
            if let Codec::Raw { pixels: Pixels::Packed { bytes: 4, masks }, stride, .. } = &mut stream.codec {
                // Alpha that is all 0 or all 255 in the first frame is no data.
                if masks.len() == 4 {
                    let f = stream.frames[0];
                    let frame = r.read(f.offset, f.size, (*stride * height).min(64 << 20))?;
                    let alphas: Vec<u8> = frame.chunks_exact(4).map(|px| masks[3].sample(u32::from_le_bytes([px[0], px[1], px[2], px[3]]))).collect();
                    if alphas.iter().all(|&a| a == 0) || alphas.iter().all(|&a| a == 255) {
                        masks.pop();
                    }
                }
                stream.samples = masks.len();
            }
            if (width * height * stream.samples) as u64 > MAX_FRAME_BYTES {
                return Err(Error::UnsupportedVariant("AVI frames above 2 GiB".into()));
            }
            let (scale, rate) = (u32_at(strh, 20), u32_at(strh, 24));
            let dt = if scale > 0 && rate > 0 { Some(scale as f64 / rate as f64) } else { Some(u32_at(avih, 0) as f64 / 1e6).filter(|v| *v > 0.0) };
            metas.push(SeriesMeta {
                name: s.name.clone().filter(|n| !n.is_empty()).or_else(|| Some(format!("video stream {n}"))),
                size_c: 1,
                size_z: 1,
                size_t: stream.frames.len() as u32,
                pixel_type: PixelType::U8,
                samples_per_pixel: stream.samples as u8,
                levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
                time_increment_s: dt,
                channels: vec![ChannelMeta::default()],
                ..Default::default()
            });
            streams.push(stream);
        }
        if streams.is_empty() {
            return Err(unsupported.into_iter().next().unwrap_or_else(|| Error::Corrupt("AVI without video stream".into())));
        }
        Ok(Avi { file, streams, metas, original, cache: Mutex::new(None) })
    }

    /// Decodes a whole frame (output samples, top row first).
    fn decode(&self, series: usize, t: usize) -> Result<Arc<Vec<u8>>> {
        if let Some((_, _, d)) =
            self.cache.lock().map_err(|_| Error::Corrupt("AVI cache poisoned".into()))?.as_ref().filter(|(s, f, _)| (*s, *f) == (series, t))
        {
            return Ok(d.clone());
        }
        let st = &self.streams[series];
        let (w, h) = (st.width, st.height);
        let read = |f: Frame| -> Result<Vec<u8>> {
            let mut b = vec![0u8; f.size as usize];
            read_exact_at(&self.file, &mut b, f.offset)?;
            Ok(b)
        };
        let out = match &st.codec {
            Codec::Jpeg { samples } => {
                let f = st.frames[t];
                let mut out = vec![0u8; w * h * *samples as usize];
                if f.size > 0 {
                    let data = read(f)?;
                    let tables = (!has_huffman_tables(&data)).then(standard_tables);
                    let mut buf = vec![0u8; w * h * 3];
                    let info = jpeg::decode(&data, tables.as_deref(), Color::YcbcrToRgb, &mut buf)?;
                    if (info.width, info.height) != (w, h) {
                        return Err(Error::Corrupt(format!("AVI JPEG frame {}x{}, stream {w}x{h}", info.width, info.height)));
                    }
                    match (info.components, *samples) {
                        (1, 1) | (3, 3) => {
                            let n = out.len();
                            out.copy_from_slice(&buf[..n]);
                        }
                        (1, _) => out.iter_mut().zip(buf.iter().flat_map(|&v| [v, v, v])).for_each(|(o, v)| *o = v),
                        _ => out
                            .iter_mut()
                            .zip(buf.chunks_exact(3))
                            .for_each(|(o, rgb)| *o = ((rgb[0] as u32 * 299 + rgb[1] as u32 * 587 + rgb[2] as u32 * 114) / 1000) as u8),
                    }
                }
                out
            }
            Codec::Rle { bits, pixels: Pixels::Indexed { lut, .. } } => {
                // Delta frames apply onto the previous frame from the last key frame.
                let key = (0..=t).rev().find(|&i| st.frames[i].key).unwrap_or(0);
                let mut indices = vec![0u8; w * h];
                for i in key..=t {
                    let f = st.frames[i];
                    if f.size > 0 {
                        dib::decode_rle_into(&read(f)?, *bits, w, h, &mut indices);
                    }
                }
                let samples = if lut.gray { 1 } else { 3 };
                indices.iter().flat_map(|&i| lut.rgb[i as usize][..samples].to_vec()).collect()
            }
            Codec::Rle { .. } => return Err(Error::Corrupt("AVI RLE stream without palette".into())),
            Codec::Raw { .. } => return Err(Error::Corrupt("AVI raw frames are read by rows".into())),
        };
        let out = Arc::new(out);
        *self.cache.lock().map_err(|_| Error::Corrupt("AVI cache poisoned".into()))? = Some((series, t, out.clone()));
        Ok(out)
    }
}

/// The codec of a video stream from its bitmap header.
fn codec(h: &dib::Header, strf: &[u8]) -> Result<Codec> {
    let bits = h.bits;
    let width = h.width.max(0) as usize;
    let stride = (width * bits as usize).div_ceil(32) * 4;
    let palette = || {
        let at = h.size as usize;
        let max = 1usize << bits.min(8);
        let count = if h.colors_used == 0 || h.colors_used as usize > max { max } else { h.colors_used as usize };
        let raw = strf.get(at..).unwrap_or_default();
        let raw = &raw[..raw.len().min(count * 4) / 4 * 4];
        if raw.is_empty() {
            // No colour table: a gray ramp.
            let ramp: Vec<u8> = (0..max)
                .flat_map(|i| {
                    let v = (i * 255 / (max - 1).max(1)) as u8;
                    [v, v, v, 0]
                })
                .collect();
            return dib::palette(&ramp, 4, bits);
        }
        dib::palette(raw, 4, bits)
    };
    let raw_rgb = |top_down: bool| -> Result<Codec> {
        let pixels = match bits {
            1 | 2 | 4 | 8 => Pixels::Indexed { bits: bits as u8, lut: palette() },
            24 => Pixels::Bgr24,
            16 => Pixels::Packed { bytes: 2, masks: packed_masks(&[0x7C00, 0x03E0, 0x001F], 0)? },
            32 => Pixels::Packed { bytes: 4, masks: packed_masks(&[0xFF_0000, 0xFF00, 0xFF], 0xFF00_0000)? },
            b => return Err(Error::Corrupt(format!("AVI uncompressed video with {b} bits per pixel"))),
        };
        Ok(Codec::Raw { pixels, stride, top_down })
    };
    match h.compression {
        BI_RGB => raw_rgb(h.height < 0),
        c if c == id(b"DIB ") || c == id(b"RGB ") || c == id(b"RAW ") => raw_rgb(h.height < 0),
        BI_BITFIELDS if bits == 16 || bits == 32 => {
            let m = |i: usize| u32_at(strf, h.size as usize + 4 * i);
            let masks = if h.size >= 52 { [h.masks[0], h.masks[1], h.masks[2]] } else { [m(0), m(1), m(2)] };
            Ok(Codec::Raw { pixels: Pixels::Packed { bytes: (bits / 8) as u8, masks: packed_masks(&masks, 0)? }, stride, top_down: h.height < 0 })
        }
        c if (c == id(b"Y800") || c == id(b"GREY") || c == id(b"Y8  ")) && bits == 8 => {
            let ramp: Vec<u8> = (0..=255u8).flat_map(|v| [v, v, v, 0]).collect();
            Ok(Codec::Raw { pixels: Pixels::Indexed { bits: 8, lut: dib::palette(&ramp, 4, 8) }, stride: width, top_down: true })
        }
        BI_RLE8 if bits == 8 => Ok(Codec::Rle { bits: 8, pixels: Pixels::Indexed { bits: 8, lut: palette() } }),
        BI_RLE4 if bits == 4 => Ok(Codec::Rle { bits: 4, pixels: Pixels::Indexed { bits: 4, lut: palette() } }),
        c if c == id(b"mrle") && (bits == 8 || bits == 4) => {
            Ok(Codec::Rle { bits: bits as u8, pixels: Pixels::Indexed { bits: bits as u8, lut: palette() } })
        }
        c if c == BI_JPEG || c == id(b"MJPG") || c == id(b"mjpg") || c == id(b"JPEG") || c == id(b"jpeg") => Ok(Codec::Jpeg { samples: 3 }),
        c => {
            let name = if c < 0x100 { format!("compression {c}") } else { format!("'{}'", fourcc(c)) };
            Err(Error::UnsupportedVariant(format!(
                "AVI video codec {name} is not supported (only uncompressed, MSRLE and Motion JPEG); convert the file, e.g. to OME-TIFF with Fiji or bfconvert"
            )))
        }
    }
}

impl Backend for Avi {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let st = &self.streams[series];
        let (x, y0, w, rows) = (region.x as usize, region.y as usize, region.width as usize, region.height as usize);
        let t = plane.t as usize;
        if let Codec::Raw { pixels, stride, top_down } = &st.codec {
            // Only the rows of the region are read from the file.
            let f = st.frames[t];
            let stride = *stride;
            if f.size == 0 {
                // A first frame without data: zeros.
                for r in 0..rows {
                    sink.write_with(r, 0, w, |dst| {
                        dst.fill(0);
                        Ok(())
                    })?;
                }
                return Ok(());
            }
            if (f.size as u64) < (stride * st.height) as u64 {
                return Err(Error::Corrupt(format!("AVI frame {t} shorter than its size")));
            }
            return scratch::with(stride, |row| {
                for r in 0..rows {
                    let y = y0 + r;
                    let stored = if *top_down { y } else { st.height - 1 - y };
                    read_exact_at(&self.file, row, f.offset + (stored * stride) as u64)?;
                    sink.write_with(r, 0, w, |dst| {
                        pixels.convert_row(row, x, dst);
                        Ok(())
                    })?;
                }
                Ok(())
            });
        }
        let frame = self.decode(series, t)?;
        let n = st.samples;
        for r in 0..rows {
            let at = ((y0 + r) * st.width + x) * n;
            let src = frame.get(at..at + w * n).ok_or_else(|| Error::Corrupt("AVI frame smaller than the stream size".into()))?;
            sink.write_with(r, 0, w, |dst| {
                dst.copy_from_slice(src);
                Ok(())
            })?;
        }
        Ok(())
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_tables_are_well_formed() {
        let t = standard_tables();
        assert!(has_huffman_tables(&t));
        // SOI, two DC and two AC DHT segments, EOI.
        assert_eq!(t.len(), 2 + 2 * (4 + 1 + 16 + 12) + 2 * (4 + 1 + 16 + 162) + 2);
    }

    #[test]
    fn parses_chunk_ids() {
        assert_eq!(stream_of(id(b"01dc")), Some(1));
        assert_eq!(stream_of(id(b"ix00")), None);
        assert!(is_video_chunk(id(b"00db")) && !is_video_chunk(id(b"01wb")));
        assert_eq!(fourcc(id(b"MJPG")), "MJPG");
    }

    #[test]
    fn finds_jpeg_segments() {
        let sof = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0, 0, 0xFF, 0xC0, 0x00, 0x0B, 8, 0, 4, 0, 4, 1, 1, 0x11, 0, 0xFF, 0xDA];
        assert!(!has_huffman_tables(&sof));
        assert_eq!(jpeg_components(&sof), Some(1));
    }
}

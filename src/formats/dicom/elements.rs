//! # elements
//!
//! DICOM Part 10 files: the file meta information and the data set as a
//! tree of data elements, without loading large values. Encapsulated pixel
//! data is kept as the positions of its fragments.
//!
//! Provenance: DICOM PS3.5 (data structures and encoding: 7.1 data
//! elements, 7.5 nesting, A.4 encapsulation), PS3.10 (file format) in
//! `doc/dicom/`. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use crate::{Error, Result};

const UNDEFINED: u32 = 0xFFFF_FFFF;
pub(super) const ITEM: u32 = 0xFFFE_E000;
const ITEM_END: u32 = 0xFFFE_E00D;
const SEQUENCE_END: u32 = 0xFFFE_E0DD;
pub(super) const PIXEL_DATA: u32 = 0x7FE0_0010;
/// Values up to this size are read into memory.
const MAX_INLINE: u32 = 1 << 20;
/// Deepest sequence nesting followed, against corrupt files.
const MAX_DEPTH: usize = 16;
/// Most elements read, against corrupt files.
const MAX_ELEMENTS: usize = 1 << 24;

/// Tags read as sequences in implicit VR (where the VR is not encoded).
const SEQUENCES: &[u32] = &[
    0x5200_9229, // Shared Functional Groups
    0x5200_9230, // Per-frame Functional Groups
    0x0048_021A, // Plane Position (Slide)
    0x0028_9110, // Pixel Measures
    0x0048_0105, // Optical Path
    0x0022_0016, // Illumination Type Code
    0x0048_0207, // Optical Path Identification
    0x0020_9111, // Frame Content
    0x0020_9113, // Plane Position (Patient)
    0x0048_0008, // Total Pixel Matrix Origin
    0x0008_1115, // Referenced Series
    0x0008_1140, // Referenced Image
    0x0040_0560, // Specimen Description
];

/// VRs of the binary numeric tags read here, for implicit VR data sets.
fn implicit_vr(tag: u32) -> Option<[u8; 2]> {
    match tag {
        0x0028_0002 | 0x0028_0006 | 0x0028_0010 | 0x0028_0011 | 0x0028_0100 | 0x0028_0101 | 0x0028_0102 | 0x0028_0103 => Some(*b"US"),
        0x0048_0006 | 0x0048_0007 | 0x0048_0302 | 0x0048_0303 | 0x0002_0000 => Some(*b"UL"),
        0x0048_021E | 0x0048_021F => Some(*b"SL"),
        0x0022_0055 | 0x0022_0057 => Some(*b"FL"),
        _ => None,
    }
}

/// A data element value.
#[derive(Debug, Clone)]
pub(super) enum Value {
    /// Bytes of a small value.
    Data(Vec<u8>),
    /// Position and length of a large value in the file.
    Far(u64, u64),
    /// Items of a sequence.
    Seq(Vec<Dataset>),
    /// Encapsulated pixel data: the basic offset table and the fragments
    /// (position, length).
    Fragments(Vec<u32>, Vec<(u64, u64)>),
}

/// A data element.
#[derive(Debug, Clone)]
pub(super) struct Element {
    pub tag: u32,
    pub vr: [u8; 2],
    pub value: Value,
}

/// A data set: elements in file order.
#[derive(Debug, Clone, Default)]
pub(super) struct Dataset(pub Vec<Element>);

impl Dataset {
    pub fn get(&self, tag: u32) -> Option<&Element> {
        self.0.iter().find(|e| e.tag == tag)
    }

    fn data(&self, tag: u32) -> Option<&[u8]> {
        match &self.get(tag)?.value {
            Value::Data(d) => Some(d),
            _ => None,
        }
    }

    /// Text value (trailing spaces and NULs removed); all values joined by `\`.
    pub fn text(&self, tag: u32) -> Option<String> {
        let d = self.data(tag)?;
        let s = String::from_utf8_lossy(d).trim_end_matches(['\0', ' ']).trim_start().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// Text values split at `\`.
    pub fn texts(&self, tag: u32) -> Vec<String> {
        self.text(tag).map(|t| t.split('\\').map(|v| v.trim().to_string()).collect()).unwrap_or_default()
    }

    /// Numbers of a binary (US, UL, SS, SL, FL, FD) or decimal-string value.
    pub fn numbers(&self, tag: u32) -> Vec<f64> {
        let Some(e) = self.get(tag) else { return Vec::new() };
        let Value::Data(d) = &e.value else { return Vec::new() };
        match &e.vr {
            b"US" => d.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]]) as f64).collect(),
            b"SS" => d.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f64).collect(),
            b"UL" => d.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as f64).collect(),
            b"SL" => d.chunks_exact(4).map(|b| i32::from_le_bytes(b.try_into().expect("4 bytes")) as f64).collect(),
            b"FL" => d.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().expect("4 bytes")) as f64).collect(),
            b"FD" => d.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes"))).collect(),
            _ => self.texts(tag).iter().filter_map(|v| v.parse().ok()).collect(),
        }
    }

    pub fn number(&self, tag: u32) -> Option<f64> {
        self.numbers(tag).into_iter().next()
    }

    /// Items of a sequence.
    pub fn items(&self, tag: u32) -> &[Dataset] {
        match self.get(tag).map(|e| &e.value) {
            Some(Value::Seq(items)) => items,
            _ => &[],
        }
    }

    /// First item of a sequence.
    pub fn item(&self, tag: u32) -> Option<&Dataset> {
        self.items(tag).first()
    }
}

/// VRs with a 32-bit length after two reserved bytes in explicit VR.
fn long_vr(vr: &[u8; 2]) -> bool {
    matches!(vr, b"OB" | b"OD" | b"OF" | b"OL" | b"OV" | b"OW" | b"SQ" | b"SV" | b"UC" | b"UN" | b"UR" | b"UT" | b"UV")
}

/// A reader keeping its position.
struct Cursor {
    r: BufReader<File>,
    pos: u64,
    len: u64,
    elements: usize,
}

impl Cursor {
    fn read(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.pos + buf.len() as u64 > self.len {
            return Err(Error::Corrupt(format!("DICOM data truncated at {}", self.pos)));
        }
        self.r.read_exact(buf)?;
        self.pos += buf.len() as u64;
        Ok(())
    }

    fn u16(&mut self) -> Result<u16> {
        let mut b = [0u8; 2];
        self.read(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn u32(&mut self) -> Result<u32> {
        let mut b = [0u8; 4];
        self.read(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn tag(&mut self) -> Result<u32> {
        let g = self.u16()? as u32;
        Ok(g << 16 | self.u16()? as u32)
    }

    fn skip(&mut self, n: u64) -> Result<()> {
        if self.pos + n > self.len {
            return Err(Error::Corrupt(format!("DICOM value beyond the end at {}", self.pos)));
        }
        self.r.seek_relative(n as i64)?;
        self.pos += n;
        Ok(())
    }

    /// Reads elements until `end` (exclusive) or a delimiter, for an item
    /// or data set of undefined length when `end` is `None`.
    fn dataset(&mut self, explicit: bool, end: Option<u64>, depth: usize, stop_at_pixels: bool) -> Result<Dataset> {
        if depth > MAX_DEPTH {
            return Err(Error::Corrupt("DICOM sequences nested too deeply".into()));
        }
        let mut out = Vec::new();
        loop {
            if end.is_some_and(|e| self.pos >= e) || (end.is_none() && depth == 0 && self.pos >= self.len) {
                break;
            }
            self.elements += 1;
            if self.elements > MAX_ELEMENTS {
                return Err(Error::Corrupt("DICOM data set too large".into()));
            }
            let tag = self.tag()?;
            if tag == ITEM_END {
                self.u32()?;
                break;
            }
            // Group 0002 is always explicit VR little endian.
            let explicit = explicit || tag >> 16 == 0x0002;
            let (vr, len) = if explicit {
                let mut vr = [0u8; 2];
                self.read(&mut vr)?;
                if long_vr(&vr) {
                    self.u16()?;
                    (vr, self.u32()?)
                } else {
                    (vr, self.u16()? as u32)
                }
            } else {
                let len = self.u32()?;
                let vr = if tag == PIXEL_DATA {
                    *b"OB"
                } else if SEQUENCES.contains(&tag) || len == UNDEFINED {
                    *b"SQ"
                } else {
                    implicit_vr(tag).unwrap_or(*b"UN")
                };
                (vr, len)
            };
            let value = if tag == PIXEL_DATA && len == UNDEFINED {
                self.fragments()?
            } else if &vr == b"SQ" || (len == UNDEFINED && &vr == b"UN") {
                // Undefined-length UN contents are implicit VR.
                let explicit = explicit && &vr != b"UN";
                Value::Seq(self.sequence(explicit, len, depth)?)
            } else if len == UNDEFINED {
                return Err(Error::Corrupt(format!("DICOM element {tag:08X} of undefined length")));
            } else if len > MAX_INLINE || tag == PIXEL_DATA {
                let at = self.pos;
                self.skip(len as u64)?;
                Value::Far(at, len as u64)
            } else {
                let mut b = vec![0u8; len as usize];
                self.read(&mut b)?;
                Value::Data(b)
            };
            out.push(Element { tag, vr, value });
            if tag == PIXEL_DATA && stop_at_pixels {
                break;
            }
        }
        Ok(Dataset(out))
    }

    fn sequence(&mut self, explicit: bool, len: u32, depth: usize) -> Result<Vec<Dataset>> {
        let end = (len != UNDEFINED).then(|| self.pos + len as u64);
        let mut items = Vec::new();
        loop {
            if end.is_some_and(|e| self.pos >= e) {
                break;
            }
            let tag = self.tag()?;
            let item_len = self.u32()?;
            match tag {
                SEQUENCE_END => break,
                ITEM => {
                    let item_end = (item_len != UNDEFINED).then(|| self.pos + item_len as u64);
                    items.push(self.dataset(explicit, item_end, depth + 1, false)?);
                }
                t => return Err(Error::Corrupt(format!("DICOM sequence with element {t:08X} instead of an item"))),
            }
        }
        Ok(items)
    }

    /// Basic offset table and fragment positions of encapsulated pixel data.
    fn fragments(&mut self) -> Result<Value> {
        let mut bot = Vec::new();
        let mut fragments = Vec::new();
        let mut first = true;
        loop {
            let tag = self.tag()?;
            let len = self.u32()?;
            match tag {
                SEQUENCE_END => break,
                ITEM if first => {
                    if len as u64 > self.len - self.pos || len % 4 != 0 {
                        return Err(Error::Corrupt("DICOM basic offset table beyond the end".into()));
                    }
                    let mut b = vec![0u8; len as usize];
                    self.read(&mut b)?;
                    bot = b.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().expect("4 bytes"))).collect();
                }
                ITEM => {
                    fragments.push((self.pos, len as u64));
                    self.skip(len as u64)?;
                }
                t => return Err(Error::Corrupt(format!("DICOM pixel data with element {t:08X} instead of an item"))),
            }
            first = false;
            if fragments.len() > MAX_ELEMENTS {
                return Err(Error::Corrupt("DICOM pixel data with too many fragments".into()));
            }
        }
        Ok(Value::Fragments(bot, fragments))
    }
}

/// Transfer syntax and data set of a DICOM Part 10 file.
#[derive(Debug)]
pub(super) struct Part10 {
    pub transfer_syntax: String,
    pub dataset: Dataset,
}

/// Returns `true` if `head` has the Part 10 preamble and `DICM` prefix.
pub(super) fn matches(head: &[u8]) -> bool {
    head.len() >= 132 && &head[128..132] == b"DICM"
}

/// Reads the meta information and the data set (up to and including the
/// pixel data element) of a Part 10 file.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid encodings and
/// [`Error::UnsupportedVariant`] for big-endian and deflated transfer
/// syntaxes.
pub(super) fn read(file: &File) -> Result<Part10> {
    let len = file.metadata()?.len();
    let mut r = BufReader::with_capacity(1 << 16, file.try_clone()?);
    r.seek(SeekFrom::Start(0))?;
    let mut c = Cursor { r, pos: 0, len, elements: 0 };
    let mut head = [0u8; 132];
    c.read(&mut head)?;
    if !matches(&head) {
        return Err(Error::Corrupt("not a DICOM Part 10 file".into()));
    }
    // File meta information: group 0002, explicit VR; its length element first.
    let meta_start = c.pos;
    let first = c.dataset(true, Some(meta_start + 12), 0, false)?;
    let group_len = first.number(0x0002_0000).ok_or_else(|| Error::Corrupt("DICOM without meta information length".into()))?;
    let meta = c.dataset(true, Some(c.pos + group_len as u64), 0, false)?;
    let ts = meta.text(0x0002_0010).ok_or_else(|| Error::Corrupt("DICOM without transfer syntax".into()))?;
    let explicit = match ts.as_str() {
        "1.2.840.10008.1.2" => false,
        "1.2.840.10008.1.2.2" => return Err(Error::UnsupportedVariant("DICOM big-endian transfer syntax".into())),
        "1.2.840.10008.1.2.1.99" => return Err(Error::UnsupportedVariant("DICOM deflated transfer syntax".into())),
        _ => true,
    };
    let dataset = c.dataset(explicit, None, 0, true)?;
    Ok(Part10 { transfer_syntax: ts, dataset })
}

//! # dir
//!
//! CZI segment structure: file header, subblock directory, attachment
//! directory and subblock headers.
//!
//! Provenance: OpenReadout CZI format notes (MIT OR Apache-2.0) in
//! `doc/zeiss-czi/`; ZEISS libCZI documentation pages (MIT, docs only);
//! own analysis of sample files. No GPL/LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;

use crate::io::read_exact_at;
use crate::{Error, Result};

/// Length of every segment header: id, allocated size, used size.
pub(super) const SEGMENT_HEADER: u64 = 32;
/// Largest directory or metadata segment read, against corrupt files.
const MAX_SEGMENT: u64 = 1 << 30;

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

/// A CZI file or a CZI embedded in an attachment: all positions are
/// relative to `base`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Span {
    pub base: u64,
    pub len: u64,
}

impl Span {
    /// Reads `buf.len()` bytes at the relative position `at`.
    pub fn read(&self, file: &File, buf: &mut [u8], at: u64) -> Result<()> {
        if at.checked_add(buf.len() as u64).is_none_or(|e| e > self.len) {
            return Err(Error::Corrupt(format!("CZI read of {} bytes at {at} beyond the end", buf.len())));
        }
        read_exact_at(file, buf, self.base + at)
    }

    /// Payload of the segment at `at` if its id is `id`.
    pub fn segment(&self, file: &File, at: u64, id: &[u8]) -> Result<Vec<u8>> {
        let mut head = [0u8; SEGMENT_HEADER as usize];
        self.read(file, &mut head, at)?;
        if !head.starts_with(id) {
            return Err(Error::Corrupt(format!("CZI segment at {at} is not {}", String::from_utf8_lossy(id))));
        }
        // Some writers leave the used size 0; the allocated size bounds it.
        let used = match u64_at(&head, 24) {
            0 => u64_at(&head, 16),
            n => n,
        };
        if used > MAX_SEGMENT {
            return Err(Error::Corrupt(format!("CZI segment at {at} too large")));
        }
        let used = used.min(self.len.saturating_sub(at + SEGMENT_HEADER));
        let mut buf = vec![0u8; used as usize];
        self.read(file, &mut buf, at + SEGMENT_HEADER)?;
        Ok(buf)
    }
}

/// Fields of the `ZISRAWFILE` segment.
#[derive(Debug, Clone, Copy)]
pub(super) struct FileHeader {
    /// GUID of the master file; equal in all parts of a document.
    pub primary_guid: [u8; 16],
    pub file_part: u32,
    pub directory: u64,
    pub metadata: u64,
    pub attachments: u64,
}

pub(super) fn file_header(file: &File, span: Span) -> Result<FileHeader> {
    let mut b = [0u8; 112];
    span.read(file, &mut b, 0)?;
    if !b.starts_with(b"ZISRAWFILE") {
        return Err(Error::Corrupt("not a CZI file".into()));
    }
    let p = &b[32..];
    Ok(FileHeader {
        primary_guid: p[16..32].try_into().expect("16 bytes"),
        file_part: u32_at(p, 48), directory: u64_at(p, 52), metadata: u64_at(p, 60), attachments: u64_at(p, 72) })
}

/// Dimension of a directory entry that is not X, Y, Z, C, T, S or M.
pub(super) const EXTRA_DIMS: &[u8] = b"BHIRV";

/// One subblock of the directory.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Entry {
    pub pixel_type: u32,
    pub position: u64,
    pub file_part: u32,
    pub compression: u32,
    /// Start and logical size of X and Y in level-0 pixels.
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Stored size; smaller than the logical size on pyramid levels.
    pub stored_w: u32,
    pub stored_h: u32,
    pub z: i32,
    pub c: i32,
    pub t: i32,
    pub s: i32,
    pub m: i32,
    /// Starts of B, H, I, R, V.
    pub extra: [i32; 5],
}

/// Parses one directory entry (schema `DV`) at the start of `b`.
///
/// Returns the entry and its length.
pub(super) fn entry(b: &[u8]) -> Result<(Entry, usize)> {
    let bad = || Error::Corrupt("CZI directory entry truncated".into());
    if b.len() < 32 || &b[..2] != b"DV" {
        return Err(Error::Corrupt("CZI directory entry without DV schema".into()));
    }
    let count = u32_at(b, 28) as usize;
    let len = count.checked_mul(20).and_then(|n| n.checked_add(32)).ok_or_else(bad)?;
    if b.len() < len {
        return Err(bad());
    }
    let mut e = Entry {
        pixel_type: u32_at(b, 2),
        position: u64_at(b, 6),
        file_part: u32_at(b, 14),
        compression: u32_at(b, 18),
        w: 1,
        h: 1,
        stored_w: 1,
        stored_h: 1,
        ..Default::default()
    };
    for d in b[32..len].chunks_exact(20) {
        let start = i32_at(d, 4);
        let size = u32_at(d, 8);
        let stored = u32_at(d, 16);
        match d[0] {
            b'X' => (e.x, e.w, e.stored_w) = (start, size, stored),
            b'Y' => (e.y, e.h, e.stored_h) = (start, size, stored),
            b'Z' => e.z = start,
            b'C' => e.c = start,
            b'T' => e.t = start,
            b'S' => e.s = start,
            b'M' => e.m = start,
            k => {
                if let Some(i) = EXTRA_DIMS.iter().position(|&x| x == k) {
                    e.extra[i] = start;
                }
            }
        }
    }
    Ok((e, len))
}

/// All entries of the subblock directory.
pub(super) fn directory(file: &File, span: Span, at: u64) -> Result<Vec<Entry>> {
    let data = span.segment(file, at, b"ZISRAWDIRECTORY")?;
    if data.len() < 128 {
        return Err(Error::Corrupt("CZI directory truncated".into()));
    }
    let count = u32_at(&data, 0) as usize;
    // An entry has at least 32 bytes.
    let mut entries = Vec::with_capacity(count.min(data.len() / 32));
    let mut pos = 128;
    for _ in 0..count {
        let (e, len) = entry(&data[pos.min(data.len())..])?;
        entries.push(e);
        pos += len;
    }
    Ok(entries)
}

/// UTF-8 XML of the `ZISRAWMETADATA` segment.
pub(super) fn metadata_xml(file: &File, span: Span, at: u64) -> Result<String> {
    let data = span.segment(file, at, b"ZISRAWMETADATA")?;
    let size = data.get(..4).map(|b| u32_at(b, 0) as usize).ok_or_else(|| Error::Corrupt("CZI metadata truncated".into()))?;
    let xml = data.get(256..256 + size).or_else(|| data.get(256..)).unwrap_or_default();
    Ok(String::from_utf8_lossy(xml).trim_end_matches('\0').to_string())
}

/// An attachment of the attachment directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Attachment {
    /// Position of its `ZISRAWATTACH` segment.
    pub position: u64,
    pub file_part: u32,
    pub content_type: String,
    pub name: String,
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

/// Parses one attachment entry (schema `A1`, 128 bytes).
pub(super) fn attachment_entry(e: &[u8]) -> Option<Attachment> {
    (e.len() >= 128 && &e[..2] == b"A1").then(|| Attachment {
        position: u64_at(e, 12),
        file_part: u32_at(e, 20),
        content_type: text(&e[40..48]),
        name: text(&e[48..128]),
    })
}

pub(super) fn attachments(file: &File, span: Span, at: u64) -> Result<Vec<Attachment>> {
    let data = span.segment(file, at, b"ZISRAWATTDIR")?;
    let count = data.get(..4).map(|b| u32_at(b, 0) as usize).unwrap_or(0);
    let list = data.get(256..).unwrap_or_default();
    Ok(list
        .chunks_exact(128)
        .take(count)
        .filter_map(attachment_entry)
        .collect())
}

/// Position and length of an attachment's payload.
pub(super) fn attachment_data(file: &File, span: Span, at: u64) -> Result<(u64, u64)> {
    let mut head = [0u8; 48];
    span.read(file, &mut head, at)?;
    if !head.starts_with(b"ZISRAWATTACH") {
        return Err(Error::Corrupt(format!("CZI attachment at {at} without segment id")));
    }
    let start = at + SEGMENT_HEADER + 256;
    let len = u32_at(&head, 32) as u64;
    if start.checked_add(len).is_none_or(|e| e > span.len) {
        return Err(Error::Corrupt(format!("CZI attachment at {at} beyond the end")));
    }
    Ok((start, len))
}

/// Sizes of a subblock segment's parts, from its header.
#[derive(Debug, Clone, Copy)]
pub(super) struct SubBlock {
    /// Position of the pixel data.
    pub data: u64,
    pub data_len: u64,
    /// Position and length of the attachment (valid-pixel mask).
    pub attachment: u64,
    pub attachment_len: u64,
}

pub(super) fn subblock(file: &File, span: Span, at: u64) -> Result<SubBlock> {
    let mut b = [0u8; 512];
    let n = (span.len.saturating_sub(at) as usize).min(b.len());
    span.read(file, &mut b[..n], at)?;
    let b = &b[..n];
    if n < 80 || !b.starts_with(b"ZISRAWSUBBLOCK") {
        return Err(Error::Corrupt(format!("CZI subblock at {at} without segment id")));
    }
    let p = &b[32..];
    let (meta_len, attachment_len, data_len) = (u32_at(p, 0) as u64, u32_at(p, 4) as u64, u64_at(p, 8));
    let dims = u32_at(p, 16 + 28) as u64;
    let header = (16 + 32 + dims * 20).max(256);
    let data = at + SEGMENT_HEADER + header + meta_len;
    let attachment = data.checked_add(data_len).ok_or_else(|| Error::Corrupt("CZI subblock size overflow".into()))?;
    if attachment.checked_add(attachment_len).is_none_or(|e| e > span.len) {
        return Err(Error::Corrupt(format!("CZI subblock at {at} extends beyond the end")));
    }
    Ok(SubBlock { data, data_len, attachment, attachment_len })
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn dim(name: u8, start: i32, size: u32, stored: u32) -> Vec<u8> {
        let mut d = vec![name, 0, 0, 0];
        d.extend_from_slice(&start.to_le_bytes());
        d.extend_from_slice(&size.to_le_bytes());
        d.extend_from_slice(&0f32.to_le_bytes());
        d.extend_from_slice(&stored.to_le_bytes());
        d
    }

    #[test]
    fn parses_directory_entries() {
        let mut b = b"DV".to_vec();
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&4096u64.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&5u32.to_le_bytes());
        b.extend_from_slice(&[2, 0, 0, 0, 0, 0]);
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend(dim(b'X', -10, 1024, 512));
        b.extend(dim(b'Y', 20, 512, 256));
        b.extend(dim(b'C', 1, 1, 1));
        b.extend(dim(b'H', 3, 1, 1));
        let (e, len) = entry(&b).unwrap();
        assert_eq!(len, 32 + 80);
        assert_eq!((e.pixel_type, e.position, e.compression), (1, 4096, 5));
        assert_eq!((e.x, e.y, e.w, e.h, e.stored_w, e.stored_h), (-10, 20, 1024, 512, 512, 256));
        assert_eq!((e.c, e.extra[1]), (1, 3));
        assert!(entry(&b[..100]).is_err());
    }
}

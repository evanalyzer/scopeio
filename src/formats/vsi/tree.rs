//! # tree
//!
//! The tagged record tree of a `.vsi` file: nested volumes of fields that
//! hold all metadata.
//!
//! Provenance: OpenReadout VSI format notes and bioio-vsi format notes
//! (`doc/olympus-vsi/`, both written from sample files and permissively
//! licensed readers); own analysis of sample files. No GPL code consulted.
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

/// Magic of a volume header.
const VOLUME_MAGIC: u16 = 21321;
/// The value is stored in the size word.
const FLAG_INLINE: u32 = 0x4000_0000;
/// Extended field: nested volume (types 0-2) or embedded TIFF (type 10).
const FLAG_EXTENDED: u32 = 0x1000_0000;
/// An index word follows the field header.
const FLAG_INDEXED: u32 = 0x0800_0000;
/// Largest payload read eagerly; larger ones (embedded images, blobs) are
/// skipped.
const MAX_PAYLOAD: u32 = 1 << 20;
/// Deepest nesting accepted, against corrupt files.
const MAX_DEPTH: usize = 32;

/// Tag of the value inside a value container.
pub(super) const VALUE: u32 = 0x1000_0002;

/// One field of the tree.
#[derive(Debug, Clone)]
pub(super) struct Field {
    pub tag: u32,
    pub index: Option<u32>,
    /// Value type (low 24 bits of the type word).
    pub kind: u32,
    pub inline: Option<u32>,
    pub data: Vec<u8>,
    pub children: Vec<Field>,
}

impl Field {
    /// First child with `tag`.
    pub fn get(&self, tag: u32) -> Option<&Field> {
        self.children.iter().find(|c| c.tag == tag)
    }

    /// All children with `tag`.
    pub fn all(&self, tag: u32) -> impl Iterator<Item = &Field> {
        self.children.iter().filter(move |c| c.tag == tag)
    }

    /// The field itself, or the value inside it if it is a value container.
    fn value(&self) -> &Field {
        self.get(VALUE).unwrap_or(self)
    }

    /// Integers of the value: inline, i32/u32/i64 scalars or i32 arrays.
    pub fn ints(&self) -> Vec<i64> {
        let v = self.value();
        if let Some(i) = v.inline {
            return vec![match v.kind {
                5 => i as i32 as i64,
                _ => i as i64,
            }];
        }
        match v.kind {
            7 if v.data.len() == 8 => vec![i64::from_le_bytes(v.data[..8].try_into().expect("8 bytes"))],
            _ => v.data.chunks_exact(4).map(|b| i32::from_le_bytes(b.try_into().expect("4 bytes")) as i64).collect(),
        }
    }

    /// First integer of the value.
    pub fn int(&self) -> Option<i64> {
        self.ints().first().copied()
    }

    /// Doubles of the value (kinds 10 and the double vectors).
    pub fn floats(&self) -> Vec<f64> {
        let v = self.value();
        match v.kind {
            5 | 6 | 7 | 14 => v.ints().into_iter().map(|i| i as f64).collect(),
            _ if v.inline.is_some() => Vec::new(),
            _ => v.data.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes"))).collect(),
        }
    }

    /// First double of the value.
    pub fn float(&self) -> Option<f64> {
        self.floats().first().copied()
    }

    /// Text of the value: UTF-16 for the string kinds, else UTF-8.
    pub fn text(&self) -> Option<String> {
        let v = self.value();
        if v.data.is_empty() {
            return None;
        }
        let s = match v.kind {
            13 | 8192 | 8194 => {
                let units: Vec<u16> = v.data.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
                String::from_utf16_lossy(&units)
            }
            _ => String::from_utf8_lossy(&v.data).into_owned(),
        };
        let s = s.trim_end_matches('\0').trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}

/// Parses the tree starting at `offset` (8 in `.vsi` files).
///
/// # Errors
///
/// Returns [`Error::Corrupt`] if the root volume header is invalid.
pub(super) fn parse(file: &File, len: u64, offset: u64) -> Result<Vec<Field>> {
    volume(file, len, offset, 0)
}

fn volume(file: &File, len: u64, offset: u64, depth: usize) -> Result<Vec<Field>> {
    if depth > MAX_DEPTH {
        return Err(Error::Corrupt("VSI record tree nested too deeply".into()));
    }
    let mut head = [0u8; 24];
    if offset + 24 > len {
        return Err(Error::Corrupt("VSI record volume beyond the end of the file".into()));
    }
    read_exact_at(file, &mut head, offset)?;
    if u16::from_le_bytes([head[2], head[3]]) != VOLUME_MAGIC {
        return Err(Error::Corrupt(format!("VSI record volume without signature at {offset}")));
    }
    let first = u64::from_le_bytes(head[8..16].try_into().expect("8 bytes"));
    let count = u32::from_le_bytes(head[16..20].try_into().expect("4 bytes")) & 0x0FFF_FFFF;
    let mut fields = Vec::with_capacity(count.min(4096) as usize);
    let mut next = first;
    while next != 0 && fields.len() < count as usize {
        let at = offset + next;
        if at + 20 > len {
            break;
        }
        let mut h = [0u8; 20];
        read_exact_at(file, &mut h, at)?;
        let word = |i: usize| u32::from_le_bytes(h[i..i + 4].try_into().expect("4 bytes"));
        let (typ, tag, link, size) = (word(0), word(4), word(8), word(12));
        let indexed = typ & FLAG_INDEXED != 0;
        let header = if indexed { 20 } else { 16 };
        let kind = typ & 0x00FF_FFFF;
        let mut field = Field {
            tag,
            index: indexed.then(|| word(16)),
            kind,
            inline: None,
            data: Vec::new(),
            children: Vec::new(),
        };
        if typ & FLAG_EXTENDED != 0 && kind <= 2 {
            // Nested volumes may be empty or damaged; keep what was read.
            field.children = volume(file, len, at + header, depth + 1).unwrap_or_default();
        } else if typ & FLAG_INLINE != 0 {
            field.inline = Some(size);
        } else if typ & FLAG_EXTENDED == 0 && size <= MAX_PAYLOAD && at + header + size as u64 <= len {
            field.data = vec![0u8; size as usize];
            read_exact_at(file, &mut field.data, at + header)?;
        }
        fields.push(field);
        // Links point forward; anything else ends the chain.
        if link == 0 || link as u64 <= next {
            break;
        }
        next = link as u64;
    }
    Ok(fields)
}

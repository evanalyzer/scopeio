//! # meta
//!
//! ND2 metadata encodings: the binary "lite variant" (LV) of version 3 files
//! and the XML variants of version 2 files, both decoded into one ordered
//! [`Value`] tree.
//!
//! Provenance: OpenReadout ND2 format notes (MIT OR Apache-2.0) and the
//! nd2-rs data structure notes (MIT, aligned with the BSD-3 `nd2` Python
//! package) in `doc/nikon-nd2/`; own analysis of sample files. No GPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Error, Result, codecs};

/// Deepest nesting accepted, against corrupt files.
const MAX_DEPTH: usize = 64;
/// Upper bound of decompressed metadata.
const MAX_INFLATED: usize = 256 << 20;

/// A decoded metadata value.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::formats) enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    /// Named children in file order; names may repeat.
    Map(Vec<(String, Value)>),
}

impl Value {
    /// First child named `key`, looking through single-child wrappers
    /// (`""`, `i0000000000`, `SLx...` roots, `no_name`).
    pub fn get(&self, key: &str) -> Option<&Value> {
        let Value::Map(items) = self else { return None };
        if let Some((_, v)) = items.iter().find(|(k, _)| k == key) {
            return Some(v);
        }
        match items.as_slice() {
            [(_, inner)] => inner.get(key),
            _ => None,
        }
    }

    /// Children of a map in order.
    pub fn items(&self) -> &[(String, Value)] {
        match self {
            Value::Map(items) => items,
            _ => &[],
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            Value::Bool(b) => Some(*b as i64),
            Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Integer child.
    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key)?.as_i64()
    }

    /// Float child.
    pub fn float(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_f64()
    }

    /// Non-empty text child.
    pub fn text(&self, key: &str) -> Option<String> {
        self.get(key)?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
    }
}

/// Decodes an LV (version 3) metadata chunk.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for truncated or invalid data.
pub(in crate::formats) fn decode_lv(data: &[u8]) -> Result<Value> {
    let mut items = Vec::new();
    let mut pos = 0;
    lv_items(data, &mut pos, usize::MAX, &mut items, 0)?;
    Ok(Value::Map(items))
}

fn take<'a>(data: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8]> {
    let end = pos.checked_add(n).filter(|&e| e <= data.len()).ok_or_else(|| Error::Corrupt("ND2 metadata truncated".into()))?;
    let s = &data[*pos..end];
    *pos = end;
    Ok(s)
}

fn utf16z(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).take_while(|&u| u != 0).collect();
    String::from_utf16_lossy(&units)
}

/// Decodes up to `count` items at `pos`.
fn lv_items(data: &[u8], pos: &mut usize, count: usize, out: &mut Vec<(String, Value)>, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Error::Corrupt("ND2 metadata nested too deeply".into()));
    }
    let mut n = 0;
    while n < count && *pos < data.len() {
        let typ = take(data, pos, 1)?[0];
        let name_units = take(data, pos, 1)?[0] as usize;
        let name = utf16z(take(data, pos, name_units * 2)?);
        let value = match typ {
            1 => Value::Bool(take(data, pos, 1)?[0] != 0),
            2 => Value::Int(i32::from_le_bytes(take(data, pos, 4)?.try_into().expect("4 bytes")) as i64),
            3 => Value::Int(u32::from_le_bytes(take(data, pos, 4)?.try_into().expect("4 bytes")) as i64),
            4 => Value::Int(i64::from_le_bytes(take(data, pos, 8)?.try_into().expect("8 bytes"))),
            5 | 7 => Value::Int(u64::from_le_bytes(take(data, pos, 8)?.try_into().expect("8 bytes")) as i64),
            6 => Value::Float(f64::from_le_bytes(take(data, pos, 8)?.try_into().expect("8 bytes"))),
            8 => {
                // UTF-16 text up to and including a zero unit.
                let start = *pos;
                while *pos + 1 < data.len() && (data[*pos] != 0 || data[*pos + 1] != 0) {
                    *pos += 2;
                }
                let s = utf16z(&data[start..*pos]);
                *pos = (*pos + 2).min(data.len());
                Value::Text(s)
            }
            9 => {
                let len = u64::from_le_bytes(take(data, pos, 8)?.try_into().expect("8 bytes")) as usize;
                let bytes = take(data, pos, len)?;
                // Byte arrays may hold a nested LV structure.
                match looks_like_lv(bytes).then(|| decode_lv(bytes)).and_then(|r| r.ok()) {
                    Some(v) => v,
                    None => Value::Bytes(bytes.to_vec()),
                }
            }
            11 => {
                let children = u32::from_le_bytes(take(data, pos, 4)?.try_into().expect("4 bytes")) as usize;
                let _len = take(data, pos, 8)?;
                let mut items = Vec::new();
                lv_items(data, pos, children, &mut items, depth + 1)?;
                // Offsets of the children follow; skip them.
                let skip = children.saturating_mul(8).min(data.len() - *pos);
                *pos += skip;
                Value::Map(items)
            }
            76 => {
                // Compressed block: 10 bytes, then a zlib stream of more items
                // that belong to the enclosing level.
                let stream = data.get(*pos + 10..).ok_or_else(|| Error::Corrupt("ND2 compressed block truncated".into()))?;
                let inflated = codecs::inflate_vec(stream, MAX_INFLATED)?;
                let mut p = 0;
                lv_items(&inflated, &mut p, usize::MAX, out, depth + 1)?;
                *pos = data.len();
                n += 1;
                continue;
            }
            t => return Err(Error::Corrupt(format!("ND2 metadata item type {t}"))),
        };
        out.push((name, value));
        n += 1;
    }
    Ok(())
}

/// Returns `true` if `b` plausibly starts with an LV item: a known type and
/// a name of more than one unit (validity masks are plain bytes).
fn looks_like_lv(b: &[u8]) -> bool {
    b.len() >= 4 && matches!(b[0], 1..=9 | 11) && b[1] > 1 && (b[1] as usize) * 2 + 2 <= b.len()
}

/// Decodes an XML-variant (version 2) metadata chunk.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid XML.
pub(super) fn decode_xml(data: &[u8]) -> Result<Value> {
    let text = String::from_utf8_lossy(data);
    let text = text.trim_end_matches('\0');
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(text, options).map_err(|e| Error::Corrupt(format!("ND2 XML: {e}")))?;
    Ok(xml_node(doc.root_element()))
}

fn xml_node(n: roxmltree::Node) -> Value {
    let value = n.attribute("value");
    let runtype = n.attribute("runtype").unwrap_or("");
    let children: Vec<(String, Value)> =
        n.children().filter(|c| c.is_element()).map(|c| (c.tag_name().name().to_string(), xml_node(c))).collect();
    if !children.is_empty() || value.is_none() {
        return Value::Map(children);
    }
    let v = value.unwrap_or("");
    match runtype {
        "lx_uint32" | "lx_int32" | "lx_uint64" | "lx_int64" | "lx_uint16" | "lx_int16" | "lx_uint8" | "lx_int8" => {
            v.trim().parse::<i64>().map(Value::Int).unwrap_or_else(|_| Value::Text(v.to_string()))
        }
        "double" | "float" => v.trim().parse::<f64>().map(Value::Float).unwrap_or_else(|_| Value::Text(v.to_string())),
        "bool" => Value::Bool(v.trim() == "true" || v.trim() == "1"),
        "CLxByteArray" => Value::Bytes(base64(v)),
        _ => Value::Text(v.to_string()),
    }
}

/// Decodes standard base64, ignoring whitespace and invalid characters.
fn base64(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn item(typ: u8, name: &str, value: &[u8]) -> Vec<u8> {
        let mut v = vec![typ, name.len() as u8 + 1];
        for u in name.encode_utf16().chain([0]) {
            v.extend_from_slice(&u.to_le_bytes());
        }
        v.extend_from_slice(value);
        v
    }

    #[test]
    fn lv_scalars_text_and_levels() {
        let mut child = item(3, "uiWidth", &1024u32.to_le_bytes());
        child.extend(item(8, "sName", &[b'A', 0, b'b', 0, 0, 0]));
        let mut level = 2u32.to_le_bytes().to_vec();
        level.extend_from_slice(&0u64.to_le_bytes());
        level.extend(&child);
        level.extend_from_slice(&[0u8; 16]);
        let mut data = item(11, "SLxImageAttributes", &level);
        data.extend(item(6, "dCalibration", &0.5f64.to_le_bytes()));
        let v = decode_lv(&data).unwrap();
        assert_eq!(v.get("SLxImageAttributes").and_then(|a| a.int("uiWidth")), Some(1024));
        assert_eq!(v.get("SLxImageAttributes").unwrap().text("sName").as_deref(), Some("Ab"));
        assert_eq!(v.float("dCalibration"), Some(0.5));
        // Looking through the single-child wrapper.
        assert_eq!(Value::Map(vec![("".into(), v.clone())]).float("dCalibration"), Some(0.5));
    }

    #[test]
    fn xml_variants() {
        let xml = br#"<?xml version="1.0"?><variant version="1.0"><no_name runtype="CLxListVariant"><uiWidth runtype="lx_uint32" value="696"/><dCal runtype="double" value="0.25"/><sName runtype="CLxStringW" value="x"/><b runtype="CLxByteArray" value="AQID"/></no_name></variant>"#;
        let v = decode_xml(xml).unwrap();
        assert_eq!(v.int("uiWidth"), Some(696));
        assert_eq!(v.float("dCal"), Some(0.25));
        assert_eq!(v.text("sName").as_deref(), Some("x"));
        assert_eq!(v.get("b"), Some(&Value::Bytes(vec![1, 2, 3])));
    }
}

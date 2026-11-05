//! # amira
//!
//! AmiraMesh / Avizo reader for regular lattices (`.am`): scalar and
//! vector fields and label fields on a 2D or 3D `Lattice`.
//!
//! The file is an ASCII header followed by a data section. The first line
//! is `# AmiraMesh` or `# Avizo`, optionally `3D`, the encoding (`BINARY`:
//! big endian, `BINARY-LITTLE-ENDIAN`, `ASCII`) and a version. The header
//! defines arrays (`define Lattice nx ny [nz]`), a `Parameters { ... }`
//! tree (nested groups, `key value...` entries separated by commas or new
//! lines, `&key "base64"` binary attributes) and data declarations
//! `Array { type[n] Name } [=] @k[(codec,size)]` with the types `byte`
//! (unsigned), `sbyte`, `short`, `ushort`, `int`, `uint`, `float` and
//! `double` and the codecs `HxZip` (zlib) and `HxByteRLE`. The data section
//! (after `# Data section follows`, or at the first `@k` line of ASCII
//! files) holds one block per declaration, each after its `@k` line:
//! binary blocks are as long as their declaration (or the compressed size),
//! ASCII blocks are numbers separated by white space. Samples run x
//! fastest, then y and z, with the `n` components of a vector interleaved.
//!
//! HxByteRLE: a control byte `n`; `n < 128`: the next byte repeated `n`
//! times; `n >= 128`: `n - 128` literal bytes.
//!
//! Lattice fields of the same type are the channels of one series (a
//! vector field contributes one channel per component); fields of another
//! type are further series. Pixel size from the top-level `BoundingBox`
//! (node positions: the span over `n - 1`) of uniform lattices, or
//! `BoundingBoxXY` of stacked ones, in the unit of
//! `Parameters { Units { Coordinates } }` (micrometres if not given).
//!
//! Provenance: Amira 3.1 User's Guide and Reference Manual (ZIB / TGS,
//! chapter "AmiraMesh Format", `doc/amira/`); T. Weinkauf, "Reading
//! AmiraMesh Files" (KTH, `doc/amira/`); own analysis of the AmiraMesh
//! samples on the OME sample server (CC BY 4.0); the HxByteRLE layout
//! checked against Bio-Formats as a black box. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::{Path, PathBuf};

use super::strided::{Data, Strided};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo =
    FormatInfo::new(FormatId::AmiraMesh, "AmiraMesh", "1").extensions(&["am", "amiramesh", "grey", "hx", "labels"]);

/// Largest header parsed.
const MAX_HEADER: usize = 1 << 20;
/// Largest block decoded into memory (compressed or ASCII data).
const MAX_DECODED: u64 = 4 << 30;

/// Returns `true` for an AmiraMesh or Avizo header.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(b"# AmiraMesh") || head.starts_with(b"# Avizo")
}

// --- Header ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Str(String),
    Open,
    Close,
    Comma,
    LParen,
    RParen,
    Equals,
    Newline,
}

/// Tokens of header text (bytes as Latin-1): `#` comments to the end of
/// the line, quoted strings, braces, parentheses, commas, `=`.
fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '\n' => {
                chars.next();
                out.push(Token::Newline);
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            '#' => {
                while chars.peek().is_some_and(|&c| c != '\n') {
                    chars.next();
                }
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    s.push(c);
                }
                out.push(Token::Str(s));
            }
            '{' | '}' | ',' | '(' | ')' | '=' => {
                chars.next();
                out.push(match c {
                    '{' => Token::Open,
                    '}' => Token::Close,
                    ',' => Token::Comma,
                    '(' => Token::LParen,
                    ')' => Token::RParen,
                    _ => Token::Equals,
                });
            }
            _ => {
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || "{},()=\"#".contains(c) {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                out.push(Token::Word(s));
            }
        }
    }
    out
}

/// A `Parameters` entry: words of a value, or a nested group.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Words(Vec<String>),
    Group(Vec<(String, Value)>),
}

/// Parses group entries up to the matching `}` (or the end), starting at
/// `*at`; nesting is limited against corrupt input.
fn parse_group(tokens: &[Token], at: &mut usize, depth: usize) -> Vec<(String, Value)> {
    let mut entries = Vec::new();
    while *at < tokens.len() {
        match &tokens[*at] {
            Token::Close => {
                *at += 1;
                return entries;
            }
            Token::Word(key) | Token::Str(key) => {
                let key = key.clone();
                *at += 1;
                if tokens.get(*at) == Some(&Token::Open) {
                    *at += 1;
                    let group = if depth < 64 { parse_group(tokens, at, depth + 1) } else { skip_group(tokens, at) };
                    entries.push((key, Value::Group(group)));
                    continue;
                }
                let mut words = Vec::new();
                while let Some(t) = tokens.get(*at) {
                    match t {
                        Token::Word(w) | Token::Str(w) => words.push(w.clone()),
                        Token::Comma | Token::Newline => {
                            *at += 1;
                            break;
                        }
                        Token::Close => break,
                        // A group value after words: `key word { ... }`.
                        Token::Open => {
                            *at += 1;
                            skip_group(tokens, at);
                            break;
                        }
                        _ => {}
                    }
                    *at += 1;
                }
                // `&key` entries hold binary copies of their key's value.
                if !key.starts_with('&') {
                    entries.push((key, Value::Words(words)));
                }
            }
            Token::Open => {
                // An anonymous group (`Materials { { Name ... } }`).
                *at += 1;
                let group = if depth < 64 { parse_group(tokens, at, depth + 1) } else { skip_group(tokens, at) };
                entries.push((String::new(), Value::Group(group)));
            }
            _ => *at += 1,
        }
    }
    entries
}

/// Skips to after the `}` closing the group whose `{` was just consumed.
fn skip_group(tokens: &[Token], at: &mut usize) -> Vec<(String, Value)> {
    let mut depth = 1usize;
    while *at < tokens.len() && depth > 0 {
        match tokens[*at] {
            Token::Open => depth += 1,
            Token::Close => depth -= 1,
            _ => {}
        }
        *at += 1;
    }
    Vec::new()
}

/// A data declaration `Array { type[n] Name } @k(codec,size)`.
#[derive(Debug, Clone, PartialEq)]
struct Decl {
    array: String,
    ty: String,
    components: u64,
    name: String,
    block: u32,
    codec: Option<(String, u64)>,
}

#[derive(Debug, Default)]
struct Header {
    defines: Vec<(String, Vec<u64>)>,
    params: Vec<(String, Value)>,
    decls: Vec<Decl>,
}

fn parse_header(text: &str) -> Header {
    let tokens = tokenize(text);
    let mut h = Header::default();
    let mut at = 0;
    while at < tokens.len() {
        let Token::Word(word) = &tokens[at] else {
            at += 1;
            continue;
        };
        at += 1;
        if word == "define" {
            let Some(Token::Word(name)) = tokens.get(at) else { continue };
            at += 1;
            let mut dims = Vec::new();
            while let Some(Token::Word(w)) = tokens.get(at) {
                match w.parse() {
                    Ok(v) => dims.push(v),
                    Err(_) => break,
                }
                at += 1;
            }
            h.defines.push((name.clone(), dims));
            continue;
        }
        if tokens.get(at) != Some(&Token::Open) {
            continue;
        }
        at += 1;
        if word == "Parameters" || word == "Materials" {
            let group = parse_group(&tokens, &mut at, 0);
            if word == "Parameters" {
                h.params.extend(group);
            } else {
                h.params.push((word.clone(), Value::Group(group)));
            }
            continue;
        }
        // `Array { type[n] Name } [=] @k[(codec,size)]`.
        let (Some(Token::Word(ty)), Some(Token::Word(name)), Some(Token::Close)) = (tokens.get(at), tokens.get(at + 1), tokens.get(at + 2)) else {
            skip_group(&tokens, &mut at);
            continue;
        };
        at += 3;
        if tokens.get(at) == Some(&Token::Equals) {
            at += 1;
        }
        let Some(Token::Word(marker)) = tokens.get(at) else { continue };
        let Some(block) = marker.strip_prefix('@').and_then(|k| k.parse().ok()) else { continue };
        at += 1;
        let mut codec = None;
        if let (Some(Token::LParen), Some(Token::Word(c)), Some(Token::Comma), Some(Token::Word(size)), Some(Token::RParen)) =
            (tokens.get(at), tokens.get(at + 1), tokens.get(at + 2), tokens.get(at + 3), tokens.get(at + 4))
        {
            codec = Some((c.clone(), size.parse().unwrap_or(0)));
            at += 5;
        }
        let (ty, components) = match ty.split_once('[') {
            Some((t, n)) => (t.to_string(), n.trim_end_matches(']').parse().unwrap_or(0)),
            None => (ty.clone(), 1),
        };
        h.decls.push(Decl { array: word.clone(), ty, components, name: name.clone(), block, codec });
    }
    h
}

/// Entry `key` of a parameter group.
fn get<'a>(group: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    group.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Numbers of a top-level parameter.
fn numbers(group: &[(String, Value)], key: &str) -> Vec<f64> {
    match get(group, key) {
        Some(Value::Words(w)) => w.iter().filter_map(|s| s.parse().ok()).collect(),
        _ => Vec::new(),
    }
}

/// Flattens the parameter tree into `Parameters.a.b` pairs.
fn flatten(group: &[(String, Value)], prefix: &str, out: &mut Vec<(String, String)>) {
    for (k, v) in group {
        let key = if k.is_empty() { prefix.to_string() } else { format!("{prefix}.{k}") };
        match v {
            Value::Words(w) => out.push((key, w.join(" "))),
            Value::Group(g) => flatten(g, &key, out),
        }
    }
}

/// Sample type of a declaration type.
fn pixel_type(ty: &str) -> Option<PixelType> {
    Some(match ty {
        "byte" | "ubyte" => PixelType::U8,
        "sbyte" => PixelType::I8,
        "short" => PixelType::I16,
        "ushort" => PixelType::U16,
        "int" => PixelType::I32,
        "uint" => PixelType::U32,
        "float" => PixelType::F32,
        "double" => PixelType::F64,
        _ => return None,
    })
}

/// Nanometres per unit of `Units { Coordinates }` (Latin-1 or UTF-8).
fn unit_nm(unit: &str) -> Option<f64> {
    Some(match unit.trim() {
        "" | "um" | "\u{b5}m" | "\u{c2}\u{b5}m" | "\u{3bc}m" | "micron" | "microns" | "micrometer" | "micrometre" => 1e3,
        "nm" | "nanometer" | "nanometre" => 1.0,
        "\u{c5}" | "\u{c3}\u{85}" | "A" | "angstrom" => 0.1,
        "mm" | "millimeter" | "millimetre" => 1e6,
        "cm" => 1e7,
        "m" | "meter" | "metre" => 1e9,
        _ => return None,
    })
}

// --- Data blocks ------------------------------------------------------------

/// Where the samples of one block are in the file.
#[derive(Debug, Clone, Copy)]
struct Block {
    start: u64,
    /// Bytes stored (compressed size, raw size or text length).
    len: u64,
}

/// Locates the data blocks of a binary file in their stored order: each
/// after its `@k` line and as long as its declaration.
fn binary_blocks(
    file: &std::fs::File,
    file_len: u64,
    data_at: u64,
    decls: &[Decl],
    bytes_of: impl Fn(&Decl) -> Option<u64>,
) -> Result<Vec<(u32, Block)>> {
    let mut out = Vec::new();
    let mut at = data_at;
    let mut line = [0u8; 64];
    while at < file_len && out.len() < decls.len() {
        // Skip white space before the marker, then read the marker line.
        let n = ((file_len - at) as usize).min(line.len());
        read_exact_at(file, &mut line[..n], at)?;
        let lead = line[..n].iter().take_while(|b| b.is_ascii_whitespace()).count();
        if lead == n {
            at += n as u64;
            continue;
        }
        let rest = &line[lead..n];
        let end = rest.iter().position(|&b| b == b'\n').ok_or_else(|| Error::Corrupt("AmiraMesh data block marker without line end".into()))?;
        let marker = std::str::from_utf8(&rest[..end]).unwrap_or("").trim();
        let k: u32 =
            marker.strip_prefix('@').and_then(|k| k.parse().ok()).ok_or_else(|| Error::Corrupt(format!("AmiraMesh data block marker {marker:?}")))?;
        let start = at + (lead + end + 1) as u64;
        let decl = decls.iter().find(|d| d.block == k).ok_or_else(|| Error::Corrupt(format!("AmiraMesh data block @{k} without declaration")))?;
        let len = match &decl.codec {
            // Compressed without size: the rest of the file (the last block).
            Some((_, 0)) => file_len - start,
            Some((_, size)) => *size,
            None => bytes_of(decl).ok_or_else(|| Error::Corrupt(format!("AmiraMesh array {} without size", decl.array)))?,
        };
        if start.checked_add(len).is_none_or(|e| e > file_len) {
            return Err(Error::Corrupt(format!("AmiraMesh data block @{k} extends beyond the end of the file")));
        }
        out.push((k, Block { start, len }));
        at = start + len;
    }
    Ok(out)
}

/// Locates the blocks of an ASCII file: from each `@k` line to the next.
fn ascii_blocks(text: &[u8], data_at: usize) -> Vec<(u32, Block)> {
    let mut marks = Vec::new();
    let mut at = data_at;
    for line in text[data_at..].split_inclusive(|&b| b == b'\n') {
        let l = std::str::from_utf8(line).unwrap_or("").trim();
        if let Some(k) = l.strip_prefix('@').and_then(|k| k.parse::<u32>().ok()) {
            marks.push((k, at + line.len()));
        }
        at += line.len();
    }
    let ends: Vec<usize> = marks.iter().skip(1).map(|&(_, s)| s).chain([text.len()]).collect();
    marks.iter().zip(ends).map(|(&(k, start), end)| (k, Block { start: start as u64, len: (end - start) as u64 })).collect()
}

/// Decodes HxByteRLE `src` into exactly `total` bytes.
fn unrle(src: &[u8], total: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(total);
    let mut i = 0;
    while out.len() < total {
        let &n = src.get(i).ok_or_else(|| Error::Corrupt("AmiraMesh HxByteRLE data shorter than its array".into()))?;
        i += 1;
        let count = (n & 0x7F) as usize;
        let count = count.min(total - out.len());
        if n & 0x80 != 0 {
            let lit = src.get(i..i + count).ok_or_else(|| Error::Corrupt("AmiraMesh HxByteRLE literal beyond its data".into()))?;
            out.extend_from_slice(lit);
            i += count;
        } else {
            let &v = src.get(i).ok_or_else(|| Error::Corrupt("AmiraMesh HxByteRLE run beyond its data".into()))?;
            out.resize(out.len() + count, v);
            i += 1;
        }
    }
    Ok(out)
}

/// Parses `count` ASCII numbers into little-endian samples.
fn parse_ascii(text: &[u8], pixel_type: PixelType, count: u64) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity((count as usize).saturating_mul(pixel_type.bytes()));
    let mut n = 0u64;
    for line in text.split(|&b| b == b'\n') {
        let line = line.split(|&b| b == b'#').next().unwrap_or(&[]);
        for word in line.split(|b| b.is_ascii_whitespace()).filter(|w| !w.is_empty()) {
            if n == count {
                return Ok(out);
            }
            let v: f64 = std::str::from_utf8(word)
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| Error::Corrupt(format!("AmiraMesh ASCII value {:?}", String::from_utf8_lossy(word))))?;
            match pixel_type {
                PixelType::U8 => out.push(v as u8),
                PixelType::I8 => out.push(v as i8 as u8),
                PixelType::I16 => out.extend_from_slice(&(v as i16).to_le_bytes()),
                PixelType::U16 => out.extend_from_slice(&(v as u16).to_le_bytes()),
                PixelType::I32 => out.extend_from_slice(&(v as i32).to_le_bytes()),
                PixelType::U32 => out.extend_from_slice(&(v as u32).to_le_bytes()),
                PixelType::F32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
                _ => out.extend_from_slice(&v.to_le_bytes()),
            }
            n += 1;
        }
    }
    if n < count {
        return Err(Error::Corrupt(format!("AmiraMesh ASCII data with {n} of {count} values")));
    }
    Ok(out)
}

// --- Reader ---------------------------------------------------------------

/// Lattice fields of one sample type: the channels of one series as
/// (field, component), with their names.
struct Kind {
    pixel_type: PixelType,
    channels: Vec<(usize, u32)>,
    names: Vec<String>,
}

/// One series: its channels as (field samples, component).
#[derive(Debug)]
struct Series {
    channels: Vec<(usize, u32)>,
}

/// An AmiraMesh lattice file.
#[derive(Debug)]
pub(super) struct Amira {
    fields: Vec<Strided>,
    series: Vec<Series>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Amira {
    /// Opens an AmiraMesh file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header or short data and
    /// [`Error::UnsupportedVariant`] for files without a lattice field
    /// (surfaces, tetrahedral grids), complex or 64-bit integer samples and
    /// other codecs.
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let file_len = file.metadata()?.len();
        let mut raw = vec![0u8; file_len.min(MAX_HEADER as u64) as usize];
        read_exact_at(&file, &mut raw, 0)?;
        let first_line = raw.split(|&b| b == b'\n').next().unwrap_or(&[]);
        let first = String::from_utf8_lossy(first_line).to_ascii_uppercase();
        let (binary, little) = if first.contains("BINARY-LITTLE-ENDIAN") {
            (true, true)
        } else if first.contains("BINARY") {
            (true, false)
        } else if first.contains("ASCII") {
            (false, true)
        } else {
            return Err(Error::UnsupportedVariant(format!("AmiraMesh encoding in {:?}", first.trim())));
        };

        // The header ends at "# Data section follows" or the first `@k` line.
        let data_at = header_end(&raw).ok_or_else(|| Error::Corrupt("AmiraMesh header without data section".into()))?;
        let text: String = raw[..data_at].iter().map(|&b| b as char).collect();
        let header = parse_header(&text);
        let dims = header.defines.iter().find(|(n, _)| n == "Lattice").map(|(_, d)| d.clone()).unwrap_or_default();
        let (nx, ny, nz) = match dims[..] {
            [x, y] => (x, y, 1),
            [x, y, z] => (x, y, z),
            _ => return Err(Error::UnsupportedVariant("AmiraMesh file without a 2D or 3D Lattice".into())),
        };
        if [nx, ny, nz].iter().any(|&v| v == 0 || v > u32::MAX as u64) {
            return Err(Error::Corrupt(format!("AmiraMesh lattice {nx} x {ny} x {nz}")));
        }
        let count = nx.checked_mul(ny).and_then(|n| n.checked_mul(nz)).ok_or_else(|| Error::Corrupt("AmiraMesh lattice too large".into()))?;

        // Stored bytes of each declaration (all arrays, to skip non-lattice blocks).
        let array_len =
            |name: &str| header.defines.iter().find(|(n, _)| n == name).and_then(|(_, d)| d.iter().try_fold(1u64, |a, &v| a.checked_mul(v)));
        let bytes_of = |d: &Decl| {
            let elem = pixel_type(&d.ty).map(|t| t.bytes() as u64).or(match d.ty.as_str() {
                "complex" | "long" | "int64" => Some(8),
                _ => None,
            })?;
            array_len(&d.array)?.checked_mul(d.components)?.checked_mul(elem)
        };
        let blocks = if binary {
            binary_blocks(&file, file_len, data_at as u64, &header.decls, bytes_of)?
        } else {
            let all = if (file_len as usize) <= raw.len() { raw.clone() } else { std::fs::read(path)? };
            ascii_blocks(&all, data_at)
        };

        // Lattice fields.
        let lattice: Vec<&Decl> = header.decls.iter().filter(|d| d.array == "Lattice").collect();
        if lattice.is_empty() {
            return Err(Error::UnsupportedVariant("AmiraMesh file without lattice data".into()));
        }
        let mut fields = Vec::new();
        let mut kinds: Vec<Kind> = Vec::new();
        for d in &lattice {
            let pt = pixel_type(&d.ty).ok_or_else(|| Error::UnsupportedVariant(format!("AmiraMesh sample type {}", d.ty)))?;
            if d.components == 0 || d.components > 4096 {
                return Err(Error::Corrupt(format!("AmiraMesh field {} with {} components", d.name, d.components)));
            }
            let block = blocks
                .iter()
                .find(|(k, _)| *k == d.block)
                .map(|(_, b)| *b)
                .ok_or_else(|| Error::Corrupt(format!("AmiraMesh data block @{} missing", d.block)))?;
            let total = count
                .checked_mul(d.components)
                .and_then(|n| n.checked_mul(pt.bytes() as u64))
                .ok_or_else(|| Error::Corrupt("AmiraMesh lattice too large".into()))?;
            let data = field_data(path, binary, block, d, pt, total, count * d.components)?;
            let n = d.components;
            fields.push(Strided {
                data,
                sx: n,
                sy: n * nx,
                sz: n * nx * ny,
                c_axes: vec![(1, n)],
                t_axes: Vec::new(),
                bytes: pt.bytes(),
                // ASCII values are decoded little endian; codecs keep the file's order.
                little: little || !binary,
                flip_y: false,
                height: ny,
                total,
                what: "AmiraMesh",
            });
            let field = fields.len() - 1;
            let names = (0..n).map(|c| if n > 1 { format!("{}[{c}]", d.name) } else { d.name.clone() });
            match kinds.iter_mut().find(|k| k.pixel_type == pt) {
                Some(k) => {
                    k.channels.extend((0..n as u32).map(|c| (field, c)));
                    k.names.extend(names);
                }
                None => kinds.push(Kind { pixel_type: pt, channels: (0..n as u32).map(|c| (field, c)).collect(), names: names.collect() }),
            }
        }

        // Pixel size from the bounding box of uniform or stacked lattices.
        let params = &header.params;
        let unit = match get(params, "Units").and_then(|u| if let Value::Group(g) = u { get(g, "Coordinates") } else { None }) {
            Some(Value::Words(w)) => unit_nm(&w.join(" ")),
            _ => Some(1e3),
        };
        let coord = match get(params, "CoordType") {
            Some(Value::Words(w)) => w.join(" "),
            _ => "uniform".into(),
        };
        let bbox = match coord.as_str() {
            "uniform" => numbers(params, "BoundingBox"),
            "stacked" => numbers(params, "BoundingBoxXY"),
            _ => Vec::new(),
        };
        let step = |i: usize, n: u64| -> Option<f64> {
            let (lo, hi) = (*bbox.get(2 * i)?, *bbox.get(2 * i + 1)?);
            let v = (hi - lo) / (n as f64 - 1.0) * unit?;
            (n > 1 && v.is_finite() && v > 0.0).then_some(v)
        };
        let physical_size = PhysicalSize { x_nm: step(0, nx), y_nm: step(1, ny), z_nm: if coord == "uniform" { step(2, nz) } else { None } };

        let mut original = OriginalMeta::default();
        original.pairs.push(("AmiraMesh.Header".into(), String::from_utf8_lossy(first_line).trim().to_string()));
        for (n, d) in &header.defines {
            original.pairs.push((format!("define.{n}"), d.iter().map(u64::to_string).collect::<Vec<_>>().join(" ")));
        }
        flatten(params, "Parameters", &mut original.pairs);
        for d in &header.decls {
            let ty = if d.components > 1 { format!("{}[{}]", d.ty, d.components) } else { d.ty.clone() };
            let codec = d.codec.as_ref().map(|(c, s)| format!("({c},{s})")).unwrap_or_default();
            original.pairs.push((format!("{}.{}", d.array, d.name), format!("{ty} @{}{codec}", d.block)));
        }

        let stem = path.file_stem().map(|n| n.to_string_lossy().into_owned());
        let several = kinds.len() > 1;
        let mut series = Vec::new();
        let mut metas = Vec::new();
        for Kind { pixel_type, channels, names } in kinds {
            let named = channels.len() > 1 || names.iter().any(|n| n != "Data");
            metas.push(SeriesMeta {
                name: if several { Some(names.join(", ")) } else { stem.clone() },
                size_c: channels.len() as u32,
                size_z: nz as u32,
                size_t: 1,
                pixel_type,
                samples_per_pixel: 1,
                levels: vec![LevelMeta { size_x: nx, size_y: ny, tile: None, downsample: 1.0 }],
                physical_size,
                channels: names.into_iter().map(|n| ChannelMeta { name: named.then_some(n), ..Default::default() }).collect(),
                ..Default::default()
            });
            series.push(Series { channels });
        }
        Ok(Amira { fields, series, metas, original })
    }
}

/// Offset just after the header: after the `# Data section follows`
/// line, else at the first line `@<k>`.
fn header_end(raw: &[u8]) -> Option<usize> {
    let mut at = 0;
    let mut depth = 0i32;
    for line in raw.split_inclusive(|&b| b == b'\n') {
        let l = String::from_utf8_lossy(line);
        let t = l.trim();
        if t.starts_with("# Data section follows") {
            return Some(at + line.len());
        }
        if depth <= 0 && t.strip_prefix('@').is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())) {
            return Some(at);
        }
        // Braces outside quotes and comments track the Parameters nesting.
        let mut quoted = false;
        for c in t.chars() {
            match c {
                '"' => quoted = !quoted,
                '#' if !quoted => break,
                '{' if !quoted => depth += 1,
                '}' if !quoted => depth -= 1,
                _ => {}
            }
        }
        at += line.len();
    }
    None
}

/// The samples of one lattice field.
fn field_data(path: &Path, binary: bool, block: Block, d: &Decl, pt: PixelType, total: u64, values: u64) -> Result<Data> {
    let (start, len) = (block.start, block.len);
    let path: PathBuf = path.to_path_buf();
    let decoded = |what: &str| -> Result<()> {
        if total > MAX_DECODED { Err(Error::UnsupportedVariant(format!("{what} AmiraMesh data above 4 GiB"))) } else { Ok(()) }
    };
    let read = move |p: &Path| -> Result<Vec<u8>> {
        let file = std::fs::File::open(p)?;
        let mut src = vec![0u8; len as usize];
        read_exact_at(&file, &mut src, start)?;
        Ok(src)
    };
    if !binary {
        decoded("ASCII")?;
        return Ok(Data::decoded(move || parse_ascii(&read(&path)?, pt, values)));
    }
    match d.codec.as_ref().map(|(c, _)| c.as_str()) {
        None => Data::file(&path, start, total, "AmiraMesh"),
        Some("HxZip") => {
            decoded("compressed")?;
            Ok(Data::decoded(move || {
                let src = read(&path)?;
                // Deflate expands by at most 1032:1: a larger claim is corrupt.
                if total > (src.len() as u64).saturating_mul(1032) + 64 {
                    return Err(Error::Corrupt("AmiraMesh HxZip data larger than its compressed size allows".into()));
                }
                let mut out = vec![0u8; total as usize];
                if crate::codecs::deflate(&src, &mut out, true)? < out.len() {
                    return Err(Error::Corrupt("AmiraMesh HxZip data shorter than its array".into()));
                }
                Ok(out)
            }))
        }
        Some("HxByteRLE") => {
            decoded("compressed")?;
            // A run byte expands to at most 127 samples.
            if total > len.saturating_mul(127) {
                return Err(Error::Corrupt("AmiraMesh HxByteRLE data larger than its compressed size allows".into()));
            }
            Ok(Data::decoded(move || unrle(&read(&path)?, total as usize)))
        }
        Some(c) => Err(Error::UnsupportedVariant(format!("AmiraMesh codec {c}"))),
    }
}

impl Backend for Amira {
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
        let (field, component) = self.series[series].channels[plane.c as usize];
        self.fields[field].read_plane(Plane::new(component, plane.z, 0), region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header() {
        let h = parse_header(
            "# AmiraMesh BINARY-LITTLE-ENDIAN 2.1\ndefine Lattice 4 6 8\nParameters {\n Units { Coordinates \"nm\" }\n \
             Inner { BoundingBox 9 9 9 9 9 9 }\n &Key \"QUJD\",\n BoundingBox -1 0 0 1 -0.5 0.5,\n CoordType \"uniform\"\n}\n\
             Lattice { float[2] Data } @1(HxZip,123)\nLattice { byte Labels } = @2\n",
        );
        assert_eq!(h.defines, [("Lattice".to_string(), vec![4, 6, 8])]);
        assert_eq!(numbers(&h.params, "BoundingBox"), [-1.0, 0.0, 0.0, 1.0, -0.5, 0.5]);
        assert!(get(&h.params, "&Key").is_none());
        assert_eq!(
            h.decls[0],
            Decl { array: "Lattice".into(), ty: "float".into(), components: 2, name: "Data".into(), block: 1, codec: Some(("HxZip".into(), 123)) }
        );
        assert_eq!((h.decls[1].name.as_str(), h.decls[1].block, h.decls[1].codec.is_none()), ("Labels", 2, true));
    }

    #[test]
    fn decodes_rle_and_ascii() {
        assert_eq!(unrle(&[3, 7, 0x82, 1, 2, 2, 9], 7).unwrap(), [7, 7, 7, 1, 2, 9, 9]);
        assert!(unrle(&[0x85, 1], 5).is_err());
        assert_eq!(parse_ascii(b"1 2 # c\n -3\n", PixelType::I16, 3).unwrap(), [1, 0, 2, 0, 0xfd, 0xff]);
        assert!(parse_ascii(b"1 2", PixelType::U8, 3).is_err());
    }

    #[test]
    fn finds_header_end() {
        assert_eq!(header_end(b"# AmiraMesh ASCII 1.0\nParameters {\n@1 x\n}\nLattice { float a } = @1\n@1\n0\n"), Some(67));
        assert_eq!(header_end(b"# AmiraMesh BINARY 2.1\nx\n# Data section follows\n@1\n\x01"), Some(48));
    }
}

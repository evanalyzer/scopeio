//! # hdf5
//!
//! A read-only subset of the HDF5 file format, as far as Imaris files need
//! it: superblocks 0-3, object headers 1 and 2 with continuations, groups
//! (symbol tables, compact and dense links), attributes (compact and
//! dense), datasets with contiguous, compact or chunked layout (version 1
//! B-tree, single chunk) and their filter pipeline. Dense storage is read
//! through version 2 B-trees and fractal heaps (managed and tiny objects).
//! Checksums are not verified; every size is bounds-checked.
//!
//! Provenance: HDF5 File Format Specification version 3 (The HDF Group,
//! `doc/hdf5/`). No HDF5 library code consulted.
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

const SIGNATURE: &[u8; 8] = b"\x89HDF\r\n\x1a\n";
/// Largest metadata structure read at once, against corrupt files.
const MAX_META: u64 = 64 << 20;
/// Deepest structure nesting followed (B-trees, continuations).
const MAX_DEPTH: usize = 32;

fn corrupt(what: &str) -> Error {
    Error::Corrupt(format!("HDF5: {what}"))
}

/// Bounds-checked little-endian reader over a byte slice.
pub(super) struct Cursor<'a> {
    b: &'a [u8],
    pub pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Cursor { b, pos: 0 }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.b.len()).ok_or_else(|| corrupt("structure truncated"))?;
        let s = &self.b[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("2 bytes")))
    }

    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }

    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8 bytes")))
    }

    /// Unsigned little-endian integer of `n` (0-8) bytes.
    pub fn uint(&mut self, n: usize) -> Result<u64> {
        if n > 8 {
            return Err(corrupt("integer wider than 8 bytes"));
        }
        let mut v = [0u8; 8];
        v[..n].copy_from_slice(self.take(n)?);
        Ok(u64::from_le_bytes(v))
    }

    pub fn rest(&self) -> &'a [u8] {
        &self.b[self.pos.min(self.b.len())..]
    }
}

/// Element type of a dataset or attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Uint,
    Int,
    Float,
    String,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Datatype {
    pub kind: Kind,
    pub size: usize,
    pub big_endian: bool,
}

fn datatype(b: &[u8]) -> Result<Datatype> {
    let mut c = Cursor::new(b);
    let class_version = c.u8()?;
    let bits0 = c.u8()?;
    c.skip(2)?;
    let size = c.u32()? as usize;
    let kind = match class_version & 0x0F {
        0 if bits0 & 8 != 0 => Kind::Int,
        0 => Kind::Uint,
        1 => Kind::Float,
        3 => Kind::String,
        _ => Kind::Other,
    };
    // Fixed point: bit 0; floating point: bits 0 and 6 (VAX not supported).
    Ok(Datatype { kind, size, big_endian: bits0 & 1 != 0 })
}

fn dataspace(b: &[u8], sl: usize) -> Result<Vec<u64>> {
    let mut c = Cursor::new(b);
    let version = c.u8()?;
    let ndims = c.u8()? as usize;
    let _flags = c.u8()?;
    match version {
        1 => c.skip(5)?,
        2 => {
            // Type 0 scalar, 1 simple, 2 null.
            if c.u8()? == 2 {
                return Ok(vec![0]);
            }
        }
        v => return Err(Error::UnsupportedVariant(format!("HDF5 dataspace version {v}"))),
    }
    (0..ndims).map(|_| c.uint(sl)).collect()
}

/// Raw object header messages.
#[derive(Debug, Default)]
pub(super) struct Object {
    pub messages: Vec<(u16, Vec<u8>)>,
}

impl Object {
    pub fn get(&self, typ: u16) -> Option<&[u8]> {
        self.messages.iter().find(|(t, _)| *t == typ).map(|(_, d)| d.as_slice())
    }

    pub fn all(&self, typ: u16) -> impl Iterator<Item = &[u8]> {
        self.messages.iter().filter(move |(t, _)| *t == typ).map(|(_, d)| d.as_slice())
    }
}

pub(super) mod msg {
    pub const DATASPACE: u16 = 0x0001;
    pub const LINK_INFO: u16 = 0x0002;
    pub const DATATYPE: u16 = 0x0003;
    pub const LINK: u16 = 0x0006;
    pub const LAYOUT: u16 = 0x0008;
    pub const FILTERS: u16 = 0x000B;
    pub const ATTRIBUTE: u16 = 0x000C;
    pub const CONTINUATION: u16 = 0x0010;
    pub const SYMBOL_TABLE: u16 = 0x0011;
    pub const ATTRIBUTE_INFO: u16 = 0x0015;
}

/// An attribute: name, type, shape and raw data.
#[derive(Debug, Clone)]
pub(super) struct Attribute {
    pub name: String,
    pub dtype: Datatype,
    /// Shape; empty for scalars.
    #[allow(dead_code)]
    pub dims: Vec<u64>,
    pub data: Vec<u8>,
}

impl Attribute {
    /// The value as text: strings and arrays of one-character strings
    /// joined, NULs dropped; numbers formatted.
    pub fn text(&self) -> String {
        match self.dtype.kind {
            Kind::String => String::from_utf8_lossy(&self.data).trim_end_matches('\0').replace('\0', "").trim().to_string(),
            Kind::Uint | Kind::Int | Kind::Float => {
                let n = self.dtype.size.max(1);
                self.data.chunks_exact(n).map(|v| number(v, self.dtype).to_string()).collect::<Vec<_>>().join(" ")
            }
            Kind::Other => String::new(),
        }
    }
}

/// A number element of `t` as f64.
pub(super) fn number(v: &[u8], t: Datatype) -> f64 {
    let mut b = [0u8; 8];
    let n = v.len().min(8);
    if t.big_endian {
        b[..n].copy_from_slice(&v[..n]);
        b[..n].reverse();
    } else {
        b[..n].copy_from_slice(&v[..n]);
    }
    match (t.kind, n) {
        (Kind::Float, 4) => f32::from_le_bytes(b[..4].try_into().expect("4 bytes")) as f64,
        (Kind::Float, 8) => f64::from_le_bytes(b),
        (Kind::Int, n) => {
            let shift = 64 - 8 * n as u32;
            ((u64::from_le_bytes(b) << shift) as i64 >> shift) as f64
        }
        _ => u64::from_le_bytes(b) as f64,
    }
}

/// Storage of a dataset.
#[derive(Debug, Clone)]
pub(super) enum Layout {
    Compact(Vec<u8>),
    /// Address and allocated size (`u64::MAX` if not recorded).
    Contiguous { address: u64, #[allow(dead_code)] size: u64 },
    /// Chunk dimensions (without the element size) and the index.
    Chunked { dims: Vec<u64>, index: ChunkIndex },
}

#[derive(Debug, Clone)]
pub(super) enum ChunkIndex {
    BTree(u64),
    Single { address: u64, size: u64, mask: u32 },
}

/// One filter of a pipeline: id and client data.
#[derive(Debug, Clone)]
pub(super) struct Filter {
    pub id: u16,
    pub client: Vec<u32>,
}

#[derive(Debug, Clone)]
pub(super) struct Dataset {
    pub dtype: Datatype,
    pub dims: Vec<u64>,
    pub layout: Layout,
    pub filters: Vec<Filter>,
}

/// One stored chunk.
#[derive(Debug, Clone, Copy)]
pub(super) struct Chunk {
    pub address: u64,
    pub size: u32,
    pub mask: u32,
}

/// An open HDF5 file.
#[derive(Debug)]
pub(super) struct H5 {
    pub file: File,
    len: u64,
    /// Absolute position of address 0 (the superblock).
    base: u64,
    /// Size of offsets and of lengths.
    so: usize,
    sl: usize,
    pub root: u64,
}

impl H5 {
    /// Finds the superblock (at 0, 512, 1024, ...) and reads it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] if no valid superblock is found.
    pub fn open(file: File) -> Result<H5> {
        let len = file.metadata()?.len();
        let mut at = 0u64;
        while at + 64 <= len {
            let mut head = [0u8; 8];
            read_exact_at(&file, &mut head, at)?;
            if &head == SIGNATURE {
                return Self::superblock(file, len, at);
            }
            at = if at == 0 { 512 } else { at * 2 };
        }
        Err(corrupt("no superblock"))
    }

    /// Returns `true` if `head` (the first bytes of a file) starts with the
    /// HDF5 signature.
    pub fn matches(head: &[u8]) -> bool {
        head.starts_with(SIGNATURE)
    }

    fn superblock(file: File, len: u64, at: u64) -> Result<H5> {
        let mut b = vec![0u8; ((len - at).min(256)) as usize];
        read_exact_at(&file, &mut b, at)?;
        let mut c = Cursor::new(&b);
        c.skip(8)?;
        let version = c.u8()?;
        let mut h = H5 { file, len, base: at, so: 8, sl: 8, root: 0 };
        match version {
            0 | 1 => {
                c.skip(3)?;
                c.skip(1)?;
                h.so = c.u8()? as usize;
                h.sl = c.u8()? as usize;
                c.skip(1)?;
                c.skip(4)?; // group leaf and internal node K
                c.skip(4)?; // consistency flags
                if version == 1 {
                    c.skip(4)?;
                }
                h.check_sizes()?;
                let base = c.uint(h.so)?;
                c.skip(3 * h.so)?; // free space, end of file, driver block
                // Root group symbol table entry: link name offset, object header.
                c.skip(h.so)?;
                h.root = c.uint(h.so)?;
                h.base = base.max(at);
            }
            2 | 3 => {
                h.so = c.u8()? as usize;
                h.sl = c.u8()? as usize;
                c.skip(1)?;
                h.check_sizes()?;
                let base = c.uint(h.so)?;
                c.skip(2 * h.so)?; // superblock extension, end of file
                h.root = c.uint(h.so)?;
                h.base = base.max(at);
            }
            v => return Err(Error::UnsupportedVariant(format!("HDF5 superblock version {v}"))),
        }
        Ok(h)
    }

    fn check_sizes(&self) -> Result<()> {
        if !matches!(self.so, 2 | 4 | 8) || !matches!(self.sl, 2 | 4 | 8) {
            return Err(corrupt("offset or length size"));
        }
        Ok(())
    }

    /// The undefined address (all bits set).
    fn undefined(&self, a: u64) -> bool {
        a == u64::MAX >> (64 - 8 * self.so as u32)
    }

    /// Reads `len` bytes at the HDF5 address `at`.
    pub fn read(&self, at: u64, len: u64) -> Result<Vec<u8>> {
        if len > MAX_META {
            return Err(corrupt("structure too large"));
        }
        let abs = self.base.checked_add(at).ok_or_else(|| corrupt("address overflow"))?;
        if abs.checked_add(len).is_none_or(|e| e > self.len) {
            return Err(corrupt("structure beyond the end of the file"));
        }
        let mut b = vec![0u8; len as usize];
        read_exact_at(&self.file, &mut b, abs)?;
        Ok(b)
    }

    /// Reads up to `len` bytes at `at`, fewer at the end of the file.
    fn read_upto(&self, at: u64, len: u64) -> Result<Vec<u8>> {
        let avail = self.len.saturating_sub(self.base.saturating_add(at));
        self.read(at, len.min(avail))
    }

    /// Absolute file position of an HDF5 address, checked against the end.
    pub fn absolute(&self, at: u64, len: u64) -> Result<u64> {
        let abs = self.base.checked_add(at).ok_or_else(|| corrupt("address overflow"))?;
        if abs.checked_add(len).is_none_or(|e| e > self.len) {
            return Err(corrupt("data beyond the end of the file"));
        }
        Ok(abs)
    }

    // --- Object headers ---------------------------------------------------

    /// Parses the object header at `addr` with all continuation blocks.
    pub fn object(&self, addr: u64) -> Result<Object> {
        let mut obj = Object::default();
        let head = self.read_upto(addr, 16)?;
        if head.starts_with(b"OHDR") {
            self.object_v2(addr, &mut obj)?;
        } else if head.first() == Some(&1) {
            self.object_v1(addr, &mut obj)?;
        } else {
            return Err(corrupt("object header without version"));
        }
        Ok(obj)
    }

    fn object_v1(&self, addr: u64, obj: &mut Object) -> Result<()> {
        let head = self.read(addr, 16)?;
        let mut c = Cursor::new(&head);
        c.skip(4)?;
        c.skip(4)?; // reference count
        let size = c.u32()? as u64;
        // Messages start after the 16-byte prefix (aligned to 8).
        let mut blocks = vec![(addr + 16, size)];
        let mut n = 0;
        while let Some((at, len)) = blocks.pop() {
            n += 1;
            if n > MAX_DEPTH * 8 {
                return Err(corrupt("too many header continuations"));
            }
            let data = self.read(at, len)?;
            let mut c = Cursor::new(&data);
            while c.rest().len() >= 8 {
                let typ = c.u16()?;
                let size = c.u16()? as usize;
                let flags = c.u8()?;
                c.skip(3)?;
                let body = c.take(size)?.to_vec();
                self.push_message(obj, typ, flags, body, &mut blocks)?;
            }
        }
        Ok(())
    }

    fn object_v2(&self, addr: u64, obj: &mut Object) -> Result<()> {
        let head = self.read_upto(addr, 64)?;
        let mut c = Cursor::new(&head);
        c.skip(5)?;
        let flags = c.u8()?;
        if flags & 0x20 != 0 {
            c.skip(16)?;
        }
        if flags & 0x10 != 0 {
            c.skip(4)?;
        }
        let size = c.uint(1 << (flags & 3))?;
        let order = flags & 0x04 != 0;
        // Chunk 0, then continuation blocks ("OCHK"), each with a checksum.
        let mut blocks = vec![(addr + c.pos as u64, size, false)];
        let mut n = 0;
        while let Some((at, len, signed)) = blocks.pop() {
            n += 1;
            if n > MAX_DEPTH * 8 {
                return Err(corrupt("too many header continuations"));
            }
            let data = self.read(at, len)?;
            let mut c = Cursor::new(&data);
            if signed {
                if !data.starts_with(b"OCHK") {
                    return Err(corrupt("continuation block without signature"));
                }
                c.skip(4)?;
            }
            let prefix = if order { 6 } else { 4 };
            // The last four bytes of each block are its checksum.
            let end = data.len().saturating_sub(if signed { 4 } else { 0 });
            let mut cont = Vec::new();
            while c.pos + prefix <= end {
                let typ = c.u8()? as u16;
                let size = c.u16()? as usize;
                let mflags = c.u8()?;
                if order {
                    c.skip(2)?;
                }
                let body = c.take(size)?.to_vec();
                self.push_message(obj, typ, mflags, body, &mut cont)?;
            }
            blocks.extend(cont.into_iter().map(|(a, l)| (a, l, true)));
        }
        Ok(())
    }

    fn push_message(&self, obj: &mut Object, typ: u16, flags: u8, body: Vec<u8>, blocks: &mut Vec<(u64, u64)>) -> Result<()> {
        match typ {
            0 => {}
            msg::CONTINUATION => {
                let mut c = Cursor::new(&body);
                let at = c.uint(self.so)?;
                let len = c.uint(self.sl)?;
                if !self.undefined(at) {
                    blocks.push((at, len));
                }
            }
            _ if flags & 0x02 != 0 => {
                // Shared message: stored in another object header.
                if let Some(body) = self.shared(typ, &body)? {
                    obj.messages.push((typ, body));
                }
            }
            _ => obj.messages.push((typ, body)),
        }
        Ok(())
    }

    /// Resolves a shared message stored in another object header.
    fn shared(&self, typ: u16, body: &[u8]) -> Result<Option<Vec<u8>>> {
        let mut c = Cursor::new(body);
        let version = c.u8()?;
        let kind = c.u8()?;
        if version == 1 {
            c.skip(6)?;
        }
        if kind != 0 && !(version == 3 && kind == 2) {
            return Ok(None);
        }
        let at = c.uint(self.so)?;
        let other = self.object(at)?;
        Ok(other.get(typ).map(<[u8]>::to_vec))
    }

    // --- Groups -----------------------------------------------------------

    /// Links of a group: name and object header address, in storage order.
    pub fn links(&self, obj: &Object) -> Result<Vec<(String, u64)>> {
        let mut out = Vec::new();
        if let Some(st) = obj.get(msg::SYMBOL_TABLE) {
            let mut c = Cursor::new(st);
            let tree = c.uint(self.so)?;
            let heap = c.uint(self.so)?;
            let names = self.local_heap(heap)?;
            self.group_tree(tree, &names, &mut out, 0)?;
        }
        for l in obj.all(msg::LINK) {
            if let Some(link) = self.link(l)? {
                out.push(link);
            }
        }
        if let Some(info) = obj.get(msg::LINK_INFO) {
            let mut c = Cursor::new(info);
            c.u8()?;
            let flags = c.u8()?;
            if flags & 1 != 0 {
                c.skip(8)?;
            }
            let heap = c.uint(self.so)?;
            let tree = c.uint(self.so)?;
            if !self.undefined(heap) && !self.undefined(tree) {
                let heap = FractalHeap::open(self, heap)?;
                for id in self.btree2_records(tree, 0)? {
                    if let Some(link) = self.link(&heap.get(self, &id)?)? {
                        out.push(link);
                    }
                }
            }
        }
        Ok(out)
    }

    /// A hard link message; soft and external links are skipped.
    fn link(&self, b: &[u8]) -> Result<Option<(String, u64)>> {
        let mut c = Cursor::new(b);
        c.u8()?;
        let flags = c.u8()?;
        let link_type = if flags & 0x08 != 0 { c.u8()? } else { 0 };
        if flags & 0x04 != 0 {
            c.skip(8)?;
        }
        if flags & 0x10 != 0 {
            c.skip(1)?;
        }
        let name_len = c.uint(1 << (flags & 3))? as usize;
        let name = String::from_utf8_lossy(c.take(name_len)?).into_owned();
        if link_type != 0 {
            return Ok(None);
        }
        Ok(Some((name, c.uint(self.so)?)))
    }

    fn local_heap(&self, at: u64) -> Result<Vec<u8>> {
        let head = self.read(at, 8 + 2 * self.sl as u64 + self.so as u64)?;
        if !head.starts_with(b"HEAP") {
            return Err(corrupt("local heap without signature"));
        }
        let mut c = Cursor::new(&head);
        c.skip(8)?;
        let size = c.uint(self.sl)?;
        c.skip(self.sl)?;
        let data = c.uint(self.so)?;
        self.read(data, size)
    }

    /// Walks a group's version 1 B-tree (node type 0) and its symbol nodes.
    fn group_tree(&self, at: u64, names: &[u8], out: &mut Vec<(String, u64)>, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(corrupt("group B-tree too deep"));
        }
        let (level, children) = self.btree1_node(at, 0, 0)?;
        for (_, child) in children {
            if level > 0 {
                self.group_tree(child, names, out, depth + 1)?;
                continue;
            }
            let head = self.read(child, 8)?;
            if !head.starts_with(b"SNOD") {
                return Err(corrupt("symbol node without signature"));
            }
            let n = u16::from_le_bytes([head[6], head[7]]) as u64;
            let entry = 2 * self.so as u64 + 24;
            let data = self.read(child + 8, n * entry)?;
            let mut c = Cursor::new(&data);
            for _ in 0..n {
                let name_at = c.uint(self.so)? as usize;
                let addr = c.uint(self.so)?;
                c.skip(24)?;
                let name = names.get(name_at..).unwrap_or_default();
                let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
                out.push((String::from_utf8_lossy(&name[..end]).into_owned(), addr));
            }
        }
        Ok(())
    }

    /// A version 1 B-tree node: its level and (key, child address) pairs.
    /// `key_len` is the key size; for chunk trees (type 1) the keys are
    /// returned raw.
    fn btree1_node(&self, at: u64, node_type: u8, key_len: usize) -> Result<(u8, Vec<(Vec<u8>, u64)>)> {
        let head = self.read(at, 8 + 2 * self.so as u64)?;
        if !head.starts_with(b"TREE") || head[4] != node_type {
            return Err(corrupt("B-tree node without signature"));
        }
        let level = head[5];
        let n = u16::from_le_bytes([head[6], head[7]]) as u64;
        let key = if node_type == 0 { self.sl } else { key_len };
        let len = (n + 1) * key as u64 + n * self.so as u64;
        let data = self.read(at + head.len() as u64, len)?;
        let mut c = Cursor::new(&data);
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let k = c.take(key)?.to_vec();
            let child = c.uint(self.so)?;
            out.push((k, child));
        }
        Ok((level, out))
    }

    /// Heap IDs of all records of a version 2 B-tree (types 5 and 8).
    fn btree2_records(&self, at: u64, depth: usize) -> Result<Vec<Vec<u8>>> {
        let head = self.read(at, 16 + 2 * self.so as u64 + self.sl as u64)?;
        if !head.starts_with(b"BTHD") {
            return Err(corrupt("v2 B-tree header without signature"));
        }
        let mut c = Cursor::new(&head);
        c.skip(5)?;
        let typ = c.u8()?;
        let node_size = c.u32()? as u64;
        let rec_size = c.u16()? as usize;
        let tree_depth = c.u16()?;
        c.skip(2)?;
        let root = c.uint(self.so)?;
        let root_records = c.u16()? as u64;
        let total = c.uint(self.sl)?;
        if self.undefined(root) || total == 0 {
            return Ok(Vec::new());
        }
        let id_of = |r: &[u8]| -> Vec<u8> {
            match typ {
                5 => r.get(4..).unwrap_or_default().to_vec(),
                8 => r.get(..8).unwrap_or_default().to_vec(),
                _ => Vec::new(),
            }
        };
        if !matches!(typ, 5 | 8) {
            return Err(Error::UnsupportedVariant(format!("HDF5 v2 B-tree type {typ}")));
        }
        let mut out = Vec::new();
        let tree = Btree2 { node_size, rec_size, depth: tree_depth };
        tree.collect(self, root, root_records, tree_depth, &mut |r| out.push(id_of(r)), depth)?;
        Ok(out)
    }

    // --- Attributes -------------------------------------------------------

    /// All attributes of an object (compact and dense).
    pub fn attributes(&self, obj: &Object) -> Result<Vec<Attribute>> {
        let mut out = Vec::new();
        for a in obj.all(msg::ATTRIBUTE) {
            if let Ok(attr) = self.attribute(a) {
                out.push(attr);
            }
        }
        if let Some(info) = obj.get(msg::ATTRIBUTE_INFO) {
            let mut c = Cursor::new(info);
            c.u8()?;
            let flags = c.u8()?;
            if flags & 1 != 0 {
                c.skip(2)?;
            }
            let heap = c.uint(self.so)?;
            let tree = c.uint(self.so)?;
            if !self.undefined(heap) && !self.undefined(tree) {
                let heap = FractalHeap::open(self, heap)?;
                for id in self.btree2_records(tree, 0)? {
                    if let Ok(attr) = self.attribute(&heap.get(self, &id)?) {
                        out.push(attr);
                    }
                }
            }
        }
        Ok(out)
    }

    fn attribute(&self, b: &[u8]) -> Result<Attribute> {
        let mut c = Cursor::new(b);
        let version = c.u8()?;
        let flags = c.u8()?;
        let name_len = c.u16()? as usize;
        let dt_len = c.u16()? as usize;
        let ds_len = c.u16()? as usize;
        if version == 3 {
            c.skip(1)?;
        }
        // Version 1 pads each field to a multiple of eight bytes.
        let pad = |n: usize| if version == 1 { n.div_ceil(8) * 8 } else { n };
        let name = c.take(pad(name_len))?;
        let name = String::from_utf8_lossy(&name[..name_len.min(name.len())]).trim_end_matches('\0').to_string();
        if flags & 3 != 0 {
            return Err(Error::UnsupportedVariant("HDF5 shared attribute types".into()));
        }
        let dtype = datatype(c.take(pad(dt_len))?)?;
        let dims = dataspace(c.take(pad(ds_len))?, self.sl)?;
        let count: u64 = dims.iter().product::<u64>().max(if dims.is_empty() { 1 } else { 0 });
        let len = count.checked_mul(dtype.size as u64).filter(|&l| l <= c.rest().len() as u64).ok_or_else(|| corrupt("attribute data"))?;
        Ok(Attribute { name, dtype, dims, data: c.take(len as usize)?.to_vec() })
    }

    // --- Datasets ---------------------------------------------------------

    pub fn dataset(&self, obj: &Object) -> Result<Dataset> {
        let dtype = datatype(obj.get(msg::DATATYPE).ok_or_else(|| corrupt("dataset without datatype"))?)?;
        let dims = dataspace(obj.get(msg::DATASPACE).ok_or_else(|| corrupt("dataset without dataspace"))?, self.sl)?;
        let layout = self.layout(obj.get(msg::LAYOUT).ok_or_else(|| corrupt("dataset without layout"))?)?;
        let filters = match obj.get(msg::FILTERS) {
            Some(f) => filters(f)?,
            None => Vec::new(),
        };
        Ok(Dataset { dtype, dims, layout, filters })
    }

    fn layout(&self, b: &[u8]) -> Result<Layout> {
        let mut c = Cursor::new(b);
        let version = c.u8()?;
        match version {
            1 | 2 => {
                let nd = c.u8()? as usize;
                let class = c.u8()?;
                c.skip(5)?;
                let address = if class != 0 { c.uint(self.so)? } else { 0 };
                let dims: Vec<u64> = (0..nd).map(|_| c.u32().map(u64::from)).collect::<Result<_>>()?;
                match class {
                    1 => Ok(Layout::Contiguous { address, size: u64::MAX }),
                    2 => Ok(Layout::Chunked { dims: dims[..nd.saturating_sub(1)].to_vec(), index: ChunkIndex::BTree(address) }),
                    _ => {
                        let size = c.u32()? as usize;
                        Ok(Layout::Compact(c.take(size)?.to_vec()))
                    }
                }
            }
            3 | 4 => {
                let class = c.u8()?;
                match class {
                    0 => {
                        let size = c.u16()? as usize;
                        Ok(Layout::Compact(c.take(size)?.to_vec()))
                    }
                    1 => Ok(Layout::Contiguous { address: c.uint(self.so)?, size: c.uint(self.sl)? }),
                    2 if version == 3 => {
                        let nd = c.u8()? as usize;
                        let address = c.uint(self.so)?;
                        let dims: Vec<u64> = (0..nd).map(|_| c.u32().map(u64::from)).collect::<Result<_>>()?;
                        Ok(Layout::Chunked { dims: dims[..nd.saturating_sub(1)].to_vec(), index: ChunkIndex::BTree(address) })
                    }
                    2 => {
                        let flags = c.u8()?;
                        let nd = c.u8()? as usize;
                        let width = c.u8()? as usize;
                        let dims: Vec<u64> = (0..nd).map(|_| c.uint(width)).collect::<Result<_>>()?;
                        let index_type = c.u8()?;
                        match index_type {
                            1 => {
                                let (size, mask) = if flags & 2 != 0 { (c.uint(self.sl)?, c.u32()?) } else { (u64::MAX, 0) };
                                let address = c.uint(self.so)?;
                                Ok(Layout::Chunked { dims: dims[..nd.saturating_sub(1)].to_vec(), index: ChunkIndex::Single { address, size, mask } })
                            }
                            t => Err(Error::UnsupportedVariant(format!("HDF5 chunk index type {t}"))),
                        }
                    }
                    c => Err(Error::UnsupportedVariant(format!("HDF5 layout class {c}"))),
                }
            }
            v => Err(Error::UnsupportedVariant(format!("HDF5 layout version {v}"))),
        }
    }

    /// All chunks of a version 1 B-tree chunk index: their offsets (in
    /// elements, one per dataset dimension) and storage.
    pub fn chunks(&self, tree: u64, ndims: usize) -> Result<Vec<(Vec<u64>, Chunk)>> {
        let mut out = Vec::new();
        if self.undefined(tree) {
            return Ok(out);
        }
        self.chunk_tree(tree, ndims, &mut out, 0)?;
        Ok(out)
    }

    fn chunk_tree(&self, at: u64, ndims: usize, out: &mut Vec<(Vec<u64>, Chunk)>, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(corrupt("chunk B-tree too deep"));
        }
        let key = 8 + 8 * (ndims + 1);
        let (level, children) = self.btree1_node(at, 1, key)?;
        for (k, child) in children {
            if level > 0 {
                self.chunk_tree(child, ndims, out, depth + 1)?;
                continue;
            }
            let mut c = Cursor::new(&k);
            let size = c.u32()?;
            let mask = c.u32()?;
            let offsets: Vec<u64> = (0..ndims).map(|_| c.u64()).collect::<Result<_>>()?;
            out.push((offsets, Chunk { address: child, size, mask }));
        }
        Ok(())
    }
}

fn filters(b: &[u8]) -> Result<Vec<Filter>> {
    let mut c = Cursor::new(b);
    let version = c.u8()?;
    let n = c.u8()?;
    if version == 1 {
        c.skip(6)?;
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let id = c.u16()?;
        let name_len = if version == 1 || id >= 256 { c.u16()? as usize } else { 0 };
        let _flags = c.u16()?;
        let values = c.u16()? as usize;
        c.skip(if version == 1 { name_len.div_ceil(8) * 8 } else { name_len })?;
        let client: Vec<u32> = (0..values).map(|_| c.u32()).collect::<Result<_>>()?;
        if version == 1 && values % 2 == 1 {
            c.skip(4)?;
        }
        out.push(Filter { id, client });
    }
    Ok(out)
}

/// Geometry of a version 2 B-tree, for decoding its nodes.
struct Btree2 {
    node_size: u64,
    rec_size: usize,
    depth: u16,
}

impl Btree2 {
    /// Bytes needed to store `v`.
    fn bytes_for(v: u64) -> usize {
        ((64 - v.leading_zeros()) as usize).div_ceil(8).max(1)
    }

    /// Maximum records of a node at `level` (0 = leaf) and of its subtree.
    fn max_records(&self, so: usize, level: u16) -> (u64, u64) {
        let leaf = (self.node_size.saturating_sub(10)) / self.rec_size.max(1) as u64;
        let (mut node, mut total) = (leaf, leaf);
        for l in 1..=level {
            // Pointer triplets: child address, records of the child and
            // (above the twig level) records of its subtree.
            let child_bytes = Self::bytes_for(node) + if l > 1 { Self::bytes_for(total) } else { 0 };
            let ptr = so + child_bytes;
            let records = (self.node_size.saturating_sub(10 + ptr as u64)) / (self.rec_size + ptr) as u64;
            total = (records + 1).saturating_mul(total).saturating_add(records);
            node = records;
        }
        (node, total)
    }

    fn collect(&self, h: &H5, at: u64, records: u64, level: u16, f: &mut dyn FnMut(&[u8]), depth: usize) -> Result<()> {
        if depth > MAX_DEPTH || level > self.depth {
            return Err(corrupt("v2 B-tree too deep"));
        }
        let data = h.read(at, self.node_size)?;
        let sig: &[u8] = if level == 0 { b"BTLF" } else { b"BTIN" };
        if !data.starts_with(sig) {
            return Err(corrupt("v2 B-tree node without signature"));
        }
        let mut c = Cursor::new(&data);
        c.skip(6)?;
        let mut recs = Vec::with_capacity(records as usize);
        for _ in 0..records {
            recs.push(c.take(self.rec_size)?);
        }
        if level == 0 {
            recs.iter().for_each(|r| f(r));
            return Ok(());
        }
        let (child_max, child_total) = self.max_records(h.so, level - 1);
        let (n_bytes, t_bytes) = (Self::bytes_for(child_max), if level > 1 { Self::bytes_for(child_total) } else { 0 });
        for i in 0..=records as usize {
            let child = c.uint(h.so)?;
            let n = c.uint(n_bytes)?;
            c.skip(t_bytes)?;
            self.collect(h, child, n, level - 1, f, depth + 1)?;
            if let Some(r) = recs.get(i) {
                f(r);
            }
        }
        Ok(())
    }
}

/// A fractal heap: header fields needed to locate managed objects.
struct FractalHeap {
    id_len: usize,
    max_managed: u64,
    width: u64,
    start_block: u64,
    max_direct: u64,
    max_heap_bits: u32,
    root: u64,
    root_rows: u64,
    filtered: bool,
}

impl FractalHeap {
    fn open(h: &H5, at: u64) -> Result<FractalHeap> {
        let len = 22 + 12 * h.sl as u64 + 3 * h.so as u64 + 16;
        let b = h.read_upto(at, len + 8)?;
        if !b.starts_with(b"FRHP") {
            return Err(corrupt("fractal heap without signature"));
        }
        let mut c = Cursor::new(&b);
        c.skip(5)?;
        let id_len = c.u16()? as usize;
        let filter_len = c.u16()?;
        let _flags = c.u8()?;
        let max_managed = c.u32()? as u64;
        c.skip(h.sl)?; // next huge object id
        c.skip(h.so)?; // huge object B-tree
        c.skip(h.sl)?; // free space
        c.skip(h.so)?; // free space manager
        c.skip(8 * h.sl)?; // managed and huge/tiny statistics
        let width = c.u16()? as u64;
        let start_block = c.uint(h.sl)?;
        let max_direct = c.uint(h.sl)?;
        let max_heap_bits = c.u16()? as u32;
        c.skip(2)?;
        let root = c.uint(h.so)?;
        let root_rows = c.u16()? as u64;
        if width == 0 || start_block == 0 || !start_block.is_power_of_two() || !max_direct.is_power_of_two() || max_heap_bits > 64 {
            return Err(corrupt("fractal heap doubling table"));
        }
        Ok(FractalHeap { id_len, max_managed, width, start_block, max_direct, max_heap_bits, root, root_rows, filtered: filter_len > 0 })
    }

    /// Bytes of a block offset field.
    fn offset_bytes(&self) -> usize {
        (self.max_heap_bits as usize).div_ceil(8)
    }

    /// Size of the blocks of `row` in the doubling table.
    fn row_size(&self, row: u64) -> u64 {
        if row < 2 { self.start_block } else { self.start_block << (row - 1).min(62) }
    }

    /// Rows of direct blocks in any indirect block.
    fn max_direct_rows(&self) -> u64 {
        (self.max_direct.trailing_zeros() - self.start_block.trailing_zeros()) as u64 + 2
    }

    /// The object of a heap ID (managed or tiny).
    fn get(&self, h: &H5, id: &[u8]) -> Result<Vec<u8>> {
        let mut c = Cursor::new(id);
        let flags = c.u8()?;
        match (flags >> 4) & 3 {
            0 => {
                let off = c.uint(self.offset_bytes())?;
                let len_bytes = Btree2::bytes_for(self.max_direct.min(self.max_managed));
                let len = c.uint(len_bytes)?;
                self.managed(h, off, len)
            }
            2 => {
                // Tiny: the data follows in the ID.
                let (len, start) = if self.id_len <= 18 { ((flags & 0x0F) as usize + 1, 1) } else { ((((flags & 0x0F) as usize) << 8 | *id.get(1).unwrap_or(&0) as usize) + 1, 2) };
                id.get(start..start + len).map(<[u8]>::to_vec).ok_or_else(|| corrupt("tiny heap object"))
            }
            _ => Err(Error::UnsupportedVariant("HDF5 huge heap objects".into())),
        }
    }

    /// Reads `len` bytes at heap offset `off` from its direct block.
    fn managed(&self, h: &H5, off: u64, len: u64) -> Result<Vec<u8>> {
        if self.filtered {
            return Err(Error::UnsupportedVariant("HDF5 filtered fractal heaps".into()));
        }
        // Heap offsets count from the start of each direct block, header included.
        let (block, block_off) = self.locate(h, off)?;
        h.read(block + off - block_off, len)
    }

    /// Address and heap offset of the direct block holding `off`.
    fn locate(&self, h: &H5, off: u64) -> Result<(u64, u64)> {
        if self.root_rows == 0 {
            return Ok((self.root, 0));
        }
        let (mut block, mut block_off, mut rows) = (self.root, 0u64, self.root_rows);
        for _ in 0..MAX_DEPTH {
            // Find the row and column of `off` within this indirect block.
            let mut start = block_off;
            let mut entry = None;
            'rows: for row in 0..rows {
                let size = self.row_size(row);
                for col in 0..self.width {
                    if off < start + size {
                        entry = Some((row, col, start, size));
                        break 'rows;
                    }
                    start += size;
                }
            }
            let (row, col, child_off, size) = entry.ok_or_else(|| corrupt("heap offset beyond the heap"))?;
            let direct_rows = rows.min(self.max_direct_rows());
            let k = row * self.width + col;
            let header = 5 + h.so as u64 + self.offset_bytes() as u64;
            let child_at = if row < direct_rows {
                header + k * h.so as u64
            } else {
                header + direct_rows * self.width * h.so as u64 + (k - direct_rows * self.width) * h.so as u64
            };
            let child = {
                let b = h.read(block + child_at, h.so as u64)?;
                Cursor::new(&b).uint(h.so)?
            };
            if h.undefined(child) {
                return Err(corrupt("heap block not allocated"));
            }
            if row < direct_rows {
                return Ok((child, child_off));
            }
            // A child indirect block representing `size` bytes.
            block = child;
            block_off = child_off;
            rows = (size.trailing_zeros() - self.start_block.trailing_zeros()) as u64 + 1;
        }
        Err(corrupt("fractal heap too deep"))
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_bounds_and_datatypes() {
        let mut c = Cursor::new(&[1, 2, 3]);
        assert_eq!(c.uint(2).unwrap(), 0x0201);
        assert!(c.u16().is_err());
        // Fixed point, signed, 2 bytes, big endian.
        let t = datatype(&[0x10, 0x09, 0, 0, 2, 0, 0, 0]).unwrap();
        assert_eq!((t.kind, t.size, t.big_endian), (Kind::Int, 2, true));
        assert_eq!(number(&[0xFF, 0xFE], t), -2.0);
        let s = datatype(&[0x13, 0, 0, 0, 1, 0, 0, 0]).unwrap();
        assert_eq!(s.kind, Kind::String);
    }

    #[test]
    fn dataspaces() {
        // Version 2, two dims, simple.
        let mut b = vec![2, 2, 0, 1];
        b.extend_from_slice(&5u64.to_le_bytes());
        b.extend_from_slice(&7u64.to_le_bytes());
        assert_eq!(dataspace(&b, 8).unwrap(), [5, 7]);
        assert!(dataspace(&b[..10], 8).is_err());
    }

    #[test]
    fn btree2_record_counts() {
        // 512-byte nodes of 11-byte records: leaves hold 45 records.
        let t = Btree2 { node_size: 512, rec_size: 11, depth: 1 };
        assert_eq!(t.max_records(8, 0), (45, 45));
        let (node, _) = t.max_records(8, 1);
        assert!(node > 0 && node < 45);
    }
}

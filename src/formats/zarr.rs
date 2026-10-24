//! # zarr
//!
//! OME-Zarr reader: NGFF 0.1-0.4 on Zarr v2 and NGFF 0.5 on Zarr v3.
//!
//! Supports images, `bioformats2raw.layout` series and plates (one series
//! per field), multiscales levels, `omero` channel names and colours, and
//! scale transforms converted to nanometres and seconds. Arrays may use C or
//! F order, the Blosc, zlib, gzip, zstd and LZ4 compressors, the crc32c
//! codec and the `sharding_indexed` codec. Missing chunks read as the fill
//! value. Labels are not exposed.
//!
//! Provenance: OME-NGFF 0.4 and 0.5 specifications, Zarr v2 specification,
//! Zarr v3 core specification and codec specifications (sharding, blosc,
//! bytes, gzip, crc32c, transpose) in `doc/ome-zarr/`; numcodecs Blosc and
//! LZ4 documentation; own analysis of public IDR OME-Zarr data.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::codecs;
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OpenOptions, OriginalMeta, PhysicalSize, PixelType, Plane,
    PlateMeta, Region, Result, SeriesMeta, WellSample, scratch,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::OmeZarr, "OME-Zarr", "1");

/// Most dimensions an array may have.
const MAX_DIMS: usize = 8;
/// Upper bound of a decoded chunk, against corrupt metadata.
const MAX_CHUNK_BYTES: u64 = 1 << 31;

/// Root directory of the store `path` points into: the store itself or a
/// metadata file directly inside it.
fn store_root(path: &Path) -> Option<PathBuf> {
    let is_meta = |p: &Path| p.file_name().is_some_and(|n| n == ".zattrs" || n == ".zgroup" || n == "zarr.json");
    let dir = if path.is_dir() { path.to_path_buf() } else if is_meta(path) { path.parent()?.to_path_buf() } else { return None };
    (dir.join(".zattrs").exists() || dir.join(".zgroup").exists() || dir.join("zarr.json").exists()).then_some(dir)
}

/// Returns `Some(OmeZarr)` if `path` is (a metadata file of) a Zarr group
/// with OME-NGFF metadata.
pub(super) fn probe(path: &Path) -> Option<FormatId> {
    let root = store_root(path)?;
    let attrs = read_attrs(&root).ok()?;
    let ome = attrs.get("multiscales").is_some()
        || attrs.get("plate").is_some()
        || attrs.get("bioformats2raw.layout").is_some();
    ome.then_some(FormatId::OmeZarr)
}

/// Attributes of the group at `dir`: `.zattrs` (v2) or the `ome` member of
/// the `zarr.json` attributes (v3; whole attributes before NGFF 0.5).
fn read_attrs(dir: &Path) -> Result<Value> {
    let v3 = dir.join("zarr.json");
    if v3.exists() {
        let meta = read_json(&v3)?;
        let attrs = meta.get("attributes").cloned().unwrap_or(Value::Null);
        return Ok(attrs.get("ome").cloned().unwrap_or(attrs));
    }
    let v2 = dir.join(".zattrs");
    if v2.exists() { read_json(&v2) } else { Ok(Value::Null) }
}

fn read_json(path: &Path) -> Result<Value> {
    let text = std::fs::read(path)?;
    serde_json::from_slice(&text).map_err(|e| Error::Corrupt(format!("{}: {e}", path.display())))
}

/// OME-Zarr store.
#[derive(Debug)]
pub(super) struct Zarr {
    root: PathBuf,
    images: Vec<Image>,
    metas: Vec<SeriesMeta>,
    plates: Vec<PlateMeta>,
}

/// One multiscales image.
#[derive(Debug)]
struct Image {
    /// Group directory, relative to the root.
    group: PathBuf,
    axes: Axes,
    levels: Vec<Array>,
}

/// Dimension index of each axis kind.
#[derive(Debug, Clone, Copy)]
struct Axes {
    t: Option<usize>,
    c: Option<usize>,
    z: Option<usize>,
    y: usize,
    x: usize,
}

/// Compressor applied to every stored chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compressor {
    None,
    Blosc,
    Zlib,
    Gzip,
    Zstd,
    /// numcodecs LZ4: 4-byte little-endian size, then an LZ4 block.
    Lz4,
}

/// How chunk bytes become array elements.
#[derive(Debug, Clone, Copy)]
struct Pipeline {
    compressor: Compressor,
    /// A crc32c checksum follows the data (v3).
    crc: bool,
    big_endian: bool,
}

/// Sharding of a v3 array: storage objects hold several inner chunks.
#[derive(Debug, Clone)]
struct Shard {
    /// Inner chunks per shard along each dimension.
    per_shard: [u64; MAX_DIMS],
    index_at_end: bool,
    index_crc: bool,
}

#[derive(Debug)]
struct Array {
    dir: PathBuf,
    ndim: usize,
    shape: [u64; MAX_DIMS],
    /// Shape of the decodable unit: the inner chunk if sharded.
    chunk: [u64; MAX_DIMS],
    pixel_type: PixelType,
    fortran: bool,
    /// Fill value as one native element.
    fill: [u8; 8],
    /// Chunk keys: separator and prefix (`c` for the v3 default encoding).
    separator: char,
    prefix: Option<&'static str>,
    pipeline: Pipeline,
    shard: Option<Shard>,
}

impl Zarr {
    /// Opens the OME-Zarr store at or around `path`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid metadata and
    /// [`Error::UnsupportedVariant`] for codecs, data types or axis layouts
    /// this reader does not handle.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let root = store_root(path).ok_or_else(|| Error::UnsupportedFormat(format!("{}: not a Zarr group", path.display())))?;
        let attrs = read_attrs(&root)?;
        let mut images = Vec::new();
        let mut plates = Vec::new();
        if let Some(plate) = attrs.get("plate") {
            plates.push(open_plate(&root, plate, &mut images)?);
        } else if attrs.get("multiscales").is_some() {
            images.push(open_image(&root, Path::new(""), &attrs)?);
        } else if attrs.get("bioformats2raw.layout").is_some() {
            for series in 0.. {
                let group = PathBuf::from(series.to_string());
                if !root.join(&group).is_dir() {
                    break;
                }
                let attrs = read_attrs(&root.join(&group))?;
                if attrs.get("multiscales").is_some() {
                    images.push(open_image(&root, &group, &attrs)?);
                }
            }
        }
        if images.is_empty() {
            return Err(Error::Corrupt(format!("{}: no OME-NGFF image", root.display())));
        }
        let metas = images.iter().map(|i| i.meta(&root)).collect::<Result<_>>()?;
        Ok(Self { root, images, metas, plates })
    }
}

/// Opens the images of all wells and fields of a plate.
fn open_plate(root: &Path, plate: &Value, images: &mut Vec<Image>) -> Result<PlateMeta> {
    let names = |key: &str| -> Vec<String> {
        plate[key].as_array().map(|v| v.iter().filter_map(|r| r["name"].as_str().map(String::from)).collect()).unwrap_or_default()
    };
    let (rows, cols) = (names("rows"), names("columns"));
    let mut wells = Vec::new();
    for well in plate["wells"].as_array().into_iter().flatten() {
        let Some(path) = well["path"].as_str() else { continue };
        // Row and column from the indices, else from the path names.
        let mut parts = path.split('/');
        let (row_name, col_name) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let row = well["rowIndex"].as_u64().map(|v| v as u32).or_else(|| rows.iter().position(|r| r == row_name).map(|v| v as u32));
        let col = well["columnIndex"].as_u64().map(|v| v as u32).or_else(|| cols.iter().position(|c| c == col_name).map(|v| v as u32));
        let well_attrs = read_attrs(&root.join(path))?;
        for (field, image) in well_attrs["well"]["images"].as_array().into_iter().flatten().enumerate() {
            let Some(image_path) = image["path"].as_str() else { continue };
            let group = Path::new(path).join(image_path);
            if !root.join(&group).is_dir() {
                continue;
            }
            let attrs = read_attrs(&root.join(&group))?;
            images.push(open_image(root, &group, &attrs)?);
            wells.push(WellSample {
                row: row.unwrap_or(0),
                col: col.unwrap_or(0),
                field: field as u32,
                series: images.len() - 1,
                position: None,
            });
        }
    }
    Ok(PlateMeta {
        name: plate["name"].as_str().map(String::from),
        rows: rows.len() as u32,
        cols: cols.len() as u32,
        wells,
    })
}

/// Opens the levels of the multiscales image of group `group`.
fn open_image(root: &Path, group: &Path, attrs: &Value) -> Result<Image> {
    let ms = &attrs["multiscales"][0];
    let datasets = ms["datasets"].as_array().ok_or_else(|| Error::Corrupt("multiscales without datasets".into()))?;
    let mut levels = Vec::new();
    for ds in datasets {
        let path = ds["path"].as_str().ok_or_else(|| Error::Corrupt("dataset without path".into()))?;
        levels.push(open_array(&root.join(group).join(path))?);
    }
    let ndim = levels.first().ok_or_else(|| Error::Corrupt("multiscales without datasets".into()))?.ndim;
    let axes = parse_axes(&ms["axes"], ndim)?;
    if levels.iter().any(|l| l.ndim != ndim) {
        return Err(Error::Corrupt("levels differ in dimensionality".into()));
    }
    Ok(Image { group: group.to_path_buf(), axes, levels })
}

/// Axis order from `axes`: objects (0.4+), names (0.3) or the implicit
/// `tczyx` of earlier versions.
fn parse_axes(axes: &Value, ndim: usize) -> Result<Axes> {
    let names: Vec<String> = match axes.as_array() {
        Some(list) => list
            .iter()
            .map(|a| a.as_str().or_else(|| a["name"].as_str()).unwrap_or("").to_ascii_lowercase())
            .collect(),
        None => ["t", "c", "z", "y", "x"][5 - ndim.min(5)..].iter().map(|s| s.to_string()).collect(),
    };
    if names.len() != ndim {
        return Err(Error::Corrupt(format!("{} axes for {ndim} dimensions", names.len())));
    }
    let find = |n: &str| names.iter().position(|a| a == n);
    let (Some(y), Some(x)) = (find("y"), find("x")) else {
        return Err(Error::UnsupportedVariant(format!("OME-Zarr axes {names:?} without y and x")));
    };
    if let Some(other) = names.iter().find(|a| !matches!(a.as_str(), "t" | "c" | "z" | "y" | "x")) {
        return Err(Error::UnsupportedVariant(format!("OME-Zarr axis {other}")));
    }
    Ok(Axes { t: find("t"), c: find("c"), z: find("z"), y, x })
}

/// Parses the metadata of the array at `dir`.
fn open_array(dir: &Path) -> Result<Array> {
    let v3 = dir.join("zarr.json");
    if v3.exists() { open_v3(dir, &read_json(&v3)?) } else { open_v2(dir, &read_json(&dir.join(".zarray"))?) }
}

fn dims(v: &Value, what: &str) -> Result<Vec<u64>> {
    let list = v.as_array().ok_or_else(|| Error::Corrupt(format!("array without {what}")))?;
    let dims: Vec<u64> = list.iter().map(|d| d.as_u64().ok_or_else(|| Error::Corrupt(format!("invalid {what}")))).collect::<Result<_>>()?;
    if dims.is_empty() || dims.len() > MAX_DIMS {
        return Err(Error::UnsupportedVariant(format!("array with {} dimensions", dims.len())));
    }
    Ok(dims)
}

fn fixed(v: &[u64]) -> [u64; MAX_DIMS] {
    let mut a = [1; MAX_DIMS];
    a[..v.len()].copy_from_slice(v);
    a
}

fn open_v2(dir: &Path, m: &Value) -> Result<Array> {
    let shape = dims(&m["shape"], "shape")?;
    let chunk = dims(&m["chunks"], "chunks")?;
    let dtype = m["dtype"].as_str().ok_or_else(|| Error::Corrupt("array without dtype".into()))?;
    let (big_endian, pixel_type) = v2_dtype(dtype)?;
    if !m["filters"].is_null() && m["filters"].as_array().is_none_or(|f| !f.is_empty()) {
        return Err(Error::UnsupportedVariant("Zarr v2 filters".into()));
    }
    let compressor = match m["compressor"]["id"].as_str() {
        None => Compressor::None,
        Some("blosc") => Compressor::Blosc,
        Some("zlib") => Compressor::Zlib,
        Some("gzip") => Compressor::Gzip,
        Some("zstd") => Compressor::Zstd,
        Some("lz4") => Compressor::Lz4,
        Some(c) => return Err(Error::UnsupportedVariant(format!("Zarr compressor {c}"))),
    };
    let separator = match m["dimension_separator"].as_str() {
        Some("/") => '/',
        _ => '.',
    };
    finish_array(Array {
        dir: dir.to_path_buf(),
        ndim: shape.len(),
        shape: fixed(&shape),
        chunk: fixed(&chunk),
        pixel_type,
        fortran: m["order"].as_str() == Some("F"),
        fill: fill_value(&m["fill_value"], pixel_type),
        separator,
        prefix: None,
        pipeline: Pipeline { compressor, crc: false, big_endian },
        shard: None,
    }, chunk.len())
}

/// Endianness and pixel type of a Zarr v2 dtype such as `<u2`.
fn v2_dtype(d: &str) -> Result<(bool, PixelType)> {
    let big = d.starts_with('>');
    let t = match d.trim_start_matches(['<', '>', '|', '=']) {
        "u1" | "b1" => PixelType::U8,
        "i1" => PixelType::I8,
        "u2" => PixelType::U16,
        "i2" => PixelType::I16,
        "u4" => PixelType::U32,
        "i4" => PixelType::I32,
        "f4" => PixelType::F32,
        "f8" => PixelType::F64,
        other => return Err(Error::UnsupportedVariant(format!("Zarr dtype {other}"))),
    };
    Ok((big, t))
}

fn v3_dtype(d: &str) -> Result<PixelType> {
    Ok(match d {
        "uint8" | "bool" => PixelType::U8,
        "int8" => PixelType::I8,
        "uint16" => PixelType::U16,
        "int16" => PixelType::I16,
        "uint32" => PixelType::U32,
        "int32" => PixelType::I32,
        "float32" => PixelType::F32,
        "float64" => PixelType::F64,
        other => return Err(Error::UnsupportedVariant(format!("Zarr data type {other}"))),
    })
}

/// Parses a v3 codec chain: array->bytes codec (bytes or sharding) and
/// bytes->bytes codecs.
fn v3_pipeline(codecs: &Value) -> Result<(Pipeline, Option<&Value>)> {
    let mut p = Pipeline { compressor: Compressor::None, crc: false, big_endian: false };
    let mut sharding = None;
    for codec in codecs.as_array().into_iter().flatten() {
        let config = &codec["configuration"];
        match codec["name"].as_str().unwrap_or("") {
            "bytes" => p.big_endian = config["endian"].as_str() == Some("big"),
            "sharding_indexed" => sharding = Some(config),
            "blosc" => p.compressor = Compressor::Blosc,
            "gzip" => p.compressor = Compressor::Gzip,
            "zstd" => p.compressor = Compressor::Zstd,
            "crc32c" => p.crc = true,
            "transpose" => {
                // The identity order is a no-op; others are not supported.
                let order: Vec<u64> = config["order"].as_array().into_iter().flatten().filter_map(Value::as_u64).collect();
                if order.iter().enumerate().any(|(i, &o)| i as u64 != o) {
                    return Err(Error::UnsupportedVariant("Zarr transpose codec".into()));
                }
            }
            other => return Err(Error::UnsupportedVariant(format!("Zarr codec {other}"))),
        }
    }
    Ok((p, sharding))
}

fn open_v3(dir: &Path, m: &Value) -> Result<Array> {
    if m["node_type"].as_str() != Some("array") {
        return Err(Error::Corrupt(format!("{}: not an array", dir.display())));
    }
    let shape = dims(&m["shape"], "shape")?;
    let outer = dims(&m["chunk_grid"]["configuration"]["chunk_shape"], "chunk shape")?;
    let pixel_type = v3_dtype(m["data_type"].as_str().unwrap_or(""))?;
    let (separator, prefix) = match m["chunk_key_encoding"]["name"].as_str() {
        Some("v2") => (m["chunk_key_encoding"]["configuration"]["separator"].as_str().unwrap_or(".").chars().next().unwrap_or('.'), None),
        _ => (m["chunk_key_encoding"]["configuration"]["separator"].as_str().unwrap_or("/").chars().next().unwrap_or('/'), Some("c")),
    };
    let (outer_pipeline, sharding) = v3_pipeline(&m["codecs"])?;
    let (chunk, pipeline, shard) = match sharding {
        None => (outer.clone(), outer_pipeline, None),
        Some(config) => {
            if outer_pipeline.compressor != Compressor::None {
                return Err(Error::UnsupportedVariant("Zarr compression of whole shards".into()));
            }
            let inner = dims(&config["chunk_shape"], "inner chunk shape")?;
            if inner.len() != outer.len() || inner.iter().zip(&outer).any(|(i, o)| *i == 0 || o % i != 0) {
                return Err(Error::Corrupt("inner chunks do not divide the shard".into()));
            }
            let (inner_pipeline, nested) = v3_pipeline(&config["codecs"])?;
            if nested.is_some() {
                return Err(Error::UnsupportedVariant("nested Zarr sharding".into()));
            }
            let (index, _) = v3_pipeline(&config["index_codecs"])?;
            let per: Vec<u64> = outer.iter().zip(&inner).map(|(o, i)| o / i).collect();
            let shard = Shard {
                per_shard: fixed(&per),
                index_at_end: config["index_location"].as_str() != Some("start"),
                index_crc: index.crc,
            };
            (inner, inner_pipeline, Some(shard))
        }
    };
    finish_array(Array {
        dir: dir.to_path_buf(),
        ndim: shape.len(),
        shape: fixed(&shape),
        chunk: fixed(&chunk),
        pixel_type,
        fortran: false,
        fill: fill_value(&m["fill_value"], pixel_type),
        separator,
        prefix,
        pipeline,
        shard,
    }, chunk.len())
}

fn finish_array(a: Array, chunk_dims: usize) -> Result<Array> {
    if chunk_dims != a.ndim || a.chunk[..a.ndim].contains(&0) {
        return Err(Error::Corrupt(format!("{}: chunk shape does not match the array", a.dir.display())));
    }
    let elements = a.chunk[..a.ndim].iter().try_fold(1u64, |acc, &c| acc.checked_mul(c));
    if elements.and_then(|e| e.checked_mul(a.pixel_type.bytes() as u64)).is_none_or(|b| b > MAX_CHUNK_BYTES) {
        return Err(Error::UnsupportedVariant(format!("{}: chunks larger than 2 GiB", a.dir.display())));
    }
    Ok(a)
}

/// Fill value as one native element; 0 if unspecified.
fn fill_value(v: &Value, t: PixelType) -> [u8; 8] {
    let f = match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) if s == "NaN" => f64::NAN,
        Value::String(s) if s == "Infinity" => f64::INFINITY,
        Value::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        Value::Bool(b) => *b as u8 as f64,
        _ => 0.0,
    };
    let mut out = [0u8; 8];
    let int = |v: f64| v as i64;
    match t {
        PixelType::U8 => out[0] = int(f) as u8,
        PixelType::I8 => out[0] = int(f) as i8 as u8,
        PixelType::U16 => out[..2].copy_from_slice(&(int(f) as u16).to_ne_bytes()),
        PixelType::I16 => out[..2].copy_from_slice(&(int(f) as i16).to_ne_bytes()),
        PixelType::U32 => out[..4].copy_from_slice(&(int(f) as u32).to_ne_bytes()),
        PixelType::I32 => out[..4].copy_from_slice(&(int(f) as i32).to_ne_bytes()),
        PixelType::F32 => out[..4].copy_from_slice(&(f as f32).to_ne_bytes()),
        PixelType::F64 => out.copy_from_slice(&f.to_ne_bytes()),
    }
    out
}

impl Array {
    fn chunk_bytes(&self) -> usize {
        self.chunk[..self.ndim].iter().product::<u64>() as usize * self.pixel_type.bytes()
    }

    /// Path of the storage object holding chunk grid position `idx`.
    fn key(&self, idx: &[u64]) -> PathBuf {
        let mut key = String::new();
        if let Some(prefix) = self.prefix {
            key.push_str(prefix);
            key.push(self.separator);
        }
        for (i, v) in idx.iter().enumerate() {
            if i > 0 {
                key.push(self.separator);
            }
            key.push_str(&v.to_string());
        }
        self.dir.join(key)
    }

    /// Decodes chunk `idx` (in units of [`Array::chunk`]) into `out`.
    ///
    /// # Returns
    ///
    /// `false` if the chunk is not stored; `out` is then unspecified.
    fn read_chunk(&self, idx: &[u64], out: &mut [u8]) -> Result<bool> {
        let Some(shard) = &self.shard else {
            let Ok(file) = File::open(self.key(idx)) else { return Ok(false) };
            let len = file.metadata()?.len() as usize;
            return scratch::with(len, |data| {
                read_exact_at(&file, data, 0)?;
                self.decode(data, out).map(|_| true)
            });
        };
        // Shard position and inner chunk within it, in C order.
        let mut outer = [0u64; MAX_DIMS];
        let mut inner = 0u64;
        for d in 0..self.ndim {
            outer[d] = idx[d] / shard.per_shard[d];
            inner = inner * shard.per_shard[d] + idx[d] % shard.per_shard[d];
        }
        let Ok(file) = File::open(self.key(&outer[..self.ndim])) else { return Ok(false) };
        let len = file.metadata()?.len();
        let count: u64 = shard.per_shard[..self.ndim].iter().product();
        let index_len = count * 16 + if shard.index_crc { 4 } else { 0 };
        let index_start = if shard.index_at_end { len.checked_sub(index_len) } else { Some(0) };
        let index_start = index_start.ok_or_else(|| Error::Corrupt("shard shorter than its index".into()))?;
        let mut entry = [0u8; 16];
        read_exact_at(&file, &mut entry, index_start + inner * 16)?;
        let offset = u64::from_le_bytes(entry[..8].try_into().expect("8 bytes"));
        let nbytes = u64::from_le_bytes(entry[8..].try_into().expect("8 bytes"));
        if offset == u64::MAX && nbytes == u64::MAX {
            return Ok(false);
        }
        if offset.checked_add(nbytes).is_none_or(|end| end > len) {
            return Err(Error::Corrupt("shard index points beyond the shard".into()));
        }
        scratch::with(nbytes as usize, |data| {
            read_exact_at(&file, data, offset)?;
            self.decode(data, out).map(|_| true)
        })
    }

    /// Runs the codec pipeline on stored chunk bytes.
    fn decode(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        let data = if self.pipeline.crc { &data[..data.len().saturating_sub(4)] } else { data };
        let n = match self.pipeline.compressor {
            Compressor::None => {
                let n = data.len().min(out.len());
                out[..n].copy_from_slice(&data[..n]);
                n
            }
            #[cfg(feature = "blosc")]
            Compressor::Blosc => codecs::blosc::decode(data, out)?,
            #[cfg(feature = "blosc")]
            Compressor::Lz4 => {
                let body = data.get(4..).ok_or_else(|| Error::Corrupt("LZ4 chunk truncated".into()))?;
                codecs::blosc::lz4(body, out)?
            }
            #[cfg(not(feature = "blosc"))]
            Compressor::Blosc | Compressor::Lz4 => {
                return Err(Error::UnsupportedVariant("blosc/lz4 compression (enable the `blosc` feature)".into()));
            }
            Compressor::Zlib => codecs::deflate(data, out, true)?,
            Compressor::Gzip => codecs::gzip(data, out)?,
            Compressor::Zstd => codecs::zstd(data, out)?,
        };
        if n < out.len() {
            return Err(Error::Corrupt(format!("chunk decodes to {n} bytes, expected {}", out.len())));
        }
        Ok(())
    }

    /// Element strides of a chunk, in elements.
    fn strides(&self) -> [usize; MAX_DIMS] {
        let mut s = [0usize; MAX_DIMS];
        let mut acc = 1usize;
        if self.fortran {
            for d in 0..self.ndim {
                s[d] = acc;
                acc *= self.chunk[d] as usize;
            }
        } else {
            for d in (0..self.ndim).rev() {
                s[d] = acc;
                acc *= self.chunk[d] as usize;
            }
        }
        s
    }
}

impl Image {
    fn meta(&self, root: &Path) -> Result<SeriesMeta> {
        let attrs = read_attrs(&root.join(&self.group))?;
        let ms = &attrs["multiscales"][0];
        let a = self.axes;
        let first = &self.levels[0];
        let size = |d: Option<usize>| d.map_or(1, |d| first.shape[d] as u32);
        let size_c = size(a.c);
        // Pixel sizes: dataset scale times multiscales scale, in axis units.
        let mut scale = vec![1.0f64; first.ndim];
        let mut apply = |transforms: &Value| {
            for t in transforms.as_array().into_iter().flatten() {
                if t["type"].as_str() == Some("scale") {
                    for (s, v) in scale.iter_mut().zip(t["scale"].as_array().into_iter().flatten()) {
                        *s *= v.as_f64().unwrap_or(1.0);
                    }
                }
            }
        };
        apply(&ms["datasets"][0]["coordinateTransformations"]);
        apply(&ms["coordinateTransformations"]);
        let unit = |d: usize| ms["axes"][d]["unit"].as_str().unwrap_or("");
        let length = |d: Option<usize>| d.and_then(|d| length_nm(unit(d)).map(|f| f * scale[d]));
        let omero = attrs["omero"]["channels"].as_array();
        let channels = (0..size_c as usize)
            .map(|c| {
                let ch = omero.and_then(|o| o.get(c));
                ChannelMeta {
                    name: ch.and_then(|ch| ch["label"].as_str()).map(String::from),
                    color: ch.and_then(|ch| ch["color"].as_str()).and_then(hex_color),
                    ..Default::default()
                }
            })
            .collect();
        let levels = self
            .levels
            .iter()
            .map(|l| LevelMeta {
                size_x: l.shape[a.x],
                size_y: l.shape[a.y],
                tile: Some((l.chunk[a.x] as u32, l.chunk[a.y] as u32)),
                downsample: first.shape[a.x] as f64 / l.shape[a.x].max(1) as f64,
            })
            .collect();
        let name = ms["name"].as_str().map(String::from).or_else(|| {
            let g = self.group.to_string_lossy();
            (!g.is_empty()).then(|| g.into_owned())
        });
        Ok(SeriesMeta {
            name,
            size_c,
            size_z: size(a.z),
            size_t: size(a.t),
            pixel_type: first.pixel_type,
            samples_per_pixel: 1,
            levels,
            physical_size: PhysicalSize { x_nm: length(Some(a.x)), y_nm: length(Some(a.y)), z_nm: length(a.z) },
            time_increment_s: a.t.and_then(|t| time_s(unit(t)).map(|f| f * scale[t])),
            channels,
            ..Default::default()
        })
    }
}

/// Nanometres per NGFF space unit.
fn length_nm(unit: &str) -> Option<f64> {
    Some(match unit {
        "meter" => 1e9,
        "decimeter" => 1e8,
        "centimeter" => 1e7,
        "millimeter" => 1e6,
        "micrometer" | "micron" => 1e3,
        "nanometer" => 1.0,
        "picometer" => 1e-3,
        "angstrom" => 0.1,
        "inch" => 25.4e6,
        "foot" => 304.8e6,
        _ => return None,
    })
}

/// Seconds per NGFF time unit.
fn time_s(unit: &str) -> Option<f64> {
    Some(match unit {
        "second" => 1.0,
        "millisecond" => 1e-3,
        "microsecond" => 1e-6,
        "nanosecond" => 1e-9,
        "minute" => 60.0,
        "hour" => 3600.0,
        "day" => 86400.0,
        _ => return None,
    })
}

/// Parses an `RRGGBB` colour.
fn hex_color(s: &str) -> Option<[u8; 4]> {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).ok().filter(|_| s.trim_start_matches('#').len() == 6)?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255])
}

impl Backend for Zarr {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plates(&self) -> &[PlateMeta] {
        &self.plates
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        let group = &self.images[series].group;
        for (name, dir) in [("root", self.root.clone()), ("image", self.root.join(group))] {
            for file in ["zarr.json", ".zattrs"] {
                let path = dir.join(file);
                if path.exists() && (name == "root" || !group.as_os_str().is_empty()) {
                    meta.documents.push((format!("{name} {file}"), std::fs::read_to_string(path)?));
                }
            }
        }
        let ome_xml = self.root.join("OME").join("METADATA.ome.xml");
        if ome_xml.exists() {
            meta.documents.push(("OME-XML".into(), std::fs::read_to_string(ome_xml)?));
        }
        Ok(meta)
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let image = &self.images[series];
        let arr = &image.levels[level];
        let a = image.axes;
        let n = arr.ndim;
        // Fixed coordinates of the plane in every non-spatial dimension.
        let mut coord = [0u64; MAX_DIMS];
        for (axis, v) in [(a.c, plane.c), (a.z, plane.z), (a.t, plane.t)] {
            if let Some(d) = axis {
                coord[d] = v as u64;
            }
        }
        let (cx, cy) = (arr.chunk[a.x], arr.chunk[a.y]);
        let (x0, y0) = (region.x, region.y);
        let (x1, y1) = (x0 + region.width, y0 + region.height);
        let bytes = arr.pixel_type.bytes();
        let strides = arr.strides();
        let swap = arr.pipeline.big_endian != cfg!(target_endian = "big") && bytes > 1;
        let mut idx = [0u64; MAX_DIMS];
        for d in 0..n {
            idx[d] = coord[d] / arr.chunk[d];
        }
        let mut base = 0usize;
        for d in 0..n {
            if d != a.x && d != a.y {
                base += (coord[d] % arr.chunk[d]) as usize * strides[d];
            }
        }
        scratch::with(arr.chunk_bytes(), |buf| {
            for ty in y0 / cy..y1.div_ceil(cy) {
                for tx in x0 / cx..x1.div_ceil(cx) {
                    idx[a.y] = ty;
                    idx[a.x] = tx;
                    let present = arr.read_chunk(&idx[..n], buf)?;
                    let rows = y0.max(ty * cy)..y1.min((ty + 1) * cy);
                    let cols = x0.max(tx * cx)..x1.min((tx + 1) * cx);
                    let w = (cols.end - cols.start) as usize;
                    for y in rows {
                        let ly = (y - ty * cy) as usize;
                        let lx = (cols.start - tx * cx) as usize;
                        sink.write_with((y - y0) as usize, (cols.start - x0) as usize, w, |dst| {
                            if !present {
                                for e in dst.chunks_exact_mut(bytes) {
                                    e.copy_from_slice(&arr.fill[..bytes]);
                                }
                                return Ok(());
                            }
                            let start = base + ly * strides[a.y] + lx * strides[a.x];
                            if strides[a.x] == 1 {
                                dst.copy_from_slice(&buf[start * bytes..(start + w) * bytes]);
                            } else {
                                for (i, e) in dst.chunks_exact_mut(bytes).enumerate() {
                                    let at = (start + i * strides[a.x]) * bytes;
                                    e.copy_from_slice(&buf[at..at + bytes]);
                                }
                            }
                            if swap {
                                dst.chunks_exact_mut(bytes).for_each(<[u8]>::reverse);
                            }
                            Ok(())
                        })?;
                    }
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
    fn axes_forms() {
        let objects: Value = serde_json::json!([{"name": "c"}, {"name": "z"}, {"name": "y"}, {"name": "x"}]);
        let a = parse_axes(&objects, 4).unwrap();
        assert_eq!((a.t, a.c, a.z, a.y, a.x), (None, Some(0), Some(1), 2, 3));
        let implicit = parse_axes(&Value::Null, 5).unwrap();
        assert_eq!((implicit.t, implicit.x), (Some(0), 4));
        assert!(parse_axes(&serde_json::json!(["q", "y", "x"]), 3).is_err());
    }

    #[test]
    fn dtypes_and_fill() {
        assert_eq!(v2_dtype("<u2").unwrap(), (false, PixelType::U16));
        assert_eq!(v2_dtype(">f4").unwrap(), (true, PixelType::F32));
        assert_eq!(fill_value(&serde_json::json!(7), PixelType::U16)[..2], 7u16.to_ne_bytes());
        assert!(f32::from_ne_bytes(fill_value(&serde_json::json!("NaN"), PixelType::F32)[..4].try_into().unwrap()).is_nan());
        assert_eq!(hex_color("00FF80"), Some([0, 255, 128, 255]));
    }
}

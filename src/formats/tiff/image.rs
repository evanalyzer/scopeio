//! # image
//!
//! Pixel data of one TIFF image (one IFD): chunk layout, decompression,
//! predictors, sample unpacking and region reads.
//!
//! Reference: TIFF 6.0 sections 3-6, 8, 9, 13, 14, 15, 18, 21 and the
//! Adobe floating-point predictor note (see `doc/tiff/`).
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::ops::Range;
use std::sync::OnceLock;

use super::ifd::{Entry, Ifd, TiffFile, tag};
use crate::codecs;
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::scratch;
use crate::{Error, PixelType, Region, Result};

/// Compression schemes, TIFF tag 259.
mod compression {
    pub const NONE: u16 = 1;
    pub const LZW: u16 = 5;
    pub const OLD_JPEG: u16 = 6;
    pub const JPEG: u16 = 7;
    pub const DEFLATE: u16 = 8;
    pub const PACKBITS: u16 = 32773;
    pub const DEFLATE_OLD: u16 = 32946;
    pub const ZSTD: u16 = 50000;
    /// JPEG 2000 codestream (Adobe registration).
    pub const JPEG2000: u16 = 34712;
    /// Aperio JPEG 2000 with YCbCr samples, no colour transform in the stream.
    pub const JP2K_YCBCR: u16 = 33003;
    /// Aperio JPEG 2000 with RGB samples (colour transform in the stream).
    pub const JP2K_RGB: u16 = 33005;
}

/// Photometric interpretations, TIFF tag 262.
mod photometric {
    pub const MIN_IS_WHITE: u16 = 0;
    pub const MIN_IS_BLACK: u16 = 1;
    pub const RGB: u16 = 2;
    pub const PALETTE: u16 = 3;
    pub const MASK: u16 = 4;
    pub const YCBCR: u16 = 6;
    pub const CFA: u16 = 32803;
}

/// Kind of the stored samples, TIFF tag 339.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Uint,
    Int,
    Float,
}

/// One decodable TIFF image.
#[derive(Debug, Clone)]
pub(crate) struct TiffImage {
    pub width: u32,
    pub height: u32,
    /// Chunk (tile or strip) size; strips span the full width.
    pub chunk_w: u32,
    pub chunk_h: u32,
    pub tiled: bool,
    across: u32,
    down: u32,
    /// Chunk offsets and byte counts, parsed on first read.
    offsets_entry: Entry,
    counts_entry: Option<Entry>,
    chunks: usize,
    /// Table entries before this image's chunks (BIF focal planes).
    skip: usize,
    /// Added to every chunk offset (ImageJ planes following the first).
    shift: u64,
    tables: OnceLock<std::result::Result<Tables, String>>,
    compression: u16,
    predictor: u16,
    bits: u16,
    /// Samples per pixel as stored.
    pub spp: u16,
    separate: bool,
    kind: Kind,
    /// Type of the decoded stored samples; differs from `pixel_type` only
    /// for palette images expanded to RGB.
    stored: PixelType,
    /// Type of the returned samples.
    pub pixel_type: PixelType,
    /// Samples of one returned pixel: 3 or 4 for colour images, 1 if every
    /// stored sample is a gray channel of its own.
    pub samples_out: u8,
    /// Palette as 16-bit RGB entries if the indices are returned as
    /// intensities (gray or single-colour ramp palettes).
    pub lut: Option<Vec<[u16; 3]>>,
    /// Display colour of a single-colour ramp palette.
    pub display_color: Option<[u8; 4]>,
    /// RGB colour per index in native `pixel_type` samples, for palette
    /// images expanded to RGB.
    palette: Option<Vec<u8>>,
    pub photometric: u16,
    little: bool,
    fill_reversed: bool,
    invert: bool,
    jpeg_tables: Option<Vec<u8>>,
    /// Chroma subsampling of uncompressed or lossless YCbCr data to convert.
    ycbcr: Option<Ycbcr>,
    pub significant_bits: u16,
}

/// Offsets and byte counts of all chunks.
#[derive(Debug, Clone)]
struct Tables {
    offsets: Box<[u64]>,
    counts: Box<[u64]>,
}

#[derive(Debug, Clone, Copy)]
struct Ycbcr {
    h: u32,
    v: u32,
    luma: [f32; 3],
    ref_bw: [f32; 6],
}

/// Which samples a read returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Samples {
    /// All samples of a pixel, interleaved (colour images).
    All,
    /// One sample (one channel of a multi-sample gray image).
    One(u16),
}

impl TiffImage {
    /// Interprets an IFD.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] for sample layouts or photometric
    /// interpretations this reader does not handle, [`Error::Corrupt`] for
    /// inconsistent tags.
    pub fn new(tiff: &TiffFile, ifd: &Ifd) -> Result<Self> {
        Self::with_photometric(tiff, ifd, None)
    }

    /// Like [`TiffImage::new`], reading the samples as `photometric` instead
    /// of the tag's value (e.g. raw intensities of files with display
    /// palettes).
    pub fn with_photometric(tiff: &TiffFile, ifd: &Ifd, photometric_override: Option<u16>) -> Result<Self> {
        let uint = |t: u16, default: u64| -> Result<u64> { Ok(tiff.uint(ifd, t)?.unwrap_or(default)) };
        let width = uint(tag::IMAGE_WIDTH, 0)? as u32;
        let height = uint(tag::IMAGE_LENGTH, 0)? as u32;
        if width == 0 || height == 0 {
            return Err(Error::Corrupt(format!("TIFF image size {width}x{height}")));
        }
        let spp = uint(tag::SAMPLES_PER_PIXEL, 1)?.max(1) as u16;
        let bits_list = tiff.uints(ifd, tag::BITS_PER_SAMPLE)?.unwrap_or_else(|| vec![1]);
        let bits = bits_list[0] as u16;
        if bits_list.iter().any(|&b| b != bits as u64) {
            return Err(Error::UnsupportedVariant(format!("TIFF with mixed bits per sample {bits_list:?}")));
        }
        let kind = match uint(tag::SAMPLE_FORMAT, 1)? {
            2 => Kind::Int,
            3 => Kind::Float,
            _ => Kind::Uint,
        };
        let mut pixel_type = match (kind, bits) {
            (Kind::Uint, 1..=8) => PixelType::U8,
            (Kind::Uint, 9..=16) => PixelType::U16,
            (Kind::Uint, 17..=32) => PixelType::U32,
            (Kind::Int, 8) => PixelType::I8,
            (Kind::Int, 16) => PixelType::I16,
            (Kind::Int, 32) => PixelType::I32,
            (Kind::Float, 16 | 32) => PixelType::F32,
            (Kind::Float, 64) => PixelType::F64,
            (k, b) => return Err(Error::UnsupportedVariant(format!("TIFF {k:?} samples with {b} bits"))),
        };
        let compression = uint(tag::COMPRESSION, 1)? as u16;
        let default_photometric = if spp >= 3 { photometric::RGB } else { photometric::MIN_IS_BLACK };
        let photometric = match photometric_override {
            Some(p) => p,
            None => uint(tag::PHOTOMETRIC, default_photometric as u64)? as u16,
        };
        let separate = uint(tag::PLANAR_CONFIGURATION, 1)? == 2 && spp > 1;

        let stored = pixel_type;
        let mut significant_bits = bits;
        let mut lut = None;
        let mut display_color = None;
        let mut palette = None;
        let mut ycbcr = None;
        let samples_out = match photometric {
            photometric::RGB | photometric::YCBCR => {
                // JPEG and JPEG 2000 streams carry their own colour model.
                let codestream = matches!(compression, compression::JPEG | compression::JPEG2000 | compression::JP2K_YCBCR | compression::JP2K_RGB);
                if photometric == photometric::YCBCR && !codestream {
                    if bits != 8 || spp != 3 || separate {
                        return Err(Error::UnsupportedVariant("TIFF YCbCr layout other than 8-bit chunky".into()));
                    }
                    let sub = tiff.uints(ifd, tag::YCBCR_SUBSAMPLING)?.unwrap_or_else(|| vec![2, 2]);
                    let luma = tiff.floats(ifd, 529)?.unwrap_or_else(|| vec![0.299, 0.587, 0.114]);
                    let ref_bw = tiff
                        .floats(ifd, tag::REFERENCE_BLACK_WHITE)?
                        .unwrap_or_else(|| vec![0.0, 255.0, 128.0, 255.0, 128.0, 255.0]);
                    if sub.len() < 2 || luma.len() < 3 || ref_bw.len() < 6 || !matches!(sub[0], 1 | 2 | 4) || !matches!(sub[1], 1 | 2 | 4) {
                        return Err(Error::Corrupt("invalid YCbCr subsampling tags".into()));
                    }
                    ycbcr = Some(Ycbcr {
                        h: sub[0] as u32,
                        v: sub[1] as u32,
                        luma: [luma[0] as f32, luma[1] as f32, luma[2] as f32],
                        ref_bw: std::array::from_fn(|i| ref_bw[i] as f32),
                    });
                }
                match spp {
                    3 | 4 => spp as u8,
                    // Some writers (e.g. NDPI maps) label gray data RGB.
                    1 => 1,
                    n => return Err(Error::UnsupportedVariant(format!("TIFF RGB with {n} samples"))),
                }
            }
            photometric::PALETTE => {
                if spp != 1 || kind != Kind::Uint || bits > 16 {
                    return Err(Error::UnsupportedVariant(format!("TIFF palette image with {spp} samples of {bits} bits")));
                }
                let map = tiff.uints(ifd, tag::COLOR_MAP)?.unwrap_or_default();
                let n = 1usize << bits;
                if map.len() < 3 * n {
                    return Err(Error::Corrupt("TIFF palette image without valid colour map".into()));
                }
                let entries: Vec<[u16; 3]> =
                    (0..n).map(|i| [map[i] as u16, map[n + i] as u16, map[2 * n + i] as u16]).collect();
                match palette_ramp(&entries) {
                    // The indices are intensities: return them as stored.
                    Some(color) => {
                        display_color = color;
                        lut = Some(entries);
                        1
                    }
                    None => {
                        let (out, table) = palette_table(&entries);
                        pixel_type = out;
                        significant_bits = 8 * out.bytes() as u16;
                        palette = Some(table);
                        3
                    }
                }
            }
            photometric::MIN_IS_WHITE | photometric::MIN_IS_BLACK | photometric::MASK | photometric::CFA => 1,
            p => return Err(Error::UnsupportedVariant(format!("TIFF photometric interpretation {p}"))),
        };
        if compression == compression::JPEG && bits != 8 {
            return Err(Error::UnsupportedVariant(format!("JPEG-compressed TIFF with {bits} bits")));
        }

        let tiled = ifd.has(tag::TILE_WIDTH);
        let (chunk_w, chunk_h, offsets_tag, counts_tag) = if tiled {
            let tw = uint(tag::TILE_WIDTH, 0)? as u32;
            let th = uint(tag::TILE_LENGTH, 0)? as u32;
            if tw == 0 || th == 0 {
                return Err(Error::Corrupt(format!("TIFF tile size {tw}x{th}")));
            }
            // Early writers stored tile offsets in the strip tags.
            if ifd.has(tag::TILE_OFFSETS) {
                (tw, th, tag::TILE_OFFSETS, tag::TILE_BYTE_COUNTS)
            } else {
                (tw, th, tag::STRIP_OFFSETS, tag::STRIP_BYTE_COUNTS)
            }
        } else {
            let rows = uint(tag::ROWS_PER_STRIP, height as u64)?.clamp(1, height as u64) as u32;
            (width, rows, tag::STRIP_OFFSETS, tag::STRIP_BYTE_COUNTS)
        };
        let across = width.div_ceil(chunk_w);
        let down = height.div_ceil(chunk_h);
        let planes = if separate { spp as u32 } else { 1 };
        let n = (across as u64 * down as u64 * planes as u64) as usize;
        let offsets_entry = *ifd.entry(offsets_tag).ok_or_else(|| Error::Corrupt("TIFF without data offsets".into()))?;
        let counts_entry = ifd.entry(counts_tag).copied();
        if counts_entry.is_none() && (offsets_entry.count != 1 || compression != compression::NONE) {
            return Err(Error::Corrupt("TIFF without data byte counts".into()));
        }
        // Guard allocations against image sizes that do not match the tags.
        if n as u64 > offsets_entry.count.max(1).saturating_mul(16) {
            return Err(Error::Corrupt(format!("TIFF image needs {n} chunks, has {}", offsets_entry.count)));
        }

        let jpeg_tables = match ifd.entry(tag::JPEG_TABLES) {
            Some(e) if compression == compression::JPEG => Some(tiff.raw(e)?),
            _ => None,
        };
        Ok(Self {
            width,
            height,
            chunk_w,
            chunk_h,
            tiled,
            across,
            down,
            offsets_entry,
            counts_entry,
            chunks: n,
            skip: 0,
            shift: 0,
            tables: OnceLock::new(),
            compression,
            predictor: uint(tag::PREDICTOR, 1)? as u16,
            bits,
            spp,
            separate,
            kind,
            stored,
            pixel_type,
            samples_out,
            lut,
            display_color,
            palette,
            photometric,
            little: tiff.little,
            fill_reversed: uint(tag::FILL_ORDER, 1)? == 2,
            invert: photometric == photometric::MIN_IS_WHITE,
            jpeg_tables,
            ycbcr,
            significant_bits,
        })
    }

    /// Chunk offsets and byte counts, parsed on first use.
    fn tables(&self, tiff: &TiffFile) -> Result<&Tables> {
        self.tables
            .get_or_init(|| {
                let load = || -> Result<Tables> {
                    let mut offsets = tiff.entry_uints(&self.offsets_entry)?;
                    let mut counts = match &self.counts_entry {
                        Some(e) => tiff.entry_uints(e)?,
                        // Allowed for a single uncompressed chunk.
                        None => vec![tiff.len.saturating_sub(offsets.first().copied().unwrap_or(0))],
                    };
                    offsets.drain(..self.skip.min(offsets.len()));
                    counts.drain(..self.skip.min(counts.len()));
                    // Missing entries become empty chunks, which decode as zeros.
                    offsets.resize(self.chunks, 0);
                    counts.resize(self.chunks, 0);
                    if self.shift > 0 {
                        offsets.iter_mut().filter(|o| **o != 0).for_each(|o| *o += self.shift);
                    }
                    Ok(Tables { offsets: offsets.into(), counts: counts.into() })
                };
                load().map_err(|e| e.to_string())
            })
            .as_ref()
            .map_err(|e| Error::Corrupt(e.clone()))
    }

    /// Size of the pixel data if it is uncompressed and stored contiguously
    /// in chunk order.
    pub fn contiguous_size(&self, tiff: &TiffFile) -> Result<Option<u64>> {
        if self.compression != compression::NONE {
            return Ok(None);
        }
        let t = self.tables(tiff)?;
        let mut end = t.offsets[0];
        for (&o, &c) in t.offsets.iter().zip(t.counts.iter()) {
            if o != end || c == 0 {
                return Ok(None);
            }
            end += c;
        }
        Ok(Some(end - t.offsets[0]))
    }

    /// Offset of the first chunk.
    pub fn data_start(&self, tiff: &TiffFile) -> Result<u64> {
        Ok(self.tables(tiff)?.offsets[0])
    }

    /// A copy whose chunks are `delta` bytes further into the file.
    pub fn shifted(&self, delta: u64) -> Self {
        let mut img = self.clone();
        img.shift += delta;
        img.tables = OnceLock::new();
        img
    }

    /// A copy reading block `layer` of [`chunk_count`](Self::chunk_count)
    /// entries of the chunk tables (layer 0 is this image), as volumetric
    /// BIF files store their further focal planes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] if the tables are too short.
    pub fn layer(&self, layer: usize) -> Result<Self> {
        let skip = layer
            .checked_mul(self.chunks)
            .filter(|s| s.checked_add(self.chunks).is_some_and(|end| end as u64 <= self.offsets_entry.count))
            .ok_or_else(|| Error::Corrupt(format!("TIFF chunk tables without layer {layer}")))?;
        let mut img = self.clone();
        img.skip = skip;
        img.tables = OnceLock::new();
        Ok(img)
    }

    /// Number of channels this image contributes (1 for colour images).
    pub fn channels(&self) -> u16 {
        if self.samples_out == 1 { self.spp } else { 1 }
    }

    /// Returns `true` if `other` has the same pixel layout.
    pub fn same_layout(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.spp == other.spp
            && self.pixel_type == other.pixel_type
            && self.samples_out == other.samples_out
    }

    /// Returns `true` if stored rows of `samples` can be read straight into
    /// the destination: uncompressed, unpacked, native byte order.
    fn direct(&self, samples: Samples) -> bool {
        let native = self.bits == 8 || self.little == cfg!(target_endian = "little");
        self.compression == compression::NONE
            && self.predictor == 1
            && self.ycbcr.is_none()
            && self.palette.is_none()
            && !self.fill_reversed
            && !self.invert
            && self.bits.is_multiple_of(8)
            && !(self.kind == Kind::Float && self.bits == 16)
            && native
            && match samples {
                Samples::All => !self.separate,
                Samples::One(_) => self.separate || self.spp == 1,
            }
    }

    /// Delivers `region` (image coordinates, validated) to `sink`.
    ///
    /// # Arguments
    ///
    /// * `tiff` - File holding the image data.
    /// * `samples` - Samples to return; [`Samples::All`] for colour images.
    pub fn read_native(&self, tiff: &TiffFile, region: Region, samples: Samples, sink: &mut Sink) -> Result<()> {
        let t = self.tables(tiff)?;
        let (x0, y0) = (region.x as u32, region.y as u32);
        let (x1, y1) = (x0 + region.width as u32, y0 + region.height as u32);
        let in_bytes = self.stored.bytes();
        let in_samples = if self.separate { 1 } else { self.spp as usize };
        let chunk_row = self.chunk_w as usize * in_samples * in_bytes;
        let chunk_index = |plane: u16, tx: u32, ty: u32| (plane as u32 * self.down * self.across + ty * self.across + tx) as usize;
        for ty in y0 / self.chunk_h..y1.div_ceil(self.chunk_h) {
            let cy0 = ty * self.chunk_h;
            let rows = y0.max(cy0) - cy0..y1.min(cy0 + self.chunk_h) - cy0;
            for tx in x0 / self.chunk_w..x1.div_ceil(self.chunk_w) {
                let cx0 = tx * self.chunk_w;
                let cols = x0.max(cx0) - cx0..x1.min(cx0 + self.chunk_w) - cx0;
                let ox = (cx0 + cols.start - x0) as usize;
                let oy = |r: usize| (cy0 + rows.start - y0) as usize + r;
                let n = cols.len();
                if samples == Samples::All && self.separate {
                    // Planar colour: decode every plane, interleave per row.
                    let spp = self.spp as usize;
                    let plane_len = rows.len() * chunk_row;
                    scratch::with(plane_len * spp, |buf| {
                        for p in 0..spp {
                            let index = chunk_index(p as u16, tx, ty);
                            self.decode_rows(tiff, t, index, rows.clone(), &mut buf[p * plane_len..(p + 1) * plane_len])?;
                        }
                        for r in 0..rows.len() {
                            sink.write_with(oy(r), ox, n, |dst| {
                                for p in 0..spp {
                                    let src = &buf[p * plane_len + r * chunk_row..][..chunk_row];
                                    copy_pixels(src, dst, cols.clone(), 0, in_bytes, (1, 0), (spp, p));
                                }
                                Ok(())
                            })?;
                        }
                        Ok::<(), Error>(())
                    })?;
                    continue;
                }
                let (plane, pick) = match samples {
                    Samples::One(s) if self.separate => (s, 0),
                    Samples::One(s) => (0, s as usize),
                    Samples::All => (0, 0),
                };
                let index = chunk_index(plane, tx, ty);
                if self.direct(samples) {
                    // Uncompressed native rows: read straight into the destination.
                    let (offset, count) = (t.offsets[index], t.counts[index]);
                    let px = in_samples * in_bytes;
                    for r in 0..rows.len() {
                        let start = (rows.start as usize + r) * chunk_row + cols.start as usize * px;
                        sink.write_with(oy(r), ox, n, |dst| {
                            let avail = if offset == 0 { 0 } else { count.saturating_sub(start as u64).min(dst.len() as u64) as usize };
                            read_exact_at(&tiff.file, &mut dst[..avail], offset + start as u64)?;
                            dst[avail..].fill(0);
                            Ok(())
                        })?;
                    }
                    continue;
                }
                scratch::with(rows.len() * chunk_row, |buf| {
                    self.decode_rows(tiff, t, index, rows.clone(), buf)?;
                    for r in 0..rows.len() {
                        let src = &buf[r * chunk_row..][..chunk_row];
                        sink.write_with(oy(r), ox, n, |dst| {
                            if let Some(table) = &self.palette {
                                let px = self.samples_out as usize * self.pixel_type.bytes();
                                expand_palette(src, dst, cols.clone(), 0, in_bytes, table, px);
                            } else {
                                let out_samples = if samples == Samples::All { self.spp as usize } else { 1 };
                                copy_pixels(src, dst, cols.clone(), 0, in_bytes, (in_samples, pick), (out_samples, 0));
                            }
                            Ok(())
                        })?;
                    }
                    Ok::<(), Error>(())
                })?;
            }
        }
        Ok(())
    }

    /// Decodes all rows of chunk `index` of a chunky (interleaved) image
    /// into `out` (`chunk_w * chunk_h` stored pixels), for readers that
    /// place chunks themselves.
    ///
    /// # Returns
    ///
    /// `false` if the chunk is not stored (offset or size 0); `out` is then
    /// left as it is.
    pub fn decode_chunk(&self, tiff: &TiffFile, index: usize, out: &mut [u8]) -> Result<bool> {
        let t = self.tables(tiff)?;
        if index >= self.chunks || t.offsets[index] == 0 || t.counts[index] == 0 {
            return Ok(false);
        }
        if self.separate || self.palette.is_some() {
            return Err(Error::UnsupportedVariant("direct chunk decoding of planar or palette images".into()));
        }
        self.decode_rows(tiff, t, index, 0..self.chunk_h, out)?;
        Ok(true)
    }

    /// Number of chunks in the chunk tables.
    pub fn chunk_count(&self) -> usize {
        self.chunks
    }

    /// Decodes rows `rows` of chunk `index` into native stored samples:
    /// `rows.len()` rows of `chunk_w` pixels in `out`.
    fn decode_rows(&self, tiff: &TiffFile, t: &Tables, index: usize, rows: Range<u32>, out: &mut [u8]) -> Result<()> {
        let file = &tiff.file;
        let samples = if self.separate { 1 } else { self.spp as u32 };
        let row_values = self.chunk_w as usize * samples as usize;
        let (offset, count) = (t.offsets[index], t.counts[index]);
        if offset == 0 || count == 0 {
            out.fill(0);
            return Ok(());
        }
        if offset.checked_add(count).is_none_or(|end| end > tiff.len) {
            return Err(Error::Corrupt(format!("TIFF chunk {index} extends beyond the end of the file")));
        }
        // Rows actually present in this chunk; the last strip may be short.
        let chunk_row0 = (index as u32 % (self.down * self.across)) / self.across * self.chunk_h;
        let chunk_rows = if self.tiled { self.chunk_h } else { self.chunk_h.min(self.height - chunk_row0) };
        let stored_row = (row_values * self.bits as usize).div_ceil(8);

        if self.compression == compression::JPEG {
            return self.decode_jpeg(file, offset, count, rows, row_values, out);
        }
        if matches!(self.compression, compression::JPEG2000 | compression::JP2K_YCBCR | compression::JP2K_RGB) {
            return self.decode_jpeg2000(file, offset, count, rows, row_values, out);
        }
        if self.compression == compression::NONE && self.predictor == 1 && self.ycbcr.is_none() {
            // Read only the requested rows.
            let start = rows.start as usize * stored_row;
            let len = rows.len() * stored_row;
            return scratch::with(len, |raw| {
                let avail = count.saturating_sub(start as u64).min(len as u64) as usize;
                read_exact_at(file, &mut raw[..avail], offset + start as u64)?;
                raw[avail..].fill(0);
                self.finish_rows(raw, 0..rows.len(), row_values, out)
            });
        }
        scratch::with(count as usize, |src| {
            read_exact_at(file, src, offset)?;
            let rows_needed = match self.ycbcr {
                Some(y) => chunk_rows.next_multiple_of(y.v),
                None => rows.end,
            };
            scratch::with_zeroed(self.stored_len(rows_needed, row_values), |buf| {
                match self.compression {
                    compression::NONE => {
                        let n = src.len().min(buf.len());
                        buf[..n].copy_from_slice(&src[..n]);
                    }
                    compression::LZW => {
                        codecs::lzw(src, buf)?;
                    }
                    compression::DEFLATE | compression::DEFLATE_OLD => {
                        codecs::deflate(src, buf, true)?;
                    }
                    compression::PACKBITS => {
                        codecs::packbits(src, buf)?;
                    }
                    compression::ZSTD => {
                        codecs::zstd(src, buf)?;
                    }
                    compression::OLD_JPEG => {
                        return Err(Error::UnsupportedVariant("old-style JPEG TIFF compression".into()));
                    }
                    c => return Err(Error::UnsupportedVariant(format!("TIFF compression {c}"))),
                }
                if let Some(y) = self.ycbcr {
                    self.ycbcr_to_rgb(buf, y, rows.clone(), out);
                    return Ok(());
                }
                self.finish_rows(buf, rows.start as usize..rows.end as usize, row_values, out)
            })
        })
    }

    /// Converts stored rows `rows` of `stored` into native samples in `out`:
    /// fill order, predictor, unpacking, byte order and MinIsWhite.
    fn finish_rows(&self, stored: &mut [u8], rows: Range<usize>, row_values: usize, out: &mut [u8]) -> Result<()> {
        let stored_row = (row_values * self.bits as usize).div_ceil(8);
        if self.fill_reversed {
            stored[rows.start * stored_row..rows.end * stored_row].iter_mut().for_each(|b| *b = b.reverse_bits());
        }
        let samples = if self.separate { 1 } else { self.spp as usize };
        match self.predictor {
            1 | 2 => {}
            3 => return self.unshuffle_float(stored, rows, row_values, out),
            p => return Err(Error::UnsupportedVariant(format!("TIFF predictor {p}"))),
        }
        for (r, dst) in rows.zip(out.chunks_exact_mut(row_values * self.stored.bytes())) {
            self.convert_row(&stored[r * stored_row..(r + 1) * stored_row], dst);
            if self.predictor == 2 {
                undo_horizontal(dst, self.stored, samples);
            }
            if self.invert {
                self.invert_row(dst);
            }
        }
        Ok(())
    }

    /// Stored size of `rows` rows of `row_values` samples.
    fn stored_len(&self, rows: u32, row_values: usize) -> usize {
        if let Some(y) = self.ycbcr {
            let units = (self.chunk_w.div_ceil(y.h) * rows.div_ceil(y.v)) as usize;
            return units * (y.h * y.v + 2) as usize;
        }
        rows as usize * (row_values * self.bits as usize).div_ceil(8)
    }

    #[allow(clippy::too_many_arguments)]
    fn decode_jpeg(
        &self,
        file: &File,
        offset: u64,
        count: u64,
        rows: Range<u32>,
        row_values: usize,
        out: &mut [u8],
    ) -> Result<()> {
        #[cfg(feature = "jpeg")]
        {
            use codecs::jpeg::{Color, decode};
            let color = if self.photometric == photometric::YCBCR { Color::YcbcrToRgb } else { Color::AsStored };
            // The stream may be larger than the chunk (padded strips).
            let comps = if self.separate { 1 } else { self.spp as usize };
            let max_h = self.chunk_h.next_multiple_of(16) as usize;
            let max_w = self.chunk_w.next_multiple_of(16) as usize;
            scratch::with(count as usize, |src| {
                read_exact_at(file, src, offset)?;
                scratch::with(max_w * max_h * comps, |buf| {
                    let info = decode(src, self.jpeg_tables.as_deref(), color, buf)?;
                    if info.components != comps {
                        return Err(Error::Corrupt(format!("JPEG has {} components, TIFF {comps}", info.components)));
                    }
                    let src_stride = info.width * comps;
                    for (r, dst) in rows.clone().zip(out.chunks_exact_mut(row_values)) {
                        let n = row_values.min(src_stride);
                        if (r as usize) < info.height {
                            dst[..n].copy_from_slice(&buf[r as usize * src_stride..][..n]);
                            dst[n..].fill(0);
                        } else {
                            dst.fill(0);
                        }
                    }
                    Ok(())
                })
            })
        }
        #[cfg(not(feature = "jpeg"))]
        {
            let _ = (file, offset, count, rows, row_values, out);
            Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
        }
    }

    /// Decodes a JPEG 2000 chunk; Aperio 33003 chunks hold YCbCr samples.
    fn decode_jpeg2000(&self, file: &File, offset: u64, count: u64, rows: Range<u32>, row_values: usize, out: &mut [u8]) -> Result<()> {
        #[cfg(feature = "jpeg2000")]
        {
            let comps = if self.separate { 1 } else { self.spp as usize };
            let bytes = self.stored.bytes();
            let cap = self.chunk_w as usize * self.chunk_h as usize * comps * bytes;
            scratch::with(count as usize, |src| {
                read_exact_at(file, src, offset)?;
                scratch::with(cap, |buf| {
                    let info = codecs::jpeg2000::decode(src, self.stored, buf)?;
                    if info.components != comps {
                        return Err(Error::Corrupt(format!("JPEG 2000 has {} components, TIFF {comps}", info.components)));
                    }
                    if self.compression == compression::JP2K_YCBCR && comps == 3 && bytes == 1 {
                        for px in buf[..info.width * info.height * 3].chunks_exact_mut(3) {
                            let (y, cb, cr) = (px[0] as f32, px[1] as f32 - 128.0, px[2] as f32 - 128.0);
                            px[0] = (y + 1.402 * cr).round().clamp(0.0, 255.0) as u8;
                            px[1] = (y - 0.344_136 * cb - 0.714_136 * cr).round().clamp(0.0, 255.0) as u8;
                            px[2] = (y + 1.772 * cb).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                    let src_stride = info.width * comps * bytes;
                    let row_bytes = row_values * bytes;
                    for (r, dst) in rows.clone().zip(out.chunks_exact_mut(row_bytes)) {
                        let n = row_bytes.min(src_stride);
                        if (r as usize) < info.height {
                            dst[..n].copy_from_slice(&buf[r as usize * src_stride..][..n]);
                            dst[n..].fill(0);
                        } else {
                            dst.fill(0);
                        }
                    }
                    Ok(())
                })
            })
        }
        #[cfg(not(feature = "jpeg2000"))]
        {
            let _ = (file, offset, count, rows, row_values, out);
            Err(Error::UnsupportedVariant("jpeg2000 compression (enable the `jpeg2000` feature)".into()))
        }
    }

    /// Converts one stored row to native output samples.
    fn convert_row(&self, src: &[u8], dst: &mut [u8]) {
        let swap = self.little != cfg!(target_endian = "little");
        match (self.bits, self.kind) {
            (8, _) => dst.copy_from_slice(&src[..dst.len()]),
            (16 | 32 | 64, k) if !(k == Kind::Float && self.bits == 16) => {
                dst.copy_from_slice(&src[..dst.len()]);
                if swap {
                    for s in dst.chunks_exact_mut(self.bits as usize / 8) {
                        s.reverse();
                    }
                }
            }
            (16, Kind::Float) => {
                for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(2)) {
                    let h = if self.little { u16::from_le_bytes([s[0], s[1]]) } else { u16::from_be_bytes([s[0], s[1]]) };
                    d.copy_from_slice(&f16_to_f32(h).to_ne_bytes());
                }
            }
            (bits, _) => unpack_bits(src, dst, bits as u32, self.stored.bytes()),
        }
    }

    fn invert_row(&self, row: &mut [u8]) {
        match self.stored {
            PixelType::U8 => {
                let max = ((1u32 << self.bits.min(8)) - 1) as u8;
                row.iter_mut().for_each(|v| *v = max - *v);
            }
            PixelType::U16 => {
                let max = ((1u32 << self.bits) - 1) as u16;
                for s in row.chunks_exact_mut(2) {
                    let v = u16::from_ne_bytes([s[0], s[1]]);
                    s.copy_from_slice(&(max - v.min(max)).to_ne_bytes());
                }
            }
            _ => {}
        }
    }

    /// Undoes the floating-point predictor: per row, byte-wise differences
    /// over byte planes ordered most significant first.
    fn unshuffle_float(&self, stored: &[u8], rows: Range<usize>, row_values: usize, out: &mut [u8]) -> Result<()> {
        let n = self.bits as usize / 8;
        if self.kind != Kind::Float || !matches!(n, 2 | 4 | 8) {
            return Err(Error::UnsupportedVariant("floating-point predictor on non-float data".into()));
        }
        let samples = if self.separate { 1 } else { self.spp as usize };
        let row_len = row_values * n;
        let out_n = self.stored.bytes();
        scratch::with(row_len, |row| {
        for (r, dst) in rows.zip(out.chunks_exact_mut(row_values * out_n)) {
            row.copy_from_slice(&stored[r * row_len..(r + 1) * row_len]);
            for i in samples..row_len {
                row[i] = row[i].wrapping_add(row[i - samples]);
            }
            for (k, d) in dst.chunks_exact_mut(out_n).enumerate() {
                // Byte planes hold the most significant byte first.
                let mut be = [0u8; 8];
                for (b, v) in be[..n].iter_mut().enumerate() {
                    *v = row[b * row_values + k];
                }
                match n {
                    2 => d.copy_from_slice(&f16_to_f32(u16::from_be_bytes([be[0], be[1]])).to_ne_bytes()),
                    4 => d.copy_from_slice(&f32::from_be_bytes(be[..4].try_into().unwrap()).to_ne_bytes()),
                    _ => d.copy_from_slice(&f64::from_be_bytes(be).to_ne_bytes()),
                }
            }
        }
        });
        Ok(())
    }

    /// Converts subsampled YCbCr data units to RGB rows `rows`.
    fn ycbcr_to_rgb(&self, stored: &[u8], y: Ycbcr, rows: Range<u32>, out: &mut [u8]) {
        let w = self.chunk_w as usize;
        out.fill(0);
        let units_across = self.chunk_w.div_ceil(y.h) as usize;
        let unit = (y.h * y.v + 2) as usize;
        let [kr, kg, kb] = y.luma;
        let rb = y.ref_bw;
        // TIFF 6.0 section 21: scale codes with ReferenceBlackWhite, then invert
        // the luma/chroma transform.
        let scale = |v: u8, lo: f32, hi: f32, range: f32| (v as f32 - lo) * range / (hi - lo);
        for (oy, row) in rows.clone().enumerate() {
            for x in 0..w {
                let u = (row / y.v) as usize * units_across + x / y.h as usize;
                let base = u * unit;
                let Some(data) = stored.get(base..base + unit) else { continue };
                let luma = data[((row % y.v) * y.h) as usize + x % y.h as usize];
                let l = scale(luma, rb[0], rb[1], 255.0);
                let cb = scale(data[unit - 2], rb[2], rb[3], 127.0);
                let cr = scale(data[unit - 1], rb[4], rb[5], 127.0);
                let r = cr * (2.0 - 2.0 * kr) + l;
                let b = cb * (2.0 - 2.0 * kb) + l;
                let g = (l - kb * b - kr * r) / kg;
                let o = (oy * w + x) * 3;
                out[o] = r.round().clamp(0.0, 255.0) as u8;
                out[o + 1] = g.round().clamp(0.0, 255.0) as u8;
                out[o + 2] = b.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Classifies a palette by its entries.
///
/// Returns `Some` if every entry is the last entry scaled by a
/// non-decreasing factor, i.e. the indices are intensities of one colour:
/// `Some(None)` for a gray ramp, `Some(Some(rgba))` with the colour of the
/// brightest entry otherwise. Returns `None` for any other palette.
fn palette_ramp(entries: &[[u16; 3]]) -> Option<Option<[u8; 4]>> {
    // One step of an 8-bit palette scaled to 16 bits.
    const TOLERANCE: f64 = 257.0;
    let top = *entries.last()?;
    let m = (0..3).max_by_key(|&k| top[k])?;
    if top[m] == 0 {
        return None;
    }
    let mut prev = 0;
    for e in entries {
        if e[m] < prev {
            return None;
        }
        prev = e[m];
        let t = e[m] as f64 / top[m] as f64;
        if (0..3).any(|k| (e[k] as f64 - t * top[k] as f64).abs() > TOLERANCE) {
            return None;
        }
    }
    let gray = top.iter().all(|&v| v.abs_diff(top[m]) as f64 <= TOLERANCE);
    Some((!gray).then(|| [(top[0] >> 8) as u8, (top[1] >> 8) as u8, (top[2] >> 8) as u8, 255]))
}

/// Builds the RGB lookup table of a palette in native output samples.
///
/// Uses `U8` if every 16-bit entry holds an 8-bit value in its high byte,
/// as written by the common conventions `v * 257`, `v << 8` and
/// `v << 8 | 0xFF`, otherwise `U16`.
fn palette_table(entries: &[[u16; 3]]) -> (PixelType, Vec<u8>) {
    let values = entries.iter().flatten();
    if values.clone().all(|&v| matches!(v & 0xFF, 0 | 0xFF) || v & 0xFF == v >> 8) {
        (PixelType::U8, values.map(|&v| (v >> 8) as u8).collect())
    } else {
        (PixelType::U16, values.flat_map(|&v| v.to_ne_bytes()).collect())
    }
}

/// Maps the palette indices in columns `cols` of `src` to their colours in
/// `table` (`px` bytes per index), written from pixel `ox` of `dst`.
fn expand_palette(src: &[u8], dst: &mut [u8], cols: Range<u32>, ox: usize, index_bytes: usize, table: &[u8], px: usize) {
    for (i, c) in cols.enumerate() {
        let s = &src[c as usize * index_bytes..][..index_bytes];
        let index = if index_bytes == 1 { s[0] as usize } else { u16::from_ne_bytes([s[0], s[1]]) as usize };
        // Indices have at most 16 bits and the table one entry for each.
        if let Some(entry) = table.get(index * px..(index + 1) * px) {
            let to = (ox + i) * px;
            dst[to..to + px].copy_from_slice(entry);
        }
    }
}

/// Copies columns `cols` of one decoded chunk row into an output row.
///
/// Pixels have `in_samples` samples in `src` and `out_samples` in `dst`;
/// `pick` is the first source sample and `put` the first destination
/// sample copied, `min(in_samples, out_samples)` samples per pixel.
#[allow(clippy::too_many_arguments)]
fn copy_pixels(
    src: &[u8],
    dst: &mut [u8],
    cols: Range<u32>,
    ox: usize,
    bytes: usize,
    (in_samples, pick): (usize, usize),
    (out_samples, put): (usize, usize),
) {
    let c0 = cols.start as usize;
    let n = cols.len();
    if in_samples == out_samples {
        let px = in_samples * bytes;
        dst[ox * px..(ox + n) * px].copy_from_slice(&src[c0 * px..(c0 + n) * px]);
        return;
    }
    let k = in_samples.min(out_samples) * bytes;
    for i in 0..n {
        let from = ((c0 + i) * in_samples + pick) * bytes;
        let to = ((ox + i) * out_samples + put) * bytes;
        dst[to..to + k].copy_from_slice(&src[from..from + k]);
    }
}

/// Undoes horizontal differencing (TIFF 6.0 section 14) on native samples.
fn undo_horizontal(row: &mut [u8], pixel_type: PixelType, samples: usize) {
    macro_rules! undo {
        ($t:ty) => {{
            let n = size_of::<$t>();
            for i in samples..row.len() / n {
                let prev = <$t>::from_ne_bytes(row[(i - samples) * n..][..n].try_into().unwrap());
                let cur = <$t>::from_ne_bytes(row[i * n..][..n].try_into().unwrap());
                row[i * n..][..n].copy_from_slice(&cur.wrapping_add(prev).to_ne_bytes());
            }
        }};
    }
    match pixel_type.bytes() {
        1 => undo!(u8),
        2 => undo!(u16),
        4 => undo!(u32),
        _ => undo!(u64),
    }
}

/// Unpacks MSB-first packed samples of `bits` bits into `out_bytes` wide
/// native integers.
fn unpack_bits(src: &[u8], dst: &mut [u8], bits: u32, out_bytes: usize) {
    let mask = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut acc: u64 = 0;
    let mut have = 0u32;
    let mut bytes = src.iter();
    for d in dst.chunks_exact_mut(out_bytes) {
        while have < bits {
            acc = (acc << 8) | *bytes.next().unwrap_or(&0) as u64;
            have += 8;
        }
        have -= bits;
        let v = (acc >> have) as u32 & mask;
        match out_bytes {
            1 => d[0] = v as u8,
            2 => d.copy_from_slice(&(v as u16).to_ne_bytes()),
            _ => d.copy_from_slice(&v.to_ne_bytes()),
        }
    }
}

/// Converts an IEEE 754 half-precision value.
fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) as u32) << 31;
    let exp = ((h >> 10) & 0x1F) as u32;
    let frac = (h & 0x3FF) as u32;
    let bits = match exp {
        0 if frac == 0 => sign,
        0 => {
            // Subnormal: normalise.
            let shift = frac.leading_zeros() - 21;
            sign | ((113 - shift) << 23) | ((frac << (shift + 13)) & 0x7F_FFFF)
        }
        0x1F => sign | 0x7F80_0000 | (frac << 13),
        e => sign | ((e + 112) << 23) | (frac << 13),
    };
    f32::from_bits(bits)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_msb_first_bits() {
        let mut out = [0u8; 8];
        unpack_bits(&[0b1011_0010], &mut out, 1, 1);
        assert_eq!(out, [1, 0, 1, 1, 0, 0, 1, 0]);
        let mut out = [0u8; 4];
        // Two 12-bit samples 0xABC, 0xDEF.
        unpack_bits(&[0xAB, 0xCD, 0xEF], &mut out, 12, 2);
        assert_eq!([u16::from_ne_bytes([out[0], out[1]]), u16::from_ne_bytes([out[2], out[3]])], [0xABC, 0xDEF]);
    }

    #[test]
    fn horizontal_predictor_per_sample() {
        let mut row = [10u8, 20, 1, 2, 1, 2];
        undo_horizontal(&mut row, PixelType::U8, 2);
        assert_eq!(row, [10, 20, 11, 22, 12, 24]);
    }

    #[test]
    fn palette_classification() {
        let gray: Vec<[u16; 3]> = (0..16).map(|i| [i * 0x1111; 3]).collect();
        assert_eq!(palette_ramp(&gray), Some(None));
        let green: Vec<[u16; 3]> = (0..256).map(|i| [0, i * 257, 0]).collect();
        assert_eq!(palette_ramp(&green), Some(Some([0, 255, 0, 255])));
        let mixed: Vec<[u16; 3]> = (0..256).map(|i| [i * 257, (255 - i) * 257, 0]).collect();
        assert_eq!(palette_ramp(&mixed), None);
        let falling: Vec<[u16; 3]> = (0..256).rev().map(|i| [i * 257; 3]).collect();
        assert_eq!(palette_ramp(&falling), None);
        assert_eq!(palette_ramp(&[[0; 3]; 4]), None);

        assert_eq!(palette_table(&[[0, 257, 65535]]), (PixelType::U8, vec![0, 1, 255]));
        assert_eq!(palette_table(&[[0, 256, 65280]]), (PixelType::U8, vec![0, 1, 255]));
        assert_eq!(palette_table(&[[0, 0x0BFF, 0xFEFF]]), (PixelType::U8, vec![0, 11, 254]));
        let (t, table) = palette_table(&[[0, 1000, 65535]]);
        assert_eq!((t, table.len()), (PixelType::U16, 6));
    }

    #[test]
    fn half_floats() {
        assert_eq!(f16_to_f32(0x3C00), 1.0);
        assert_eq!(f16_to_f32(0xC000), -2.0);
        assert_eq!(f16_to_f32(0x0001), 2f32.powi(-24));
        assert!(f16_to_f32(0x7C00).is_infinite());
    }
}

//! # bmp
//!
//! Windows bitmap (BMP/DIB) reader.
//!
//! Supports the core, info and V2-V5 headers with 1, 2, 4, 8, 16, 24 and
//! 32 bits per pixel, uncompressed, bit-field and RLE4/RLE8 compressed data.
//! Palette images with a gray palette are returned as gray, other palette
//! images are expanded to RGB. Packed 16- and
//! 32-bit pixels are scaled to 8 bits per sample.
//!
//! Provenance: Microsoft "Bitmap Storage", "Bitmap Compression" and the
//! `BITMAP*HEADER` structure pages (see `doc/bmp/`); own analysis of sample
//! files. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::Path;

use super::dib::{
    self, BI_ALPHABITFIELDS, BI_BITFIELDS, BI_RGB, BI_RLE4, BI_RLE8, Lut, Pixels, decode_rle, original_meta, packed_masks, parse_header, u32_at,
};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::scratch;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Bmp, "Windows Bitmap", "1").extensions(&["bmp"]);

/// Returns `true` if `header` starts like a Windows bitmap file.
pub(super) fn matches(header: &[u8]) -> bool {
    header.len() >= 18 && &header[..2] == b"BM" && matches!(u32_at(header, 14), 12 | 40 | 52 | 56 | 64 | 108 | 124)
}

const FILE_HEADER_SIZE: u64 = 14;
/// Upper bound of the scratch buffer used per read call.
const CHUNK_BYTES: usize = 1 << 20;

/// Reader for Windows bitmap files.
#[derive(Debug)]
pub(super) struct Bmp {
    file: File,
    series: [SeriesMeta; 1],
    pixels: Pixels,
    data_offset: u64,
    data_len: u64,
    stride: usize,
    top_down: bool,
    /// RLE-compressed data, decoded on every read.
    rle: bool,
}

impl Bmp {
    /// Opens a bitmap file and parses its headers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read, [`Error::UnsupportedVariant`]
    /// for JPEG/PNG payloads, OS/2 Huffman data and masks wider than 8 bits,
    /// and [`Error::Corrupt`] for invalid headers or truncated data.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let file_len = file.metadata()?.len();
        let mut head = [0u8; 14 + 124 + 16];
        let head_len = head.len().min(file_len as usize);
        read_exact_at(&file, &mut head[..head_len], 0)?;
        if !matches(&head[..head_len]) {
            return Err(Error::Corrupt("not a BMP file".into()));
        }
        let data_offset = u32_at(&head, 10) as u64;
        let h = parse_header(&head[14..head_len])?;

        if h.width <= 0 || h.height == 0 || h.height == i32::MIN {
            return Err(Error::Corrupt(format!("invalid size {}x{}", h.width, h.height)));
        }
        let width = h.width as u32;
        let height = h.height.unsigned_abs();
        let top_down = h.height < 0;
        let bits = h.bits;

        let rle = matches!(h.compression, BI_RLE8 | BI_RLE4);
        if rle && top_down {
            return Err(Error::Corrupt("compressed bitmaps must be bottom-up".into()));
        }
        // Colour masks follow a 40-byte header; larger headers contain them.
        let mut masks = h.masks;
        let mut table_offset = FILE_HEADER_SIZE + h.size as u64;
        if h.size == 40 && matches!(h.compression, BI_BITFIELDS | BI_ALPHABITFIELDS) {
            let n = if h.compression == BI_ALPHABITFIELDS { 4 } else { 3 };
            let mut buf = [0u8; 16];
            read_exact_at(&file, &mut buf[..n * 4], table_offset)?;
            for (i, m) in masks.iter_mut().enumerate().take(n) {
                *m = u32_at(&buf, i * 4);
            }
            table_offset += n as u64 * 4;
        }

        let pixels = match (bits, h.compression) {
            // OS/2 2.x headers reuse codes 3 and 4 for Huffman and RLE24.
            (_, c @ (BI_BITFIELDS | 4)) if h.size == 64 => {
                return Err(Error::UnsupportedVariant(format!("OS/2 bitmap compression {c}")));
            }
            (1 | 2 | 4 | 8, BI_RGB) | (8, BI_RLE8) | (4, BI_RLE4) => {
                let entry = if h.size == 12 { 3 } else { 4 };
                // Some writers leave the colour count at 0 with a shorter table;
                // never read the table into the pixel data.
                let room = data_offset.saturating_sub(table_offset) / entry as u64;
                let lut = read_palette(&file, table_offset, entry, bits, h.colors_used, room)?;
                Pixels::Indexed { bits: bits as u8, lut }
            }
            (24, BI_RGB) => Pixels::Bgr24,
            (16 | 32, BI_RGB) => {
                let rgb = if bits == 16 { [0x7C00, 0x03E0, 0x001F] } else { [0xFF_0000, 0xFF00, 0xFF] };
                Pixels::Packed { bytes: (bits / 8) as u8, masks: packed_masks(&rgb, 0)? }
            }
            (16 | 32, BI_BITFIELDS | BI_ALPHABITFIELDS) => {
                let alpha = if h.size >= 56 || h.compression == BI_ALPHABITFIELDS { masks[3] } else { 0 };
                Pixels::Packed { bytes: (bits / 8) as u8, masks: packed_masks(&masks[..3], alpha)? }
            }
            (_, 4 | 5) => {
                return Err(Error::UnsupportedVariant("BMP with embedded JPEG or PNG data".into()));
            }
            (b, c) => {
                return Err(Error::Corrupt(format!("invalid bit count {b} for compression {c}")));
            }
        };

        let stride = (width as usize * bits as usize).div_ceil(32) * 4;
        let data_len = if rle {
            let available = file_len.saturating_sub(data_offset);
            if h.image_size > 0 { available.min(h.image_size as u64) } else { available }
        } else {
            let needed = stride as u64 * height as u64;
            if data_offset + needed > file_len {
                return Err(Error::Corrupt(format!("pixel data needs {needed} bytes at offset {data_offset}, file has {file_len}")));
            }
            needed
        };

        let samples_per_pixel = dib::samples_per_pixel(&pixels);
        let nm_per_pixel = |ppm: i32| (ppm > 0).then(|| 1e9 / ppm as f64);
        let meta = SeriesMeta {
            size_c: 1,
            size_z: 1,
            size_t: 1,
            pixel_type: PixelType::U8,
            samples_per_pixel,
            levels: vec![LevelMeta { size_x: width as u64, size_y: height as u64, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: nm_per_pixel(h.x_ppm), y_nm: nm_per_pixel(h.y_ppm), z_nm: None },
            channels: vec![ChannelMeta::default()],
            ..Default::default()
        };

        Ok(Self { file, series: [meta], pixels, data_offset, data_len, stride, top_down, rle })
    }

    /// Decodes the palette indices of an RLE file, top row first.
    fn rle_indices(&self) -> Result<Vec<u8>> {
        let Pixels::Indexed { bits, .. } = self.pixels else {
            return Err(Error::Corrupt("RLE data without palette".into()));
        };
        let level = &self.series[0].levels[0];
        scratch::with(self.data_len as usize, |data| {
            read_exact_at(&self.file, data, self.data_offset)?;
            Ok(decode_rle(data, bits, level.size_x as usize, level.size_y as usize))
        })
    }
}

impl Backend for Bmp {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.series
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut head = [0u8; 124];
        let len = head.len().min(self.file.metadata()?.len().saturating_sub(14) as usize);
        read_exact_at(&self.file, &mut head[..len], 14)?;
        let h = parse_header(&head[..len])?;
        Ok(OriginalMeta { pairs: original_meta(&h, self.top_down), ..Default::default() })
    }

    fn read_native(&self, _series: usize, _level: usize, _plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let width = self.series[0].levels[0].size_x as usize;
        let height = self.series[0].levels[0].size_y as usize;
        let (x, y0, w, rows) = (region.x as usize, region.y as usize, region.width as usize, region.height as usize);

        if self.rle {
            let indices = self.rle_indices()?;
            let Pixels::Indexed { lut, .. } = &self.pixels else { unreachable!("checked in rle_indices") };
            let samples = if lut.gray { 1 } else { 3 };
            for r in 0..rows {
                let src = &indices[(y0 + r) * width + x..][..w];
                sink.write_with(r, 0, w, |dst| {
                    for (out, &i) in dst.chunks_exact_mut(samples).zip(src) {
                        out.copy_from_slice(&lut.rgb[i as usize][..samples]);
                    }
                    Ok(())
                })?;
            }
            return Ok(());
        }

        // Rows of the region are contiguous in the file; read them in chunks.
        let chunk_rows = (CHUNK_BYTES / self.stride).clamp(1, rows);
        scratch::with(chunk_rows * self.stride, |scratch| {
            let mut done = 0;
            while done < rows {
                let n = chunk_rows.min(rows - done);
                let first = y0 + done;
                // Index of the lowest stored row in this chunk.
                let stored = if self.top_down { first } else { height - first - n };
                let block = &mut scratch[..n * self.stride];
                read_exact_at(&self.file, block, self.data_offset + (stored * self.stride) as u64)?;
                for r in 0..n {
                    let s = if self.top_down { r } else { n - 1 - r };
                    let src = &block[s * self.stride..(s + 1) * self.stride];
                    sink.write_with(done + r, 0, w, |dst| {
                        self.pixels.convert_row(src, x, dst);
                        Ok(())
                    })?;
                }
                done += n;
            }
            Ok(())
        })
    }
}

/// Reads the colour table and pads it to 256 entries.
///
/// At most `room` entries are read if the table would otherwise overlap
/// the pixel data.
fn read_palette(file: &File, offset: u64, entry: usize, bits: u16, used: u32, room: u64) -> Result<Lut> {
    let max = 1usize << bits;
    let count = if used == 0 || used as usize > max { max } else { used as usize };
    let count = if room > 0 { count.min(room as usize) } else { count };
    let mut raw = vec![0u8; count * entry];
    read_exact_at(file, &mut raw, offset)?;
    Ok(dib::palette(&raw, entry, bits))
}

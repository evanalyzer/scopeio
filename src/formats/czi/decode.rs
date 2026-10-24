//! # decode
//!
//! Decoding of CZI subblock data into stored-size pixels, colour ordered
//! R, G, B(, A).
//!
//! Compression ids: 0 none, 1 JPEG, 2 LZW, 4 JPEG XR, 5 zstd, 6 zstd with
//! a header (optional hi/lo byte split), 7 chunked zstd/LZ4. Decoded
//! JPEG/JPEG XR bitmaps whose size or sample type disagrees with the
//! directory entry are brought to the entry's geometry (libCZI resolution
//! protocol: crop or zero-pad at the top left, convert 8/16-bit samples).
//!
//! Provenance: OpenReadout CZI format notes (MIT OR Apache-2.0) and the
//! libCZI documentation pages "chunked compression" and "resolution
//! protocol" (MIT, docs only) in `doc/zeiss-czi/`. No GPL/LGPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Error, PixelType, Result, codecs, scratch};

/// Sample layout of a CZI pixel type id: sample type, samples per pixel.
pub(super) fn layout(pixel_type: u32) -> Result<(PixelType, u8)> {
    Ok(match pixel_type {
        0 => (PixelType::U8, 1),
        1 => (PixelType::U16, 1),
        2 => (PixelType::F32, 1),
        3 => (PixelType::U8, 3),
        4 => (PixelType::U16, 3),
        8 => (PixelType::F32, 3),
        9 => (PixelType::U8, 4),
        12 => (PixelType::I32, 1),
        13 => (PixelType::F64, 1),
        10 | 11 => return Err(Error::UnsupportedVariant("CZI complex pixel types".into())),
        t => return Err(Error::Corrupt(format!("CZI pixel type {t}"))),
    })
}

/// Geometry and sample layout of a subblock as declared by its entry.
#[derive(Debug, Clone, Copy)]
pub(super) struct Geometry {
    pub width: usize,
    pub height: usize,
    pub pixel_type: PixelType,
    pub spp: usize,
}

impl Geometry {
    pub fn pixel_bytes(&self) -> usize {
        self.spp * self.pixel_type.bytes()
    }

    pub fn len(&self) -> usize {
        self.width * self.height * self.pixel_bytes()
    }
}

/// Swaps the first and third sample of each colour pixel (BGR to RGB).
pub(super) fn bgr_to_rgb(row: &mut [u8], g: &Geometry) {
    if g.spp < 3 {
        return;
    }
    let n = g.pixel_type.bytes();
    for px in row.chunks_exact_mut(g.pixel_bytes()) {
        for k in 0..n {
            px.swap(k, 2 * n + k);
        }
    }
}

/// Decodes subblock data `src` compressed with `compression` into `out`
/// (`g.len()` bytes), colour ordered RGB.
pub(super) fn decode(src: &[u8], compression: u32, g: &Geometry, out: &mut [u8]) -> Result<()> {
    debug_assert_eq!(out.len(), g.len());
    let stored_bgr = match compression {
        0 => {
            // Shorter data is padded with zeros, longer data cut.
            let n = src.len().min(out.len());
            out[..n].copy_from_slice(&src[..n]);
            out[n..].fill(0);
            true
        }
        1 => {
            jpeg(src, g, out)?;
            false
        }
        2 => {
            let n = codecs::lzw(src, out)?;
            out[n..].fill(0);
            true
        }
        4 => return jpeg_xr(src, g, out),
        5 => {
            let n = codecs::zstd(src, out)?;
            out[n..].fill(0);
            true
        }
        6 => {
            zstd1(src, g, out)?;
            true
        }
        7 => {
            chunked(src, g, out)?;
            true
        }
        3 => return Err(Error::UnsupportedVariant("CZI lossless JPEG compression".into())),
        c if c >= 100 => return Err(Error::UnsupportedVariant(format!("CZI camera or system raw compression {c}"))),
        c => return Err(Error::Corrupt(format!("CZI compression {c}"))),
    };
    if stored_bgr {
        bgr_to_rgb(out, g);
    }
    Ok(())
}

/// Reverses the hi/lo byte split of 16-bit samples: `src` holds all low
/// bytes, then all high bytes.
fn unshuffle_hilo(src: &[u8], out: &mut [u8]) {
    let n = src.len().min(out.len()) / 2;
    let (lo, hi) = src[..2 * n].split_at(n);
    for (i, d) in out.chunks_exact_mut(2).take(n).enumerate() {
        d[0] = lo[i];
        d[1] = hi[i];
    }
    out[2 * n..].fill(0);
}

/// zstd with a header: byte 0 is the header size; header chunk type 1
/// carries a flag byte whose bit 0 marks the hi/lo split.
fn zstd1(src: &[u8], g: &Geometry, out: &mut [u8]) -> Result<()> {
    let header = *src.first().ok_or_else(|| Error::Corrupt("CZI zstd1 data empty".into()))? as usize;
    let body = src.get(header.max(1)..).ok_or_else(|| Error::Corrupt("CZI zstd1 header truncated".into()))?;
    let mut hilo = false;
    let mut pos = 1;
    while pos < header {
        match src[pos] {
            1 => {
                hilo = src.get(pos + 1).is_some_and(|f| f & 1 == 1);
                pos += 2;
            }
            t => return Err(Error::UnsupportedVariant(format!("CZI zstd1 header chunk {t}"))),
        }
    }
    if hilo && g.pixel_type.bytes() == 2 {
        return scratch::with(out.len(), |tmp| {
            let n = codecs::zstd(body, tmp)?;
            unshuffle_hilo(&tmp[..n], out);
            Ok(())
        });
    }
    let n = codecs::zstd(body, out)?;
    out[n..].fill(0);
    Ok(())
}

/// Little-endian base-128 varint at `pos`.
fn varint(b: &[u8], pos: &mut usize) -> Result<u64> {
    let mut v = 0u64;
    for shift in (0..35).step_by(7) {
        let byte = *b.get(*pos).ok_or_else(|| Error::Corrupt("CZI chunked header truncated".into()))?;
        *pos += 1;
        v |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(Error::Corrupt("CZI chunked header varint too long".into()))
}

/// Chunked compression (id 7): a header of (id, length, payload) entries,
/// then the compressed chunks back to back.
fn chunked(src: &[u8], g: &Geometry, out: &mut [u8]) -> Result<()> {
    let mut pos = 0;
    let (mut sizes, mut decoded, mut codec, mut hilo) = (Vec::new(), Vec::new(), 0u8, false);
    loop {
        let id = varint(src, &mut pos)?;
        if id == 0 {
            break;
        }
        let len = varint(src, &mut pos)? as usize;
        let payload = src.get(pos..pos + len).ok_or_else(|| Error::Corrupt("CZI chunked header truncated".into()))?;
        pos += len;
        let mut p = 0;
        let mut list = || -> Result<Vec<u64>> {
            let mut v = Vec::new();
            while p < payload.len() {
                v.push(varint(payload, &mut p)?);
            }
            Ok(v)
        };
        match id {
            1 => sizes = list()?,
            3 => decoded = list()?,
            2 => codec = *payload.first().unwrap_or(&0),
            4 => hilo = payload.first() == Some(&1),
            id => return Err(Error::UnsupportedVariant(format!("CZI chunked header id {id}"))),
        }
    }
    if sizes.is_empty() || decoded.is_empty() {
        return Err(Error::Corrupt("CZI chunked data without chunk sizes".into()));
    }
    // Decoded sizes: one per chunk, or the listed values for the last
    // chunks with the first one repeated before them.
    let n = sizes.len();
    let k = decoded.len();
    let decoded_size = |i: usize| -> u64 {
        if k >= n {
            decoded[i]
        } else if i + k >= n {
            decoded[i + k - n]
        } else {
            decoded[0]
        }
    };
    let run = |dst: &mut [u8]| -> Result<()> {
        let mut at = 0usize;
        let mut written = 0usize;
        for (i, &size) in sizes.iter().enumerate() {
            let chunk = src.get(pos + at..pos + at + size as usize).ok_or_else(|| Error::Corrupt("CZI chunk beyond the data".into()))?;
            at += size as usize;
            let want = decoded_size(i) as usize;
            let dst = dst.get_mut(written..written + want).ok_or_else(|| Error::Corrupt("CZI chunks larger than the subblock".into()))?;
            let got = match codec {
                0 => codecs::zstd(chunk, dst)?,
                1 => codecs::lz4(chunk, dst)?,
                c => return Err(Error::UnsupportedVariant(format!("CZI chunk codec {c}"))),
            };
            if got != want {
                return Err(Error::Corrupt(format!("CZI chunk decodes to {got} bytes, expected {want}")));
            }
            written += want;
        }
        if written != dst.len() {
            return Err(Error::Corrupt(format!("CZI chunks decode to {written} bytes, expected {}", dst.len())));
        }
        Ok(())
    };
    if hilo && g.pixel_type.bytes() == 2 {
        return scratch::with(out.len(), |tmp| {
            run(tmp)?;
            unshuffle_hilo(tmp, out);
            Ok(())
        });
    }
    run(out)
}

/// Copies a decoded bitmap of `w` x `h` pixels with `px` bytes each into
/// `out` of geometry `g`, cropping or zero-padding at the top left.
fn conform_size(src: &[u8], w: usize, h: usize, g: &Geometry, out: &mut [u8]) {
    let px = g.pixel_bytes();
    let (cw, ch) = (w.min(g.width), h.min(g.height));
    out.fill(0);
    for y in 0..ch {
        out[y * g.width * px..][..cw * px].copy_from_slice(&src[y * w * px..][..cw * px]);
    }
}

fn jpeg(src: &[u8], g: &Geometry, out: &mut [u8]) -> Result<()> {
    if g.pixel_type != PixelType::U8 || !matches!(g.spp, 1 | 3) {
        return Err(Error::UnsupportedVariant(format!("CZI JPEG with {:?} x {}", g.pixel_type, g.spp)));
    }
    #[cfg(feature = "jpeg")]
    {
        use codecs::jpeg::{Color, decode};
        // Decode straight into `out` when the stream has the declared size.
        let cap = g.width.max(1) * g.height.max(1) * 3;
        scratch::with(cap.max(out.len()) * 2, |tmp| {
            let info = decode(src, None, Color::YcbcrToRgb, tmp)?;
            if info.components != g.spp {
                return Err(Error::UnsupportedVariant(format!("CZI JPEG with {} components for {} samples", info.components, g.spp)));
            }
            conform_size(tmp, info.width, info.height, g, out);
            Ok(())
        })
    }
    #[cfg(not(feature = "jpeg"))]
    {
        let _ = (src, out);
        Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
    }
}

#[cfg(feature = "jpegxr")]
fn jpeg_xr(src: &[u8], g: &Geometry, out: &mut [u8]) -> Result<()> {
    use openreadout_jpegxr::SampleType;
    // Working memory limit of the decoder: a generous multiple of the tile.
    let limit = (g.len() * 8).max(64 << 20);
    let img = openreadout_jpegxr::decode(src, limit).map_err(|e| Error::Corrupt(format!("JPEG XR: {e}")))?;
    let (w, h, ch) = (img.width as usize, img.height as usize, img.channels as usize);
    let src_bytes = match img.sample {
        SampleType::U8 => 1,
        SampleType::U16 => 2,
        s => return Err(Error::UnsupportedVariant(format!("CZI JPEG XR sample type {s:?}"))),
    };
    let dst_bytes = g.pixel_type.bytes();
    if !matches!(g.pixel_type, PixelType::U8 | PixelType::U16) {
        return Err(Error::UnsupportedVariant(format!("CZI JPEG XR for {:?}", g.pixel_type)));
    }
    // Samples per pixel taken from the bitmap: gray is replicated to colour,
    // alpha and extra samples are dropped.
    let take = ch.min(g.spp);
    if ch == 0 || !(take == g.spp || ch == 1) {
        return Err(Error::UnsupportedVariant(format!("CZI JPEG XR with {ch} channels for {} samples", g.spp)));
    }
    let same = src_bytes == dst_bytes && ch == g.spp && !(img.bgr && ch >= 3);
    if same && w == g.width && h == g.height {
        out.copy_from_slice(&img.data[..out.len()]);
        return Ok(());
    }
    // Convert sample by sample into the declared layout, then size.
    let px = g.pixel_bytes();
    scratch::with(w * h * px, |conv| {
        for (i, d) in conv.chunks_exact_mut(px).enumerate() {
            let s = &img.data[i * ch * src_bytes..][..ch * src_bytes];
            for k in 0..g.spp {
                // Colour order of the bitmap: BGR formats swap R and B.
                let from = if ch == 1 {
                    0
                } else if img.bgr && k < 3 {
                    2 - k
                } else {
                    k
                };
                let v = match src_bytes {
                    1 => s[from] as u16,
                    _ => u16::from_le_bytes([s[2 * from], s[2 * from + 1]]),
                };
                // 16 to 8 bits keeps the high byte; 8 to 16 bits extends.
                match (src_bytes, dst_bytes) {
                    (2, 1) => d[k] = (v >> 8) as u8,
                    (_, 1) => d[k] = v as u8,
                    _ => d[2 * k..2 * k + 2].copy_from_slice(&v.to_le_bytes()),
                }
            }
        }
        conform_size(conv, w, h, g, out);
        Ok(())
    })
}

#[cfg(not(feature = "jpegxr"))]
fn jpeg_xr(_src: &[u8], _g: &Geometry, _out: &mut [u8]) -> Result<()> {
    Err(Error::UnsupportedVariant("jpegxr compression (enable the `jpegxr` feature)".into()))
}

/// Valid-pixel mask of a subblock attachment: one bit per pixel, most
/// significant bit first, 1 = valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Mask {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bits: Vec<u8>,
}

/// GUID of the mask chunk, as stored.
const MASK_GUID: [u8; 16] = [0x67, 0xEA, 0xE3, 0xCB, 0xFC, 0x5B, 0x2B, 0x49, 0xA1, 0x6A, 0xEC, 0xE3, 0x78, 0x03, 0x14, 0x48];

impl Mask {
    /// Parses the chunk container of a subblock attachment; `None` if it
    /// holds no usable mask or the mask marks every pixel valid.
    pub fn parse(b: &[u8]) -> Option<Mask> {
        // Chunks: GUID, i32 size, payload. Some writers put 16 bytes first.
        for start in [0usize, 16] {
            let mut pos = start;
            while let Some(head) = b.get(pos..pos + 20) {
                let size = i32::from_le_bytes(head[16..20].try_into().ok()?).max(0) as usize;
                let payload = b.get(pos + 20..pos + 20 + size)?;
                if head[..16] == MASK_GUID {
                    return Self::from_payload(payload);
                }
                pos += 20 + size;
            }
        }
        None
    }

    fn from_payload(p: &[u8]) -> Option<Mask> {
        let u = |i: usize| p.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as usize);
        let (width, height, repr, stride) = (u(0)?, u(4)?, u(8)?, u(12)?);
        if repr != 0 || stride < width.div_ceil(8) {
            return None;
        }
        let bits = p.get(16..16 + stride.checked_mul(height)?)?.to_vec();
        let all = (0..height).all(|y| (0..width).all(|x| bits[y * stride + x / 8] & (0x80 >> (x % 8)) != 0));
        (!all).then_some(Mask { width, height, stride, bits })
    }

    /// Whether pixel (`x`, `y`) of the subblock is valid; outside the mask
    /// pixels are valid.
    pub fn valid(&self, x: usize, y: usize) -> bool {
        x >= self.width || y >= self.height || self.bits[y * self.stride + x / 8] & (0x80 >> (x % 8)) != 0
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn g(width: usize, height: usize, pixel_type: PixelType, spp: usize) -> Geometry {
        Geometry { width, height, pixel_type, spp }
    }

    #[test]
    fn raw_bgr_is_swapped_and_padded() {
        let geo = g(2, 1, PixelType::U8, 3);
        let mut out = [9u8; 6];
        decode(&[1, 2, 3, 4], 0, &geo, &mut out).unwrap();
        assert_eq!(out, [3, 2, 1, 0, 0, 4]);
    }

    #[test]
    fn hilo_unshuffle() {
        let mut out = [0u8; 4];
        unshuffle_hilo(&[0x01, 0x02, 0xA0, 0xB0], &mut out);
        assert_eq!(out, [0x01, 0xA0, 0x02, 0xB0]);
    }

    #[test]
    fn varints_and_sizes() {
        let mut p = 0;
        assert_eq!(varint(&[0x96, 0x01], &mut p).unwrap(), 150);
        assert_eq!(p, 2);
        assert!(varint(&[0x80], &mut 0).is_err());
    }

    #[test]
    fn chunked_lz4_two_chunks() {
        let geo = g(3, 1, PixelType::U16, 1);
        // Two LZ4 literal-only blocks of 4 and 2 bytes.
        let c0 = [0x40, 1, 2, 3, 4];
        let c1 = [0x20, 5, 6];
        let mut src = vec![1, 2, 5, 3, 3, 2, 4, 2, 2, 1, 1, 0];
        src.extend_from_slice(&c0);
        src.extend_from_slice(&c1);
        let mut out = [0u8; 6];
        decode(&src, 7, &geo, &mut out).unwrap();
        assert_eq!(out, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn masks() {
        let mut b = MASK_GUID.to_vec();
        let payload: Vec<u8> = [2u32, 2, 0, 1].iter().flat_map(|v| v.to_le_bytes()).chain([0b1000_0000, 0b1100_0000]).collect();
        b.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        b.extend(payload);
        let m = Mask::parse(&b).unwrap();
        assert!(m.valid(0, 0) && !m.valid(1, 0) && m.valid(1, 1) && m.valid(5, 5));
    }
}

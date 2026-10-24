//! # codecs
//!
//! Decompression of chunk data, shared by all readers.
//!
//! Each decoder writes into a caller-provided buffer of the expected
//! decoded size and returns the number of bytes written. Codecs are behind
//! cargo features; disabled ones return [`Error::Unsupported`].
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#[cfg(feature = "blosc")]
pub(crate) mod blosc;
#[cfg(feature = "jpeg")]
pub(crate) mod jpeg;
#[cfg(feature = "jpeg2000")]
pub(crate) mod jpeg2000;

use crate::{Error, Result};

#[allow(dead_code)]
fn disabled(name: &str) -> Error {
    Error::UnsupportedVariant(format!("{name} compression (enable the `{name}` feature)"))
}

/// Decodes PackBits run-length data (Apple TN1023, TIFF 6.0 section 9).
///
/// Stops when `out` is full or the input ends.
pub(crate) fn packbits(src: &[u8], out: &mut [u8]) -> Result<usize> {
    let (mut i, mut o) = (0, 0);
    while i < src.len() && o < out.len() {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            let len = (n as usize + 1).min(out.len() - o).min(src.len() - i);
            out[o..o + len].copy_from_slice(&src[i..i + len]);
            i += n as usize + 1;
            o += len;
        } else if n != -128 {
            let Some(&v) = src.get(i) else { break };
            let len = (1 - n as isize) as usize;
            let len = len.min(out.len() - o);
            out[o..o + len].fill(v);
            i += 1;
            o += len;
        }
    }
    Ok(o)
}

/// Decodes TIFF LZW data (TIFF 6.0 section 13), including the pre-6.0
/// variant with LSB-first codes.
#[cfg(feature = "lzw")]
pub(crate) fn lzw(src: &[u8], out: &mut [u8]) -> Result<usize> {
    use weezl::{BitOrder, LzwStatus, decode::Decoder};
    // Old-style streams start with a clear code written LSB first.
    let old_style = src.len() >= 2 && src[0] == 0 && src[1] & 1 == 1;
    let mut decoder = if old_style {
        Decoder::new(BitOrder::Lsb, 8)
    } else {
        Decoder::with_tiff_size_switch(BitOrder::Msb, 8)
    };
    let (mut i, mut o) = (0, 0);
    while o < out.len() {
        let r = decoder.decode_bytes(&src[i..], &mut out[o..]);
        i += r.consumed_in;
        o += r.consumed_out;
        match r.status {
            Ok(LzwStatus::Ok) if r.consumed_in == 0 && r.consumed_out == 0 => break,
            Ok(LzwStatus::Ok) => {}
            Ok(LzwStatus::Done | LzwStatus::NoProgress) => break,
            // Some writers end strips without an end code or with garbage after
            // the image data; keep what was decoded.
            Err(_) if o > 0 => break,
            Err(e) => return Err(Error::Corrupt(format!("LZW: {e}"))),
        }
    }
    Ok(o)
}

#[cfg(not(feature = "lzw"))]
pub(crate) fn lzw(_src: &[u8], _out: &mut [u8]) -> Result<usize> {
    Err(disabled("lzw"))
}

/// Decodes a zlib stream (RFC 1950), or raw Deflate data (RFC 1951) if
/// `zlib_header` is false.
#[cfg(feature = "deflate")]
pub(crate) fn deflate(src: &[u8], out: &mut [u8], zlib_header: bool) -> Result<usize> {
    use flate2::{Decompress, FlushDecompress, Status};
    let mut d = Decompress::new(zlib_header);
    loop {
        let before = d.total_out();
        let status = d
            .decompress(&src[d.total_in() as usize..], &mut out[d.total_out() as usize..], FlushDecompress::Finish)
            .map_err(|e| Error::Corrupt(format!("Deflate: {e}")))?;
        match status {
            Status::StreamEnd => break,
            _ if d.total_out() as usize == out.len() || d.total_out() == before => break,
            _ => {}
        }
    }
    Ok(d.total_out() as usize)
}

#[cfg(not(feature = "deflate"))]
pub(crate) fn deflate(_src: &[u8], _out: &mut [u8], _zlib_header: bool) -> Result<usize> {
    Err(disabled("deflate"))
}

/// Decodes a zlib stream of unknown decoded size, at most `limit` bytes.
#[cfg(feature = "deflate")]
pub(crate) fn inflate_vec(src: &[u8], limit: usize) -> Result<Vec<u8>> {
    use flate2::{Decompress, FlushDecompress, Status};
    let mut d = Decompress::new(true);
    let mut out = Vec::with_capacity((src.len() * 4).min(limit));
    loop {
        if out.len() == out.capacity() {
            if out.len() >= limit {
                return Err(Error::Corrupt(format!("zlib stream decodes to more than {limit} bytes")));
            }
            out.reserve((out.len().max(4096)).min(limit - out.len()));
        }
        let status = d
            .decompress_vec(&src[d.total_in() as usize..], &mut out, FlushDecompress::Finish)
            .map_err(|e| Error::Corrupt(format!("Deflate: {e}")))?;
        if status == Status::StreamEnd || d.total_in() as usize >= src.len() && out.len() < out.capacity() {
            return Ok(out);
        }
    }
}

#[cfg(not(feature = "deflate"))]
pub(crate) fn inflate_vec(_src: &[u8], _limit: usize) -> Result<Vec<u8>> {
    Err(disabled("deflate"))
}

/// Decodes an LZ4 block (no frame) of known decoded size.
#[cfg(feature = "lz4")]
pub(crate) fn lz4(src: &[u8], out: &mut [u8]) -> Result<usize> {
    lz4_flex::block::decompress_into(src, out).map_err(|e| Error::Corrupt(format!("LZ4: {e}")))
}

#[cfg(not(feature = "lz4"))]
pub(crate) fn lz4(_src: &[u8], _out: &mut [u8]) -> Result<usize> {
    Err(disabled("lz4"))
}

/// Decodes a gzip member (RFC 1952): skips the header, inflates the data.
pub(crate) fn gzip(src: &[u8], out: &mut [u8]) -> Result<usize> {
    let bad = || Error::Corrupt("invalid gzip header".into());
    if src.len() < 18 || src[0] != 0x1F || src[1] != 0x8B || src[2] != 8 {
        return Err(bad());
    }
    let flags = src[3];
    let mut pos = 10;
    if flags & 0x04 != 0 {
        let xlen = u16::from_le_bytes([*src.get(pos).ok_or_else(bad)?, *src.get(pos + 1).ok_or_else(bad)?]) as usize;
        pos += 2 + xlen;
    }
    for flag in [0x08, 0x10] {
        if flags & flag != 0 {
            // Zero-terminated file name or comment.
            pos += src.get(pos..).ok_or_else(bad)?.iter().position(|&b| b == 0).ok_or_else(bad)? + 1;
        }
    }
    if flags & 0x02 != 0 {
        pos += 2;
    }
    deflate(src.get(pos..).ok_or_else(bad)?, out, false)
}

/// Decodes Zstandard frames (RFC 8878).
#[cfg(feature = "zstd-c")]
pub(crate) fn zstd(src: &[u8], out: &mut [u8]) -> Result<usize> {
    zstd_c_lib::bulk::decompress_to_buffer(src, out).map_err(|e| Error::Corrupt(format!("Zstandard: {e}")))
}

/// Decodes Zstandard frames (RFC 8878).
#[cfg(all(feature = "zstd", not(feature = "zstd-c")))]
pub(crate) fn zstd(src: &[u8], out: &mut [u8]) -> Result<usize> {
    use std::cell::RefCell;
    thread_local! {
        // Reused per thread: a new decoder allocates its window buffers.
        static DECODER: RefCell<ruzstd::decoding::FrameDecoder> = RefCell::new(ruzstd::decoding::FrameDecoder::new());
    }
    DECODER.with(|d| d.borrow_mut().decode_all(src, out)).map_err(|e| Error::Corrupt(format!("Zstandard: {e}")))
}

#[cfg(not(any(feature = "zstd", feature = "zstd-c")))]
pub(crate) fn zstd(_src: &[u8], _out: &mut [u8]) -> Result<usize> {
    Err(disabled("zstd"))
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packbits_tn1023_example() {
        // Example from Apple TN1023.
        let src = [0xFE, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0xFD, 0xAA, 0x03, 0x80, 0x00, 0x2A, 0x22, 0xF7, 0xAA];
        let mut out = [0u8; 24];
        assert_eq!(packbits(&src, &mut out).unwrap(), 24);
        assert_eq!(
            out,
            [
                0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0xAA, 0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x22, 0xAA,
                0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA
            ]
        );
    }

    #[cfg(feature = "deflate")]
    #[test]
    fn deflate_stored_block() {
        // zlib header, one stored block "abc", Adler-32.
        let src = [0x78, 0x01, 0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c', 0x02, 0x4D, 0x01, 0x27];
        let mut out = [0u8; 3];
        assert_eq!(deflate(&src, &mut out, true).unwrap(), 3);
        assert_eq!(&out, b"abc");
    }

    #[cfg(feature = "deflate")]
    #[test]
    fn gzip_stored_block_with_name() {
        // Header with FNAME "a", one stored block "hi", CRC-32 and size.
        let mut src = vec![0x1F, 0x8B, 8, 0x08, 0, 0, 0, 0, 0, 3, b'a', 0];
        src.extend_from_slice(&[0x01, 0x02, 0x00, 0xFD, 0xFF, b'h', b'i']);
        src.extend_from_slice(&[0xAC, 0x2A, 0x93, 0xD8, 2, 0, 0, 0]);
        let mut out = [0u8; 2];
        assert_eq!(gzip(&src, &mut out).unwrap(), 2);
        assert_eq!(&out, b"hi");
    }

    #[cfg(any(feature = "zstd", feature = "zstd-c"))]
    #[test]
    fn zstd_raw_block() {
        // Frame header with single segment and 3-byte content, one raw last block.
        let src = [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x03, 0x19, 0x00, 0x00, b'x', b'y', b'z'];
        let mut out = [0u8; 3];
        assert_eq!(zstd(&src, &mut out).unwrap(), 3);
        assert_eq!(&out, b"xyz");
    }
}

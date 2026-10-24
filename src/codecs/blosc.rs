//! # blosc
//!
//! Decoder of the Blosc 1 chunk format: header, block table, optional
//! splits, an inner codec per split and the byte-shuffle filter.
//!
//! Provenance: c-blosc1 `README_CHUNK_FORMAT.rst` (`doc/codecs/blosc/`),
//! numcodecs Blosc documentation; checked on public OME-Zarr data.
//! No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::{Error, Result, scratch};

const HEADER: usize = 16;
const FLAG_SHUFFLE: u8 = 0x01;
const FLAG_MEMCPY: u8 = 0x02;
const FLAG_BITSHUFFLE: u8 = 0x04;
const FLAG_DONT_SPLIT: u8 = 0x10;

fn u32_at(b: &[u8], at: usize) -> Result<usize> {
    b.get(at..at + 4)
        .map(|v| u32::from_le_bytes(v.try_into().expect("4 bytes")) as usize)
        .ok_or_else(|| Error::Corrupt("Blosc chunk truncated".into()))
}

/// Decodes a Blosc chunk into `out`, returning the number of bytes written.
///
/// # Errors
///
/// Returns [`Error::Corrupt`] for invalid chunks or if `out` is too small,
/// [`Error::UnsupportedVariant`] for bit-shuffle and the blosclz and snappy
/// codecs.
pub(crate) fn decode(src: &[u8], out: &mut [u8]) -> Result<usize> {
    if src.len() < HEADER {
        return Err(Error::Corrupt("Blosc chunk shorter than its header".into()));
    }
    let (flags, typesize) = (src[2], src[3].max(1) as usize);
    let nbytes = u32_at(src, 4)?;
    let blocksize = u32_at(src, 8)?;
    if nbytes > out.len() {
        return Err(Error::Corrupt(format!("Blosc chunk holds {nbytes} bytes, expected at most {}", out.len())));
    }
    if flags & FLAG_MEMCPY != 0 {
        let data = src.get(HEADER..HEADER + nbytes).ok_or_else(|| Error::Corrupt("Blosc chunk truncated".into()))?;
        out[..nbytes].copy_from_slice(data);
        return Ok(nbytes);
    }
    if nbytes == 0 {
        return Ok(0);
    }
    if blocksize == 0 {
        return Err(Error::Corrupt("Blosc block size 0".into()));
    }
    if flags & FLAG_BITSHUFFLE != 0 {
        return Err(Error::UnsupportedVariant("Blosc bit-shuffle".into()));
    }
    let codec = flags >> 5;
    let nblocks = nbytes.div_ceil(blocksize);
    let shuffle = flags & FLAG_SHUFFLE != 0 && typesize > 1;
    scratch::with(blocksize, |tmp| {
        for j in 0..nblocks {
            let bsize = blocksize.min(nbytes - j * blocksize);
            let leftover = bsize < blocksize;
            // Block starts are offsets from the start of the chunk.
            let mut pos = u32_at(src, HEADER + 4 * j)?;
            let nsplits = if flags & FLAG_DONT_SPLIT == 0 && !leftover && bsize % typesize == 0 { typesize } else { 1 };
            let split = bsize / nsplits;
            let dst = if shuffle { &mut tmp[..bsize] } else { &mut out[j * blocksize..j * blocksize + bsize] };
            for s in 0..nsplits {
                let csize = u32_at(src, pos)?;
                pos += 4;
                let data = src.get(pos..pos + csize).ok_or_else(|| Error::Corrupt("Blosc split truncated".into()))?;
                let part = &mut dst[s * split..(s + 1) * split];
                if csize == split {
                    part.copy_from_slice(data);
                } else {
                    let n = inner(codec, data, part)?;
                    if n != split {
                        return Err(Error::Corrupt(format!("Blosc split decodes to {n} bytes, expected {split}")));
                    }
                }
                pos += csize;
            }
            if shuffle {
                unshuffle(&tmp[..bsize], &mut out[j * blocksize..j * blocksize + bsize], typesize);
            }
        }
        Ok(nbytes)
    })
}

/// Decodes one split with the codec numbered `codec` in the header flags.
fn inner(codec: u8, src: &[u8], out: &mut [u8]) -> Result<usize> {
    match codec {
        1 => super::lz4(src, out),
        3 => super::deflate(src, out, true),
        4 => super::zstd(src, out),
        0 => Err(Error::UnsupportedVariant("Blosc blosclz codec".into())),
        2 => Err(Error::UnsupportedVariant("Blosc snappy codec".into())),
        c => Err(Error::Corrupt(format!("Blosc codec {c}"))),
    }
}

/// Reverses the byte shuffle: `src` holds byte `k` of all elements, then
/// byte `k + 1`, and so on; trailing bytes are not shuffled.
fn unshuffle(src: &[u8], out: &mut [u8], typesize: usize) {
    let n = src.len() / typesize;
    for (i, element) in out.chunks_exact_mut(typesize).take(n).enumerate() {
        for (k, b) in element.iter_mut().enumerate() {
            *b = src[k * n + i];
        }
    }
    let tail = n * typesize;
    out[tail..src.len()].copy_from_slice(&src[tail..]);
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memcpy_chunk() {
        let mut src = vec![2, 1, FLAG_MEMCPY, 1];
        src.extend_from_slice(&3u32.to_le_bytes());
        src.extend_from_slice(&3u32.to_le_bytes());
        src.extend_from_slice(&19u32.to_le_bytes());
        src.extend_from_slice(b"abc");
        let mut out = [0u8; 3];
        assert_eq!(decode(&src, &mut out).unwrap(), 3);
        assert_eq!(&out, b"abc");
    }

    #[test]
    fn stored_splits_with_shuffle() {
        // Two u16 elements 0x0201, 0x0403 shuffled into [01 03 02 04], one
        // block, two stored splits of two bytes each.
        let mut src = vec![2, 1, FLAG_SHUFFLE, 2];
        src.extend_from_slice(&4u32.to_le_bytes());
        src.extend_from_slice(&4u32.to_le_bytes());
        src.extend_from_slice(&32u32.to_le_bytes());
        src.extend_from_slice(&20u32.to_le_bytes()); // block 0 starts after the table
        src.extend_from_slice(&2u32.to_le_bytes());
        src.extend_from_slice(&[1, 3]);
        src.extend_from_slice(&2u32.to_le_bytes());
        src.extend_from_slice(&[2, 4]);
        let mut out = [0u8; 4];
        assert_eq!(decode(&src, &mut out).unwrap(), 4);
        assert_eq!(out, [1, 2, 3, 4]);
    }

    #[test]
    fn unshuffle_keeps_tail() {
        let mut out = [0u8; 5];
        unshuffle(&[1, 3, 2, 4, 9], &mut out, 2);
        assert_eq!(out, [1, 2, 3, 4, 9]);
    }
}

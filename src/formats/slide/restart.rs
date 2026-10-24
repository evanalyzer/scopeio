//! # restart
//!
//! JPEG streams with restart markers read one restart interval at a time,
//! as Hamamatsu NDPI stores its pyramid levels.
//!
//! Every restart interval of one MCU row becomes a tile: the stream's
//! header with the frame size set to the tile, the interval's entropy data
//! and an end marker. DC predictions reset at each restart marker (ITU-T
//! T.81, F.2.1.3.1), so the intervals decode independently.
//!
//! Provenance: ITU-T T.81; openslide.org Hamamatsu format page (prose,
//! CC BY-SA 4.0) for the NDPI offset tags. No GPL/LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use crate::formats::tiff::ifd::{Ifd, TiffFile, tag};
use crate::formats::tiff::image::TiffImage;
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::{Error, Region, Result, scratch};

/// NDPI: offsets of the restart intervals (low 32 bits, high 32 bits).
const TAG_MCU_STARTS: u16 = 65426;
const TAG_MCU_STARTS_HIGH: u16 = 65432;

/// A JPEG stream with restart markers, read one restart interval at a time.
#[derive(Debug)]
pub(crate) struct RestartJpeg {
    /// File offset of the stream.
    start: u64,
    len: u64,
    /// SOI up to and including the SOS header, frame size set to one tile.
    header: Box<[u8]>,
    /// Offsets of each interval's entropy data, relative to `start`.
    offsets: Box<[u64]>,
    pub tile_w: u32,
    pub tile_h: u32,
    across: u32,
    width: u32,
    height: u32,
    components: usize,
    ycbcr: bool,
}

/// Fields of a JPEG header up to its scan.
struct JpegHeader {
    /// Offset of the SOF segment's frame size (height at +0, width at +2).
    sof_size_at: usize,
    mcu_w: u32,
    mcu_h: u32,
    components: usize,
    restart_interval: u32,
    /// Offset of the first entropy-coded byte.
    data_at: usize,
}

fn parse_header(b: &[u8]) -> Option<JpegHeader> {
    if b.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut pos = 2;
    let (mut sof, mut interval) = (None, 0u32);
    loop {
        while *b.get(pos)? == 0xFF && *b.get(pos + 1)? == 0xFF {
            pos += 1;
        }
        let marker = *b.get(pos + 1)?;
        let len = u16::from_be_bytes([*b.get(pos + 2)?, *b.get(pos + 3)?]) as usize;
        let seg = b.get(pos + 4..pos + 2 + len)?;
        match marker {
            // Baseline and extended sequential Huffman frames.
            0xC0 | 0xC1 => {
                let n = *seg.get(5)? as usize;
                let (mut hmax, mut vmax) = (1u32, 1u32);
                for c in 0..n {
                    let f = *seg.get(7 + 3 * c)?;
                    hmax = hmax.max((f >> 4) as u32);
                    vmax = vmax.max((f & 15) as u32);
                }
                sof = Some((pos + 5, hmax * 8, vmax * 8, n));
            }
            // Progressive, lossless and arithmetic frames.
            0xC2 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return None,
            0xDD => interval = u16::from_be_bytes([*seg.first()?, *seg.get(1)?]) as u32,
            0xDA => {
                let (sof_size_at, mcu_w, mcu_h, components) = sof?;
                return Some(JpegHeader { sof_size_at, mcu_w, mcu_h, components, restart_interval: interval, data_at: pos + 2 + len });
            }
            _ => {}
        }
        pos += 2 + len;
    }
}

impl RestartJpeg {
    /// Index of the restart intervals of an NDPI level, or `None` if its
    /// stream cannot be read interval by interval.
    pub fn ndpi(tiff: &TiffFile, ifd: &Ifd, image: &TiffImage) -> Result<Option<Self>> {
        let (Some(start), Some(len)) = (tiff.uint(ifd, tag::STRIP_OFFSETS)?, tiff.uint(ifd, tag::STRIP_BYTE_COUNTS)?) else {
            return Ok(None);
        };
        let Some(low) = tiff.uints(ifd, TAG_MCU_STARTS)? else { return Ok(None) };
        if start.checked_add(len).is_none_or(|e| e > tiff.len) || len < 4 {
            return Err(Error::Corrupt("NDPI JPEG stream beyond the end of the file".into()));
        }
        let mut head = vec![0u8; len.min(1 << 16) as usize];
        read_exact_at(&tiff.file, &mut head, start)?;
        let Some(h) = parse_header(&head) else { return Ok(None) };
        if h.restart_interval == 0 {
            return Ok(None);
        }
        if h.components != image.spp as usize || image.pixel_type != crate::PixelType::U8 {
            return Err(Error::Corrupt(format!("NDPI JPEG with {} components for {} samples", h.components, image.spp)));
        }
        let mcus_x = image.width.div_ceil(h.mcu_w);
        let mcus_y = image.height.div_ceil(h.mcu_h);
        if mcus_x % h.restart_interval != 0 {
            return Ok(None);
        }
        let across = mcus_x / h.restart_interval;
        if low.len() as u64 != across as u64 * mcus_y as u64 {
            return Ok(None);
        }
        let high = tiff.uints(ifd, TAG_MCU_STARTS_HIGH)?.filter(|v| v.len() == low.len());
        let offsets: Box<[u64]> = low.iter().enumerate().map(|(i, &l)| l | high.as_ref().map_or(0, |h| h[i] << 32)).collect();
        if offsets.windows(2).any(|w| w[1] <= w[0]) || offsets.last().is_some_and(|&o| o >= len) {
            return Err(Error::Corrupt("NDPI restart offsets out of order".into()));
        }
        let tile_w = h.restart_interval * h.mcu_w;
        let mut header = head[..h.data_at].to_vec();
        header[h.sof_size_at..h.sof_size_at + 2].copy_from_slice(&(h.mcu_h as u16).to_be_bytes());
        header[h.sof_size_at + 2..h.sof_size_at + 4].copy_from_slice(&(tile_w.min(u16::MAX as u32) as u16).to_be_bytes());
        Ok(Some(RestartJpeg {
            start,
            len,
            header: header.into(),
            offsets,
            tile_w,
            tile_h: h.mcu_h,
            across,
            width: image.width,
            height: image.height,
            components: h.components,
            ycbcr: image.photometric == 6,
        }))
    }

    /// Delivers `region` to `sink`, decoding the intervals it touches.
    pub fn read(&self, tiff: &TiffFile, region: Region, sink: &mut Sink) -> Result<()> {
        #[cfg(feature = "jpeg")]
        {
            use crate::codecs::jpeg::{Color, decode};
            let (x0, y0) = (region.x as u32, region.y as u32);
            let (x1, y1) = (x0 + region.width as u32, y0 + region.height as u32);
            let px = self.components;
            let color = if self.ycbcr { Color::YcbcrToRgb } else { Color::AsStored };
            let tile_len = self.tile_w as usize * self.tile_h as usize * px;
            scratch::with(tile_len, |tile| {
                for ty in y0 / self.tile_h..y1.div_ceil(self.tile_h) {
                    for tx in x0 / self.tile_w..x1.div_ceil(self.tile_w) {
                        let k = (ty * self.across + tx) as usize;
                        let from = self.offsets[k];
                        // Entropy data ends before the next RST marker or the EOI.
                        let to = self.offsets.get(k + 1).map_or(self.len, |&n| n - 2);
                        let seg = (to - from) as usize;
                        let total = self.header.len() + seg + 2;
                        scratch::with(total, |stream| {
                            stream[..self.header.len()].copy_from_slice(&self.header);
                            read_exact_at(&tiff.file, &mut stream[self.header.len()..total - 2], self.start + from)?;
                            // A trailing EOI of the last interval is kept; one more does no harm.
                            stream[total - 2..].copy_from_slice(&[0xFF, 0xD9]);
                            let info = decode(stream, None, color, tile)?;
                            if info.components != px {
                                return Err(Error::Corrupt(format!("NDPI JPEG with {} components", info.components)));
                            }
                            Ok(())
                        })?;
                        let (cx0, cy0) = (tx * self.tile_w, ty * self.tile_h);
                        let (ax0, ax1) = (x0.max(cx0), x1.min(cx0 + self.tile_w).min(self.width));
                        let w = (ax1 - ax0) as usize;
                        for y in y0.max(cy0)..y1.min(cy0 + self.tile_h).min(self.height) {
                            let src = &tile[(((y - cy0) * self.tile_w + ax0 - cx0) as usize) * px..][..w * px];
                            sink.write_with((y - y0) as usize, (ax0 - x0) as usize, w, |dst| {
                                dst.copy_from_slice(src);
                                Ok(())
                            })?;
                        }
                    }
                }
                Ok(())
            })
        }
        #[cfg(not(feature = "jpeg"))]
        {
            let _ = (tiff, region, sink);
            Err(Error::UnsupportedVariant("jpeg compression (enable the `jpeg` feature)".into()))
        }
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jpeg_header() {
        // SOI, DRI 4, SOF0 16x8 with 2x1 luma sampling, SOS.
        let mut b = vec![0xFF, 0xD8, 0xFF, 0xDD, 0, 4, 0, 4];
        b.extend_from_slice(&[0xFF, 0xC0, 0, 17, 8, 0, 8, 0, 16, 3, 1, 0x21, 0, 2, 0x11, 1, 3, 0x11, 1]);
        b.extend_from_slice(&[0xFF, 0xDA, 0, 12, 3, 1, 0, 2, 0x11, 3, 0x11, 0, 63, 0]);
        let h = parse_header(&b).unwrap();
        assert_eq!((h.mcu_w, h.mcu_h, h.components, h.restart_interval), (16, 8, 3, 4));
        assert_eq!(h.data_at, b.len());
        assert_eq!(&b[h.sof_size_at..h.sof_size_at + 4], &[0, 8, 0, 16]);
        assert!(parse_header(&b[..10]).is_none());
    }
}

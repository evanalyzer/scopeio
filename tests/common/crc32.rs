//! # crc32
//!
//! CRC-32 (IEEE 802.3, as Python's `zlib.crc32`) for comparing pixel data
//! with reference checksums.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Incremental CRC-32 checksum.
pub struct Crc32 {
    table: [u32; 256],
    crc: u32,
}

impl Crc32 {
    pub fn new() -> Self {
        let mut table = [0u32; 256];
        for (i, entry) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *entry = c;
        }
        Self { table, crc: 0xFFFF_FFFF }
    }

    pub fn update(&mut self, data: &[u8]) {
        for &b in data {
            self.crc = self.table[((self.crc ^ b as u32) & 0xFF) as usize] ^ (self.crc >> 8);
        }
    }

    pub fn finish(&self) -> u32 {
        self.crc ^ 0xFFFF_FFFF
    }
}

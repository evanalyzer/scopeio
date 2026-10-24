//! # cfb
//!
//! Read-only Compound File Binary (OLE2) container: the directory tree and
//! positioned reads of its streams, as used by Olympus OIB and Zeiss ZVI.
//!
//! Provenance: Microsoft [MS-CFB] "Compound File Binary File Format"
//! (published open specification). No GPL code consulted.
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

use crate::io::read_exact_at;
use crate::{Error, Result};

/// Signature of a compound file.
pub(super) const MAGIC: &[u8] = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1";
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const NO_STREAM: u32 = 0xFFFF_FFFF;
/// Largest stream read into memory.
const MAX_STREAM: u64 = 1 << 31;
/// Most directory entries walked, against corrupt files.
const MAX_ENTRIES: usize = 1 << 20;

/// Returns `true` if `head` starts like a compound file.
pub(super) fn matches(head: &[u8]) -> bool {
    head.starts_with(MAGIC)
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

/// A stream of the compound file.
#[derive(Debug, Clone)]
pub(super) struct Stream {
    /// Path of storages and the stream name, joined by `/`.
    pub path: String,
    pub size: u64,
    start: u32,
}

/// An open compound file.
#[derive(Debug)]
pub(super) struct Cfb {
    file: File,
    len: u64,
    sector: u64,
    mini_sector: u64,
    mini_cutoff: u64,
    fat: Vec<u32>,
    mini_fat: Vec<u32>,
    /// Sectors of the mini stream (root entry), in order.
    mini_stream: Vec<u32>,
    streams: Vec<Stream>,
}

impl Cfb {
    /// Opens a compound file and reads its allocation tables and directory.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid headers, chains or directories.
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut h = [0u8; 512];
        if len < 512 {
            return Err(Error::Corrupt("compound file too short".into()));
        }
        read_exact_at(&file, &mut h, 0)?;
        if !matches(&h) || u16_at(&h, 0x1C) != 0xFFFE {
            return Err(Error::Corrupt("not a compound file".into()));
        }
        let shift = u16_at(&h, 0x1E);
        let mini_shift = u16_at(&h, 0x20);
        if !(9..=16).contains(&shift) || mini_shift >= shift {
            return Err(Error::Corrupt(format!("compound file sector shift {shift}")));
        }
        let sector = 1u64 << shift;
        let per_sector = (sector / 4) as usize;
        let mut cfb = Cfb {
            file,
            len,
            sector,
            mini_sector: 1 << mini_shift,
            mini_cutoff: u32_at(&h, 0x38) as u64,
            fat: Vec::new(),
            mini_fat: Vec::new(),
            mini_stream: Vec::new(),
            streams: Vec::new(),
        };
        let max_sectors = (len / sector + 1) as usize;
        // FAT sectors: 109 in the header, then the DIFAT chain.
        let fat_count = u32_at(&h, 0x2C) as usize;
        if fat_count > max_sectors {
            return Err(Error::Corrupt("compound file FAT larger than the file".into()));
        }
        let mut fat_sectors: Vec<u32> = (0..109).map(|i| u32_at(&h, 0x4C + 4 * i)).take(fat_count).collect();
        let mut difat = u32_at(&h, 0x44);
        let mut seen = 0;
        while fat_sectors.len() < fat_count && difat != END_OF_CHAIN && difat != FREE {
            seen += 1;
            if seen > max_sectors {
                return Err(Error::Corrupt("compound file DIFAT loops".into()));
            }
            let b = cfb.read_sector(difat)?;
            fat_sectors.extend((0..per_sector - 1).map(|i| u32_at(&b, 4 * i)).take(fat_count - fat_sectors.len()));
            difat = u32_at(&b, 4 * (per_sector - 1));
        }
        let mut fat = Vec::with_capacity(fat_sectors.len() * per_sector);
        for s in fat_sectors {
            let b = cfb.read_sector(s)?;
            fat.extend((0..per_sector).map(|i| u32_at(&b, 4 * i)));
        }
        cfb.fat = fat;
        // Directory.
        let dir = cfb.read_chain(u32_at(&h, 0x30), u64::MAX)?;
        let entry = |i: usize| dir.get(i * 128..i * 128 + 128);
        let root = entry(0).ok_or_else(|| Error::Corrupt("compound file without root entry".into()))?;
        if root[66] != 5 {
            return Err(Error::Corrupt("compound file root entry missing".into()));
        }
        let root_start = u32_at(root, 116);
        let root_size = u32_at(root, 120) as u64;
        // Mini FAT and mini stream.
        let mini_fat = cfb.read_chain(u32_at(&h, 0x3C), u64::MAX)?;
        cfb.mini_fat = mini_fat.chunks_exact(4).map(|c| u32_at(c, 0)).collect();
        cfb.mini_stream = cfb.chain(root_start, root_size.div_ceil(sector))?;
        // Walk the red-black trees of the storages depth first.
        let mut stack = vec![(u32_at(root, 76), String::new())];
        let mut visited = vec![false; dir.len() / 128];
        while let Some((id, prefix)) = stack.pop() {
            if id == NO_STREAM {
                continue;
            }
            let i = id as usize;
            let Some(e) = entry(i) else { return Err(Error::Corrupt("compound file directory entry out of range".into())) };
            if std::mem::replace(&mut visited[i], true) || cfb.streams.len() > MAX_ENTRIES {
                return Err(Error::Corrupt("compound file directory loops".into()));
            }
            let name_len = (u16_at(e, 64) as usize).min(64);
            let units: Vec<u16> = e[..name_len.saturating_sub(2)].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let name = String::from_utf16_lossy(&units);
            stack.push((u32_at(e, 68), prefix.clone()));
            stack.push((u32_at(e, 72), prefix.clone()));
            let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            match e[66] {
                1 => stack.push((u32_at(e, 76), path)),
                2 => {
                    let size = u32_at(e, 120) as u64 | if shift > 9 { (u32_at(e, 124) as u64) << 32 } else { 0 };
                    cfb.streams.push(Stream { path, size, start: u32_at(e, 116) });
                }
                _ => {}
            }
        }
        Ok(cfb)
    }

    /// All streams.
    pub fn streams(&self) -> &[Stream] {
        &self.streams
    }

    /// The stream at `path` (storages separated by `/`), ignoring case.
    pub fn stream(&self, path: &str) -> Option<&Stream> {
        self.streams.iter().find(|s| s.path.eq_ignore_ascii_case(path))
    }

    /// Reads a whole stream.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for broken chains or streams too large.
    pub fn read(&self, s: &Stream) -> Result<Vec<u8>> {
        if s.size > MAX_STREAM || s.size > self.len.saturating_mul(64) {
            return Err(Error::Corrupt(format!("compound file stream {} too large", s.path)));
        }
        if s.size < self.mini_cutoff {
            let unit = self.mini_sector;
            let mut out = Vec::with_capacity(s.size as usize);
            let mut id = s.start;
            let mut buf = vec![0u8; unit as usize];
            while (out.len() as u64) < s.size {
                if id == END_OF_CHAIN || out.len() / unit as usize > self.mini_fat.len() {
                    return Err(Error::Corrupt(format!("compound file mini stream {} ends early", s.path)));
                }
                let at = id as u64 * unit;
                let sector = self.mini_stream.get((at / self.sector) as usize).ok_or_else(|| Error::Corrupt("mini stream sector out of range".into()))?;
                read_exact_at(&self.file, &mut buf, (*sector as u64 + 1) * self.sector + at % self.sector)?;
                let n = (s.size - out.len() as u64).min(unit) as usize;
                out.extend_from_slice(&buf[..n]);
                id = *self.mini_fat.get(id as usize).ok_or_else(|| Error::Corrupt("mini FAT entry out of range".into()))?;
            }
            return Ok(out);
        }
        let mut out = self.read_chain(s.start, s.size.div_ceil(self.sector))?;
        if (out.len() as u64) < s.size {
            return Err(Error::Corrupt(format!("compound file stream {} ends early", s.path)));
        }
        out.truncate(s.size as usize);
        Ok(out)
    }

    /// The sectors of a stream stored outside the mini stream, for
    /// [`read_at`](Self::read_at); `None` for streams in the mini stream.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for broken chains.
    pub fn sectors(&self, s: &Stream) -> Result<Option<Vec<u32>>> {
        if s.size < self.mini_cutoff {
            return Ok(None);
        }
        let ids = self.chain(s.start, s.size.div_ceil(self.sector))?;
        if (ids.len() as u64) * self.sector < s.size {
            return Err(Error::Corrupt(format!("compound file stream {} ends early", s.path)));
        }
        Ok(Some(ids))
    }

    /// Reads `buf.len()` bytes at offset `at` of a stream whose sectors are
    /// `sectors` (see [`sectors`](Self::sectors)), reading runs of
    /// consecutive sectors at once.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] if the range is outside the stream.
    pub fn read_at(&self, sectors: &[u32], at: u64, buf: &mut [u8]) -> Result<()> {
        let end = at.checked_add(buf.len() as u64).filter(|&e| e <= sectors.len() as u64 * self.sector);
        if end.is_none() {
            return Err(Error::Corrupt("compound file read beyond its stream".into()));
        }
        let mut done = 0usize;
        while done < buf.len() {
            let pos = at + done as u64;
            let i = (pos / self.sector) as usize;
            let run = sectors[i..].windows(2).take_while(|w| w[1] == w[0] + 1).count() + 1;
            let within = pos % self.sector;
            let n = ((run as u64 * self.sector - within) as usize).min(buf.len() - done);
            read_exact_at(&self.file, &mut buf[done..done + n], (sectors[i] as u64 + 1) * self.sector + within)?;
            done += n;
        }
        Ok(())
    }

    /// Sector ids of the chain starting at `start`, at most `max` sectors.
    fn chain(&self, mut id: u32, max: u64) -> Result<Vec<u32>> {
        let mut out = Vec::new();
        while id != END_OF_CHAIN && (out.len() as u64) < max {
            if out.len() > self.fat.len() {
                return Err(Error::Corrupt("compound file chain loops".into()));
            }
            out.push(id);
            id = *self.fat.get(id as usize).ok_or_else(|| Error::Corrupt("compound file FAT entry out of range".into()))?;
        }
        Ok(out)
    }

    /// Bytes of the chain starting at `start` (at most `max` sectors),
    /// reading runs of consecutive sectors at once.
    fn read_chain(&self, start: u32, max: u64) -> Result<Vec<u8>> {
        let ids = self.chain(start, max)?;
        let total = (ids.len() as u64).saturating_mul(self.sector);
        if total > MAX_STREAM || total > self.len {
            return Err(Error::Corrupt("compound file chain longer than the file".into()));
        }
        let mut out = vec![0u8; total as usize];
        let mut i = 0;
        while i < ids.len() {
            let run = ids[i..].windows(2).take_while(|w| w[1] == w[0] + 1).count() + 1;
            let bytes = run * self.sector as usize;
            let at = i * self.sector as usize;
            read_exact_at(&self.file, &mut out[at..at + bytes], (ids[i] as u64 + 1) * self.sector)?;
            i += run;
        }
        Ok(out)
    }

    fn read_sector(&self, id: u32) -> Result<Vec<u8>> {
        let mut b = vec![0u8; self.sector as usize];
        read_exact_at(&self.file, &mut b, (id as u64 + 1) * self.sector)?;
        Ok(b)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "inspection helper: CFB_DUMP=<file> cargo test cfb_dump -- --ignored --nocapture"]
    fn cfb_dump() {
        let Ok(path) = std::env::var("CFB_DUMP") else { return };
        let c = Cfb::open(Path::new(&path)).unwrap();
        for s in c.streams() {
            println!("{:>10} {}", s.size, s.path);
        }
        if let Ok(name) = std::env::var("CFB_STREAM") {
            let s = c.stream(&name).unwrap();
            let b = c.read(s).unwrap();
            let text = if b.starts_with(&[0xFF, 0xFE]) {
                String::from_utf16_lossy(&b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
            } else {
                String::from_utf8_lossy(&b).into_owned()
            };
            println!("{text}");
        }
    }
}

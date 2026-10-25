//! # omp2info
//!
//! Olympus/Evident FLUOVIEW multi-area acquisitions (`.omp2info`): an XML
//! file (`matl:properties`) listing, per group (`matl:group`), the tile
//! grid (`matl:areaInfo`: tile counts, left/top and tile size in nm), the
//! stage overlap in percent (`matl:stage/matl:overlap`) and one
//! `matl:area` per tile with its OIR file and grid indices.
//!
//! The tile positions Olympus uses for stitching are computed by image
//! matching (`matl:stitchPOCParam`) and stored nowhere; the stitched result
//! is saved as `Stitch_<area>_<group>.oir` next to the tiles. When that file
//! is present it is series 0 (the exact vendor mosaic); every tile follows
//! as its own series, in file order, with its grid indices in the name and
//! its nominal stage position (grid origin plus index times tile size less
//! the overlap). Planes are read by the OIR reader.
//!
//! Provenance: own analysis of the public multi-area acquisition
//! `Olympus-OIR/gh-4205/zenodo-13680725` (OME sample server, CC BY 4.0):
//! XML elements, file naming of the vendor stitch; Bio-Formats run as a
//! black box (it stitches on its own grid, which differs from the vendor
//! stitch). No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use super::oir::Oir;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{Error, FormatId, FormatInfo, OriginalMeta, Plane, PlaneMeta, Position, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Omp2info, "Olympus OMP2INFO", "1").extensions(&["omp2info"]);

/// Largest index file parsed.
const MAX_INDEX: u64 = 64 << 20;

/// Returns `true` for an `.omp2info` file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("omp2info"));
    ext && String::from_utf8_lossy(head).contains("matl:properties")
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn num(n: Node, name: &str) -> Option<f64> {
    child(n, name)?.text()?.trim().parse().ok()
}

/// One tile of the acquisition.
#[derive(Debug)]
struct Tile {
    file: PathBuf,
    x: u32,
    y: u32,
    position: Option<Position>,
}

/// The vendor stitch of a tile file `<name>_<area>_<group>_<n>.oir`:
/// `Stitch_<area>_<group>.oir` in the same folder.
fn stitch_for(tile: &Path) -> Option<PathBuf> {
    let stem = tile.file_stem()?.to_str()?;
    let parts: Vec<&str> = stem.rsplitn(4, '_').collect();
    // [number, group, area, rest]
    if parts.len() < 3 || !parts[0].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let stitch = tile.with_file_name(format!("Stitch_{}_{}.oir", parts[2], parts[1]));
    stitch.is_file().then_some(stitch)
}

/// Olympus multi-area acquisition.
#[derive(Debug)]
pub(super) struct Omp2info {
    /// Readers and series index within each, one per series.
    parts: Vec<(usize, usize)>,
    readers: Vec<Oir>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Omp2info {
    /// Opens an `.omp2info` file and the OIR files it lists.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML or an acquisition without
    /// tiles, or the error of a tile file that cannot be opened.
    pub fn open(path: &Path) -> Result<Self> {
        if std::fs::metadata(path)?.len() > MAX_INDEX {
            return Err(Error::Corrupt("OMP2INFO file too large".into()));
        }
        let raw = std::fs::read(path)?;
        let xml = String::from_utf8_lossy(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw)).into_owned();
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("OMP2INFO: {e}")))?;
        let root = doc.root_element();
        let dir = path.parent().unwrap_or(Path::new("."));
        let overlap = child(root, "stage").and_then(|s| num(s, "overlap")).filter(|v| (0.0..100.0).contains(v)).unwrap_or(0.0);
        let mut tiles = Vec::new();
        for group in root.children().filter(|c| c.tag_name().name() == "group") {
            let info = child(group, "areaInfo");
            let grid = info.and_then(|i| Some((num(i, "areaLeft")?, num(i, "areaTop")?, num(i, "areaWidth")?, num(i, "areaHeight")?)));
            for area in group.children().filter(|c| c.tag_name().name() == "area") {
                let Some(image) = child(area, "image").and_then(|i| i.text()).map(str::trim).filter(|t| !t.is_empty()) else { continue };
                let (x, y) = (num(area, "xIndex").unwrap_or(0.0) as u32, num(area, "yIndex").unwrap_or(0.0) as u32);
                // Nominal position: grid origin plus index times the step.
                let position = grid.map(|(left, top, w, h)| {
                    let step = 1.0 - overlap / 100.0;
                    Position { x_nm: Some(left + x as f64 * w * step), y_nm: Some(top + y as f64 * h * step), z_nm: None }
                });
                tiles.push(Tile { file: dir.join(image), x, y, position });
            }
        }
        if tiles.is_empty() {
            return Err(Error::Corrupt("OMP2INFO without tiles".into()));
        }
        let mut readers = Vec::new();
        let mut parts = Vec::new();
        let mut metas = Vec::new();
        let mut original = OriginalMeta::default();
        original.pairs.push(("Stage overlap (%)".into(), overlap.to_string()));
        if let Some(stitch) = stitch_for(&tiles[0].file) {
            let oir = Oir::open(&stitch)?;
            for (s, m) in oir.series().iter().enumerate() {
                let mut m = m.clone();
                m.name = Some(format!("stitched ({})", stitch.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
                metas.push(m);
                parts.push((readers.len(), s));
            }
            original.pairs.push(("Stitched image".into(), stitch.display().to_string()));
            readers.push(oir);
        }
        for tile in &tiles {
            let oir = Oir::open(&tile.file)?;
            let name = tile.file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            for (s, m) in oir.series().iter().enumerate() {
                let mut m = m.clone();
                m.name = Some(format!("tile x{} y{} ({name})", tile.x, tile.y));
                if let Some(p) = tile.position {
                    m.stage_position = Some(Position { z_nm: m.stage_position.and_then(|q| q.z_nm), ..p });
                }
                metas.push(m);
                parts.push((readers.len(), s));
            }
            readers.push(oir);
        }
        Ok(Omp2info { parts, readers, metas, original })
    }
}

impl Backend for Omp2info {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, series: usize, plane: Plane) -> Result<PlaneMeta> {
        let (r, s) = self.parts[series];
        self.readers[r].plane_meta(s, plane)
    }

    fn original_metadata(&self, series: usize) -> Result<OriginalMeta> {
        let (r, s) = self.parts[series];
        let mut meta = self.readers[r].original_metadata(s)?;
        meta.pairs.extend(self.original.pairs.iter().cloned());
        Ok(meta)
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let (r, s) = self.parts[series];
        self.readers[r].read_native(s, level, plane, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stitch_name() {
        let dir = std::env::temp_dir().join(format!("scopeio-omp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Stitch_A01_G001.oir"), b"").unwrap();
        assert_eq!(stitch_for(&dir.join("brain_section_A01_G001_0001.oir")), Some(dir.join("Stitch_A01_G001.oir")));
        assert_eq!(stitch_for(&dir.join("brain_section_A02_G001_0001.oir")), None);
        assert_eq!(stitch_for(&dir.join("noindex.oir")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

//! # vms
//!
//! Hamamatsu VMS reader (`.vms` and its JPEG files).
//!
//! The `.vms` file is INI text; its `[Virtual Microscope Specimen]` section
//! names a grid of `NoJpegColumns` x `NoJpegRows` JPEG files
//! (`ImageFile`, `ImageFile(x,y)`, `ImageFile(z,x,y)` for further focal
//! planes), a lower-resolution `MapFile` of the whole slide, a
//! `MacroImage` and the `OptimisationFile`. The JPEG files tile the image:
//! the widths of the first row's files give the columns, the heights of the
//! first column's files the rows. They carry restart markers; each is
//! indexed by scanning its entropy data on first use (the optimisation file
//! is incomplete, see the format notes) and read one restart interval at a
//! time (see [`RestartJpeg`]). The map is level 1 if its size is the image
//! size divided by a whole factor. Pixel size from `PhysicalWidth` /
//! `PhysicalHeight` (nm), objective from `SourceLens`; focal planes are z.
//!
//! Not supported: VMU (uncompressed NGR files).
//!
//! Provenance: openslide.org Hamamatsu format page (prose, CC BY-SA 4.0,
//! `doc/hamamatsu-ndpi-vms/`); ITU-T T.81 (JPEG); own analysis of the
//! OpenSlide VMS test slides (CC0); checked against Bio-Formats as a black
//! box. OpenSlide's C code (LGPL) was not consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::slide::restart::RestartJpeg;
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize,
    PixelType, Plane, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Vms, "Hamamatsu VMS", "1").extensions(&["vms"]);

const SECTION: &str = "[Virtual Microscope Specimen]";
/// Largest `.vms` file parsed.
const MAX_INDEX: u64 = 1 << 20;
/// Most JPEG files of one slide, against corrupt counts.
const MAX_FILES: usize = 4096;

/// Returns `true` for a VMS index file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("vms")) && String::from_utf8_lossy(head).contains(SECTION)
}

/// Key-value pairs of the VMS section, in file order.
fn parse(text: &str) -> Vec<(String, String)> {
    let mut in_section = false;
    let mut out = Vec::new();
    for line in text.split(['\r', '\n']).map(str::trim) {
        if line.starts_with('[') {
            in_section = line.eq_ignore_ascii_case(SECTION);
        } else if in_section && let Some((k, v)) = line.split_once('=') {
            out.push((k.trim().to_string(), v.trim().trim_end_matches(';').trim().to_string()));
        }
    }
    out
}

/// Grid position `(z, x, y)` of an `ImageFile` key.
fn image_key(key: &str) -> Option<(u32, u32, u32)> {
    let rest = key.strip_prefix("ImageFile")?;
    if rest.is_empty() {
        return Some((0, 0, 0));
    }
    let n: Vec<u32> = rest.strip_prefix('(')?.strip_suffix(')')?.split(',').map(|v| v.trim().parse().ok()).collect::<Option<_>>()?;
    match n[..] {
        [x, y] => Some((0, x, y)),
        [z, x, y] => Some((z, x, y)),
        _ => None,
    }
}

/// One JPEG file, indexed on first use.
#[derive(Debug)]
struct Jpeg {
    path: PathBuf,
    file: File,
    len: u64,
    index: OnceLock<std::result::Result<RestartJpeg, String>>,
}

impl Jpeg {
    fn open(path: PathBuf) -> Result<Self> {
        let file = File::open(&path)?;
        let len = file.metadata()?.len();
        Ok(Jpeg { path, file, len, index: OnceLock::new() })
    }

    /// The restart interval index, scanning the file on first use.
    fn index(&self) -> Result<&RestartJpeg> {
        let index = self.index.get_or_init(|| RestartJpeg::scan(&self.file, self.len).map_err(|e| e.to_string()));
        index.as_ref().map_err(|e| Error::Corrupt(format!("{}: {e}", self.path.display())))
    }
}

/// Size and restart tile of a JPEG file, from its header.
fn jpeg_info(j: &Jpeg) -> Result<((u32, u32), Option<(u32, u32)>)> {
    RestartJpeg::header_info(&j.file, j.len)
}

/// Hamamatsu VMS slide.
#[derive(Debug)]
pub(super) struct Vms {
    /// Image files by `(z * rows + row) * columns + column`.
    tiles: Vec<Option<Jpeg>>,
    columns: usize,
    rows: usize,
    /// Left edge of each column and top edge of each row; one extra entry
    /// holds the image size.
    col_x: Vec<u64>,
    row_y: Vec<u64>,
    map: Option<Jpeg>,
    macro_image: Option<Jpeg>,
    associated_kinds: Vec<AssociatedKind>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Vms {
    /// Opens a VMS index file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for a missing or inconsistent image grid
    /// and [`Error::UnsupportedVariant`] for unreadable JPEG files.
    pub fn open(path: &Path) -> Result<Self> {
        if std::fs::metadata(path)?.len() > MAX_INDEX {
            return Err(Error::Corrupt("VMS file too large".into()));
        }
        let text = String::from_utf8_lossy(&std::fs::read(path)?).into_owned();
        let pairs = parse(&text);
        let kv: HashMap<&str, &str> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let count = |k: &str| kv.get(k).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
        let (columns, rows) = (count("NoJpegColumns"), count("NoJpegRows"));
        let dir = path.parent().unwrap_or(Path::new("."));
        // Only file names: never leave the slide's folder.
        let file = |name: &str| Path::new(&name.replace('\\', "/")).file_name().map(|n| dir.join(n));
        let mut grid: HashMap<(u32, u32, u32), PathBuf> = HashMap::new();
        for (k, v) in &pairs {
            if let (Some(pos), Some(f)) = (image_key(k), file(v)) {
                grid.insert(pos, f);
            }
        }
        let layers = grid.keys().map(|p| p.0 as usize + 1).max().unwrap_or(0);
        if layers == 0 || columns.saturating_mul(rows).saturating_mul(layers) > MAX_FILES {
            return Err(Error::Corrupt("VMS without a valid image grid".into()));
        }
        let mut tiles = Vec::with_capacity(layers * rows * columns);
        for z in 0..layers {
            for y in 0..rows {
                for x in 0..columns {
                    tiles.push(match grid.get(&(z as u32, x as u32, y as u32)) {
                        Some(p) if p.is_file() => Some(Jpeg::open(p.clone())?),
                        _ => None,
                    });
                }
            }
        }
        // Column widths from the first row, row heights from the first column.
        let size = |x: usize, y: usize| -> Result<(u32, u32)> {
            let j = tiles[y * columns + x].as_ref().ok_or_else(|| Error::Corrupt(format!("VMS image file ({x},{y}) missing")))?;
            Ok(jpeg_info(j)?.0)
        };
        let mut col_x = vec![0u64];
        for x in 0..columns {
            col_x.push(col_x[x] + size(x, 0)?.0 as u64);
        }
        let mut row_y = vec![0u64];
        for y in 0..rows {
            row_y.push(row_y[y] + size(0, y)?.1 as u64);
        }
        for y in 0..rows {
            for x in 0..columns {
                let (w, h) = size(x, y)?;
                if (w as u64, h as u64) != (col_x[x + 1] - col_x[x], row_y[y + 1] - row_y[y]) {
                    return Err(Error::Corrupt(format!("VMS image file ({x},{y}) does not fit the grid")));
                }
            }
        }
        let (width, height) = (col_x[columns], row_y[rows]);
        let mut levels = vec![LevelMeta { size_x: width, size_y: height, tile: None, downsample: 1.0 }];
        let open_opt = |k: &str| kv.get(k).and_then(|v| file(v)).filter(|p| p.is_file()).map(Jpeg::open).transpose();
        // The map is a level if it scales the image by a whole factor.
        let mut map = open_opt("MapFile")?;
        if let Some(m) = &map {
            let ((mw, mh), tile) = jpeg_info(m)?;
            let f = width / (mw as u64).max(1);
            if f > 1 && mw as u64 * f == width && mh as u64 * f == height {
                levels.push(LevelMeta { size_x: mw as u64, size_y: mh as u64, tile, downsample: f as f64 });
            } else {
                map = None;
            }
        }
        // Tile size from the first file's restart interval.
        if let Some(Some(first)) = tiles.first() {
            levels[0].tile = jpeg_info(first)?.1;
        }
        let macro_image = open_opt("MacroImage")?;
        let associated_kinds = if macro_image.is_some() { vec![AssociatedKind::Macro] } else { Vec::new() };
        let nm = |k: &str, px: u64| kv.get(k).and_then(|v| v.parse::<f64>().ok()).map(|v| v / px as f64).filter(|v| v.is_finite() && *v > 0.0);
        let meta = SeriesMeta {
            name: path.file_stem().map(|s| s.to_string_lossy().into_owned()),
            size_c: 1,
            size_z: layers as u32,
            size_t: 1,
            pixel_type: PixelType::U8,
            samples_per_pixel: 3,
            levels,
            physical_size: PhysicalSize { x_nm: nm("PhysicalWidth", width), y_nm: nm("PhysicalHeight", height), z_nm: None },
            channels: vec![ChannelMeta::default()],
            objective: kv.get("SourceLens").and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0).map(|m| ObjectiveMeta { magnification: Some(m), ..Default::default() }),
            ..Default::default()
        };
        let original = OriginalMeta { pairs: pairs.iter().map(|(k, v)| (format!("hamamatsu.{k}"), v.clone())).collect(), documents: Vec::new() };
        Ok(Vms { tiles, columns, rows, col_x, row_y, map, macro_image, associated_kinds, metas: vec![meta], original })
    }
}

/// Delivers `region` of an image made of one JPEG to `sink`.
fn read_jpeg(j: &Jpeg, region: Region, origin: (usize, usize), sink: &mut Sink) -> Result<()> {
    j.index()?.read(&j.file, region, origin, sink)
}

impl Backend for Vms {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let (Some(j), AssociatedKind::Macro) = (&self.macro_image, kind) else {
            return Err(Error::OutOfRange(format!("no associated image {kind:?}")));
        };
        let (w, h) = j.index()?.size();
        native_image(Region::new(0, 0, w as u64, h as u64), 3, PixelType::U8, |sink| read_jpeg(j, Region::new(0, 0, w as u64, h as u64), (0, 0), sink))
    }

    fn read_native(&self, _series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        if level == 1 {
            let map = self.map.as_ref().ok_or_else(|| Error::OutOfRange("VMS map level".into()))?;
            return read_jpeg(map, region, (0, 0), sink);
        }
        let (x0, y0) = (region.x, region.y);
        let (x1, y1) = (x0 + region.width, y0 + region.height);
        for row in 0..self.rows {
            let (ty0, ty1) = (self.row_y[row], self.row_y[row + 1]);
            if ty1 <= y0 || ty0 >= y1 {
                continue;
            }
            for col in 0..self.columns {
                let (tx0, tx1) = (self.col_x[col], self.col_x[col + 1]);
                if tx1 <= x0 || tx0 >= x1 {
                    continue;
                }
                let (ax0, ax1, ay0, ay1) = (x0.max(tx0), x1.min(tx1), y0.max(ty0), y1.min(ty1));
                let sub = Region::new(ax0 - tx0, ay0 - ty0, ax1 - ax0, ay1 - ay0);
                let origin = ((ay0 - y0) as usize, (ax0 - x0) as usize);
                match &self.tiles[(plane.z as usize * self.rows + row) * self.columns + col] {
                    Some(j) => read_jpeg(j, sub, origin, sink)?,
                    // A missing file of a focal plane reads as zeros.
                    None => {
                        for y in 0..sub.height as usize {
                            sink.write_with(origin.0 + y, origin.1, sub.width as usize, |d| {
                                d.fill(0);
                                Ok(())
                            })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_index_and_keys() {
        let p = parse("[Other]\r\nNoLayers=9\r\n[Virtual Microscope Specimen]\r\nNoJpegColumns=2\r\nPhysicalMacroHeight=26000000;\r\n");
        assert_eq!(p, [("NoJpegColumns".into(), "2".into()), ("PhysicalMacroHeight".into(), "26000000".into())]);
        assert_eq!(image_key("ImageFile"), Some((0, 0, 0)));
        assert_eq!(image_key("ImageFile(1,0)"), Some((0, 1, 0)));
        assert_eq!(image_key("ImageFile(2,1,0)"), Some((2, 1, 0)));
        assert_eq!(image_key("ImageFile(x)"), None);
        assert_eq!(image_key("MapFile"), None);
    }
}

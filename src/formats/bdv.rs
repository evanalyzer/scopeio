//! # bdv
//!
//! BigDataViewer XML/HDF5 reader (`.xml` + `.h5`).
//!
//! The XML (`SpimData`) lists view setups (id, name, size, voxel size and
//! attributes such as channel, angle, illumination and tile), the time
//! points (`range`, `list` or `pattern`), missing views and the HDF5 file
//! (`ImageLoader format="bdv.hdf5"`). The HDF5 file holds one multi-
//! resolution volume per view: `t<tp:05>/s<setup:02>/<level>/cells`, a
//! (z, y, x) chunked dataset; `s<setup:02>/resolutions` holds the
//! downsampling factors (x, y, z) per level. 16-bit data are stored as
//! HDF5 int16 but are unsigned (BigDataViewer convention) and read as
//! `U16`.
//!
//! Setups that differ only in their `channel` attribute (same size and
//! levels) form one series with the channels in attribute order; every
//! other attribute combination is a series. Time points are T. Levels with
//! fewer z planes serve every z plane of level 0 from the nearest stored
//! plane. Missing views and absent datasets read as zeros. Registrations
//! (affine transforms) are not applied; partitioned HDF5 and the N5 / Zarr
//! loaders are not supported. Opening the `.h5` file opens the XML of the
//! same name next to it.
//!
//! Provenance: BigDataViewer documentation (ImageJ wiki) and Pietzsch et
//! al., Nat. Methods 2015 (arXiv 1412.0488), in `doc/bdv/`; HDF5 File Format
//! Specification (see `formats/hdf5.rs`); own analysis of the public BDV
//! samples on the OME sample server (public domain); checked against
//! Bio-Formats and bioformats-rs as black boxes. No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-29
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use roxmltree::{Document, Node};

use super::hdf5::{H5, Kind, Layout, Volume};
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, OriginalMeta, PhysicalSize, PixelType, Plane, Region, Result, SeriesMeta};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Bdv, "BigDataViewer XML/HDF5", "1").extensions(&["xml", "h5"]);

/// Largest XML file parsed.
const MAX_XML: u64 = 64 << 20;
/// Most views (setups x time points), against corrupt time point lists.
const MAX_VIEWS: usize = 1 << 24;

/// Returns `true` for a BigDataViewer XML file, or an HDF5 file with one
/// next to it.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    if H5::matches(head) {
        return companion_xml(path).is_some();
    }
    is_spimdata(head)
}

fn is_spimdata(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    text.trim_start().starts_with("<?xml") && text.contains("<SpimData")
}

/// `<stem>.xml` next to an `.h5` file, if it is BigDataViewer XML.
fn companion_xml(h5: &Path) -> Option<PathBuf> {
    let xml = h5.with_extension("xml");
    let mut head = Vec::with_capacity(512);
    std::io::Read::read_to_end(&mut std::io::Read::take(File::open(&xml).ok()?, 512), &mut head).ok()?;
    is_spimdata(&head).then_some(xml)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.tag_name().name() == name)
}

fn text(n: Node, name: &str) -> Option<String> {
    child(n, name).and_then(|c| c.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn numbers(s: &str) -> Vec<f64> {
    s.split_whitespace().filter_map(|v| v.parse().ok()).collect()
}

/// Time point ids of `Timepoints`: `range` (first, last), `list` or
/// `pattern` (comma-separated ids and ranges `a-b` or `a-b:step`).
fn timepoints(n: Node) -> Result<Vec<i64>> {
    let too_many = || Error::Corrupt("BigDataViewer time points".into());
    match n.attribute("type").unwrap_or("range") {
        "range" => {
            let first: i64 = text(n, "first").and_then(|v| v.parse().ok()).unwrap_or(0);
            let last: i64 = text(n, "last").and_then(|v| v.parse().ok()).unwrap_or(first);
            if last < first || (last - first) as u64 >= MAX_VIEWS as u64 {
                return Err(too_many());
            }
            Ok((first..=last).collect())
        }
        _ => {
            let pattern = text(n, "integerpattern").or_else(|| n.text().map(str::to_string)).unwrap_or_default();
            let mut out = Vec::new();
            for part in pattern.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let (range, step) = part.split_once(':').map_or((part, 1), |(r, s)| (r, s.trim().parse().unwrap_or(1)));
                let bounds: Vec<i64> = range.splitn(2, '-').filter_map(|v| v.trim().parse().ok()).collect();
                match bounds[..] {
                    [a] => out.push(a),
                    [a, b] if b >= a && step > 0 => {
                        if (b - a) as u64 / step as u64 + out.len() as u64 >= MAX_VIEWS as u64 {
                            return Err(too_many());
                        }
                        out.extend((a..=b).step_by(step as usize));
                    }
                    _ => {}
                }
            }
            Ok(out)
        }
    }
}

/// Nanometres per unit of a voxel size unit; `None` for `pixel` or unknown.
fn unit_nm(unit: &str) -> Option<f64> {
    Some(match unit.trim() {
        "um" | "µm" | "micron" | "microns" | "micrometer" => 1e3,
        "nm" | "nanometer" => 1.0,
        "mm" | "millimeter" => 1e6,
        "m" => 1e9,
        _ => return None,
    })
}

/// One view setup of the XML.
#[derive(Debug, Clone)]
struct Setup {
    id: i64,
    name: Option<String>,
    voxel_nm: [Option<f64>; 3],
    /// Attribute name -> id, e.g. `channel` -> 1.
    attributes: BTreeMap<String, i64>,
}

#[derive(Debug)]
struct Group {
    /// Setup ids, by channel.
    setups: Vec<i64>,
}

/// BigDataViewer dataset.
#[derive(Debug)]
pub(super) struct Bdv {
    h5: H5,
    /// Object header of each `t<tp>` group, by time point id.
    tp_groups: HashMap<i64, u64>,
    timepoints: Vec<i64>,
    groups: Vec<Group>,
    missing: HashSet<(i64, i64)>,
    /// Volumes by (setup, time point index, level).
    volumes: HashMap<(i64, usize, usize), OnceLock<std::result::Result<Option<Volume>, String>>>,
    /// Size (z, y, x) of each level, by setup.
    level_sizes: HashMap<i64, Vec<[u64; 3]>>,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Bdv {
    /// Opens a BigDataViewer XML file, or the `.h5` file of one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for invalid XML, [`Error::UnsupportedVariant`]
    /// for other image loaders than `bdv.hdf5` or partitioned HDF5.
    pub fn open(path: &Path) -> Result<Self> {
        let mut head = [0u8; 8];
        let n = std::io::Read::read(&mut File::open(path)?, &mut head)?;
        let xml_path = if H5::matches(&head[..n]) {
            companion_xml(path).ok_or_else(|| Error::UnsupportedFormat(path.display().to_string()))?
        } else {
            path.to_path_buf()
        };
        if std::fs::metadata(&xml_path)?.len() > MAX_XML {
            return Err(Error::Corrupt("BigDataViewer XML too large".into()));
        }
        let raw = std::fs::read(&xml_path)?;
        let xml = String::from_utf8_lossy(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw)).into_owned();
        let doc = Document::parse(&xml).map_err(|e| Error::Corrupt(format!("BigDataViewer XML: {e}")))?;
        let root = doc.root_element();
        let dir = xml_path.parent().unwrap_or(Path::new("."));
        let base = match child(root, "BasePath") {
            Some(b) if b.attribute("type") == Some("absolute") => PathBuf::from(b.text().unwrap_or(".").trim()),
            Some(b) => dir.join(b.text().unwrap_or(".").trim()),
            None => dir.to_path_buf(),
        };
        let seq = child(root, "SequenceDescription").ok_or_else(|| Error::Corrupt("BigDataViewer XML without SequenceDescription".into()))?;
        let loader = child(seq, "ImageLoader").ok_or_else(|| Error::Corrupt("BigDataViewer XML without ImageLoader".into()))?;
        let format = loader.attribute("format").unwrap_or_default();
        if format != "bdv.hdf5" {
            return Err(Error::UnsupportedVariant(format!("BigDataViewer image loader {format}")));
        }
        if child(loader, "partition").is_some() {
            return Err(Error::UnsupportedVariant("BigDataViewer partitioned HDF5".into()));
        }
        let h5_node = child(loader, "hdf5").ok_or_else(|| Error::Corrupt("BigDataViewer XML without hdf5 path".into()))?;
        let h5_name = h5_node.text().unwrap_or_default().trim().to_string();
        let h5_path = if h5_node.attribute("type") == Some("absolute") { PathBuf::from(&h5_name) } else { base.join(&h5_name) };
        // A moved dataset: the HDF5 file next to the XML.
        let h5_path = if h5_path.is_file() { h5_path } else { dir.join(Path::new(&h5_name).file_name().unwrap_or_default()) };
        let h5 = H5::open(File::open(&h5_path)?)?;

        let setups_node = child(seq, "ViewSetups").ok_or_else(|| Error::Corrupt("BigDataViewer XML without ViewSetups".into()))?;
        let mut setups = Vec::new();
        for s in children(setups_node, "ViewSetup") {
            let Some(id) = text(s, "id").and_then(|v| v.parse().ok()) else { continue };
            let mut voxel_nm = [None; 3];
            if let Some(vs) = child(s, "voxelSize")
                && let Some(nm) = unit_nm(&text(vs, "unit").unwrap_or_default()) {
                    for (i, v) in numbers(&text(vs, "size").unwrap_or_default()).into_iter().take(3).enumerate() {
                        voxel_nm[i] = Some(v * nm).filter(|v| v.is_finite() && *v > 0.0);
                    }
                }
            let attributes = child(s, "attributes")
                .map(|a| a.children().filter(Node::is_element).filter_map(|c| Some((c.tag_name().name().to_string(), c.text()?.trim().parse().ok()?))).collect())
                .unwrap_or_default();
            setups.push(Setup { id, name: text(s, "name"), voxel_nm, attributes });
        }
        if setups.is_empty() {
            return Err(Error::Corrupt("BigDataViewer XML without view setups".into()));
        }
        // Attribute entity names, e.g. channel 1 -> "GFP".
        let mut entity_names: HashMap<(String, i64), String> = HashMap::new();
        for a in children(setups_node, "Attributes") {
            let attr = a.attribute("name").unwrap_or_default().to_string();
            for e in a.children().filter(Node::is_element) {
                if let (Some(id), Some(name)) = (text(e, "id").and_then(|v| v.parse().ok()), text(e, "name")) {
                    entity_names.insert((attr.clone(), id), name);
                }
            }
        }
        let timepoints = match child(seq, "Timepoints") {
            Some(t) => timepoints(t)?,
            None => vec![0],
        };
        if timepoints.is_empty() || timepoints.len().saturating_mul(setups.len()) > MAX_VIEWS {
            return Err(Error::Corrupt("BigDataViewer dataset without time points or with too many views".into()));
        }
        let mut missing = HashSet::new();
        if let Some(m) = child(seq, "MissingViews") {
            for v in children(m, "MissingView") {
                if let (Some(t), Some(s)) = (v.attribute("timepoint").and_then(|v| v.parse().ok()), v.attribute("setup").and_then(|v| v.parse().ok())) {
                    missing.insert((t, s));
                }
            }
        }

        let root_links = h5.links(&h5.object(h5.root)?)?;
        let tp_groups: HashMap<i64, u64> = root_links.iter().filter_map(|(n, a)| Some((n.strip_prefix('t')?.parse().ok()?, *a))).collect();
        let setup_groups: HashMap<i64, u64> = root_links.iter().filter_map(|(n, a)| Some((n.strip_prefix('s')?.parse().ok()?, *a))).collect();
        let mut bdv = Bdv {
            h5,
            tp_groups,
            timepoints,
            groups: Vec::new(),
            missing,
            volumes: HashMap::new(),
            level_sizes: HashMap::new(),
            metas: Vec::new(),
            original: OriginalMeta::default(),
        };

        // Levels of each setup: downsampling factors and the dataset sizes
        // of the first time point holding the view.
        let mut factors: HashMap<i64, Vec<f64>> = HashMap::new();
        let mut pixel_type = None;
        for s in &setups {
            let res = setup_groups.get(&s.id).and_then(|&g| bdv.h5.links(&bdv.h5.object(g).ok()?).ok()).and_then(|l| l.iter().find(|(n, _)| n == "resolutions").map(|(_, a)| *a));
            let f: Vec<f64> = res.and_then(|a| bdv.read_f64s(a)).map(|v| v.chunks_exact(3).map(|c| c[0]).collect()).unwrap_or_else(|| vec![1.0]);
            let mut sizes = Vec::new();
            for level in 0..f.len().max(1) {
                let found = (0..bdv.timepoints.len()).find_map(|t| bdv.lookup(s.id, t, level).ok().flatten());
                let Some(addr) = found else { break };
                let v = Volume::open(&bdv.h5, addr)?;
                let dims = match v.ds.dims[..] {
                    [z, y, x] => [z, y, x],
                    [y, x] => [1, y, x],
                    _ => return Err(Error::UnsupportedVariant("BigDataViewer volume dimensions".into())),
                };
                if level == 0 && pixel_type.is_none() {
                    pixel_type = Some(match (v.ds.dtype.kind, v.ds.dtype.size) {
                        // Unsigned data in an int16 dataset, by convention.
                        (Kind::Uint | Kind::Int, 2) => PixelType::U16,
                        (Kind::Uint, 1) => PixelType::U8,
                        (Kind::Int, 1) => PixelType::I8,
                        (Kind::Uint, 4) => PixelType::U32,
                        (Kind::Int, 4) => PixelType::I32,
                        (Kind::Float, 4) => PixelType::F32,
                        (Kind::Float, 8) => PixelType::F64,
                        (k, n) => return Err(Error::UnsupportedVariant(format!("BigDataViewer data type {k:?} of {n} bytes"))),
                    });
                }
                if level > 0 && (dims[2] > sizes.last().map_or(u64::MAX, |d: &[u64; 3]| d[2]) || dims.contains(&0)) {
                    break;
                }
                sizes.push(dims);
            }
            factors.insert(s.id, f);
            bdv.level_sizes.insert(s.id, sizes);
        }
        let pixel_type = pixel_type.ok_or_else(|| Error::Corrupt("BigDataViewer HDF5 without image data".into()))?;
        setups.retain(|s| bdv.level_sizes.get(&s.id).is_some_and(|l| !l.is_empty()));

        // Series: setups grouped by all attributes but `channel`.
        let mut keyed: Vec<(BTreeMap<String, i64>, Vec<&Setup>)> = Vec::new();
        for s in &setups {
            let key: BTreeMap<String, i64> = s.attributes.iter().filter(|(k, _)| *k != "channel").map(|(k, v)| (k.clone(), *v)).collect();
            match keyed.iter_mut().find(|(k, _)| *k == key) {
                Some((_, list)) if s.attributes.contains_key("channel") => list.push(s),
                _ => keyed.push((key, vec![s])),
            }
        }
        let mut groups = Vec::new();
        for (key, mut list) in keyed {
            list.sort_by_key(|s| s.attributes.get("channel").copied().unwrap_or(0));
            let first = &bdv.level_sizes[&list[0].id];
            let same = list.iter().all(|s| bdv.level_sizes[&s.id] == *first);
            if same {
                groups.push((key, list));
            } else {
                groups.extend(list.into_iter().map(|s| (s.attributes.clone(), vec![s])));
            }
        }
        let size_t = bdv.timepoints.len() as u32;
        for (key, list) in &groups {
            let s0 = list[0];
            let sizes = &bdv.level_sizes[&s0.id];
            let [z0, _, x0] = sizes[0];
            let f = &factors[&s0.id];
            let levels = sizes
                .iter()
                .enumerate()
                .map(|(l, &[_, y, x])| {
                    let tile = bdv.volumes_tile(s0.id, l);
                    LevelMeta { size_x: x, size_y: y, tile, downsample: f.get(l).copied().filter(|v| *v > 0.0).unwrap_or(x0 as f64 / x.max(1) as f64) }
                })
                .collect();
            let name = if list.len() == 1 {
                s0.name.clone().unwrap_or_else(|| format!("setup {}", s0.id))
            } else {
                let parts: Vec<String> = key.iter().map(|(k, v)| format!("{k} {}", entity_names.get(&(k.clone(), *v)).cloned().unwrap_or_else(|| v.to_string()))).collect();
                if parts.is_empty() { "setups ".to_string() + &list.iter().map(|s| s.id.to_string()).collect::<Vec<_>>().join(", ") } else { parts.join(", ") }
            };
            let channels = list
                .iter()
                .map(|s| ChannelMeta {
                    name: s.attributes.get("channel").and_then(|c| entity_names.get(&("channel".to_string(), *c)).cloned()).or_else(|| s.name.clone()),
                    ..Default::default()
                })
                .collect();
            bdv.metas.push(SeriesMeta {
                name: Some(name),
                size_c: list.len() as u32,
                size_z: z0 as u32,
                size_t,
                pixel_type,
                samples_per_pixel: 1,
                levels,
                physical_size: PhysicalSize { x_nm: s0.voxel_nm[0], y_nm: s0.voxel_nm[1], z_nm: s0.voxel_nm[2] },
                channels,
                ..Default::default()
            });
            bdv.groups.push(Group { setups: list.iter().map(|s| s.id).collect() });
        }
        for g in &bdv.groups {
            for &s in &g.setups {
                for t in 0..bdv.timepoints.len() {
                    for l in 0..bdv.level_sizes[&s].len() {
                        bdv.volumes.insert((s, t, l), OnceLock::new());
                    }
                }
            }
        }
        bdv.original.documents.push(("SpimData".into(), xml.clone()));
        bdv.original.pairs.push(("HDF5 file".into(), h5_name));
        Ok(bdv)
    }

    /// Object header of the `cells` dataset of a view, if stored.
    fn lookup(&self, setup: i64, t: usize, level: usize) -> Result<Option<u64>> {
        let tp = self.timepoints[t];
        if self.missing.contains(&(tp, setup)) {
            return Ok(None);
        }
        let Some(&g) = self.tp_groups.get(&tp) else { return Ok(None) };
        let h = &self.h5;
        let find = |addr: u64, name: &str| -> Result<Option<u64>> { Ok(h.links(&h.object(addr)?)?.into_iter().find(|(n, _)| n == name).map(|(_, a)| a)) };
        let Some(s) = find(g, &format!("s{setup:02}"))? else { return Ok(None) };
        let Some(l) = find(s, &level.to_string())? else { return Ok(None) };
        find(l, "cells")
    }

    /// The volume of a view, resolved on first use; `None` if not stored.
    fn volume(&self, setup: i64, t: usize, level: usize) -> Result<Option<&Volume>> {
        let slot = self.volumes.get(&(setup, t, level)).ok_or_else(|| Error::OutOfRange("BigDataViewer view".into()))?;
        let v = slot.get_or_init(|| {
            let resolve = || -> Result<Option<Volume>> {
                match self.lookup(setup, t, level)? {
                    Some(addr) => Volume::open(&self.h5, addr).map(Some),
                    None => Ok(None),
                }
            };
            resolve().map_err(|e| e.to_string())
        });
        v.as_ref().map(Option::as_ref).map_err(|e| Error::Corrupt(e.clone()))
    }

    /// Chunk width and height of a level, from its first stored view.
    fn volumes_tile(&self, setup: i64, level: usize) -> Option<(u32, u32)> {
        let addr = (0..self.timepoints.len()).find_map(|t| self.lookup(setup, t, level).ok().flatten())?;
        let v = Volume::open(&self.h5, addr).ok()?;
        matches!(v.ds.layout, Layout::Chunked { .. }).then(|| (v.chunk[2] as u32, v.chunk[1] as u32))
    }

    /// A small float dataset (e.g. `resolutions`) as `f64` values.
    fn read_f64s(&self, addr: u64) -> Option<Vec<f64>> {
        let v = Volume::open(&self.h5, addr).ok()?;
        let bytes = v.read_all(&self.h5, 1 << 16).ok()?;
        Some(bytes.chunks_exact(v.ds.dtype.size.max(1)).map(|b| super::hdf5::number(b, v.ds.dtype)).collect())
    }
}

impl Backend for Bdv {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let setup = self.groups[series].setups[plane.c as usize];
        let sizes = &self.level_sizes[&setup];
        let Some(v) = self.volume(setup, plane.t as usize, level)? else {
            for y in 0..region.height as usize {
                sink.write_with(y, 0, region.width as usize, |d| {
                    d.fill(0);
                    Ok(())
                })?;
            }
            return Ok(());
        };
        // Levels may hold fewer z planes than level 0: nearest plane.
        let (z_level, z_base) = (sizes[level][0].max(1), sizes[0][0].max(1));
        let z = (plane.z as u64 * z_level / z_base).min(z_level - 1);
        v.read_plane(&self.h5, z, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timepoint_patterns() {
        let doc = Document::parse("<T type=\"pattern\"><integerpattern>0-4:2, 7, 9-10</integerpattern></T>").unwrap();
        assert_eq!(timepoints(doc.root_element()).unwrap(), [0, 2, 4, 7, 9, 10]);
        let doc = Document::parse("<T type=\"range\"><first>3</first><last>5</last></T>").unwrap();
        assert_eq!(timepoints(doc.root_element()).unwrap(), [3, 4, 5]);
        let doc = Document::parse("<T type=\"range\"><first>0</first><last>99999999999</last></T>").unwrap();
        assert!(timepoints(doc.root_element()).is_err());
    }
}

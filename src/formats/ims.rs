//! # ims
//!
//! Bitplane Imaris 5 reader (`.ims`, HDF5).
//!
//! `DataSet/ResolutionLevel L/TimePoint T/Channel C/Data` holds a (z, y, x)
//! volume per level, time point and channel, possibly padded to whole
//! chunks; the `ImageSizeX/Y/Z` attributes of the channel group give the
//! real extent. `DataSetInfo` holds the metadata: `Image` (extent, physical
//! bounding box, unit, recording date, lens), `Channel C` (name, colour,
//! wavelengths) and `TimeInfo` (time point dates). Every attribute is an
//! array of one-character strings. Lower levels may hold fewer z planes;
//! every level serves all z planes of level 0, each from its nearest
//! stored plane. Datasets are resolved on first use; a
//! plane read decodes only the chunks it touches (see `formats/hdf5.rs`).
//!
//! Imaris 3 files (also `.ims`, TIFF based) are not this format.
//!
//! Provenance: Imaris file format description (Bitplane, `doc/bitplane-
//! imaris-ims/`); OpenReadout IMS notes (MIT OR Apache-2.0); HDF5 File
//! Format Specification (see `formats/hdf5.rs`); own analysis of sample files. No
//! GPL code or HDF5 library code consulted.
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
use std::sync::OnceLock;

use super::hdf5::{Attribute, H5, Kind, Layout, Volume};

use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::{Backend, NativeImage, native_image};
use crate::{
    AssociatedKind, ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OpenOptions, OriginalMeta,
    PhysicalSize, PixelType, Plane, PlaneMeta, Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Ims, "Imaris IMS", "1").extensions(&["ims"]);

/// Returns `true` for an HDF5 file with the `.ims` extension or an
/// `ImarisDataSet` attribute in its first bytes.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    H5::matches(head) && (path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ims")) || head.windows(13).any(|w| w == b"ImarisDataSet"))
}

#[derive(Debug)]
struct Level {
    /// Object header of each `TimePoint` group, by time index.
    timepoints: Vec<u64>,
    size: [u64; 3],
}

/// Bitplane Imaris 5 file.
#[derive(Debug)]
pub(super) struct Ims {
    h5: H5,
    levels: Vec<Level>,
    size_c: usize,
    /// Indexed by `(level * size_t + t) * size_c + c`.
    volumes: Vec<OnceLock<std::result::Result<Volume, String>>>,
    metas: Vec<SeriesMeta>,
    times: Vec<f64>,
    thumbnail: Option<u64>,
    associated_kinds: Vec<AssociatedKind>,
    info: Option<u64>,
}

/// Children of a group named `<prefix><number>`, sorted by number.
fn numbered(links: &[(String, u64)], prefix: &str) -> Vec<u64> {
    let mut v: Vec<(u64, u64)> = links
        .iter()
        .filter_map(|(n, a)| n.strip_prefix(prefix).and_then(|i| i.trim().parse::<u64>().ok()).map(|i| (i, *a)))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, a)| a).collect()
}

fn find(links: &[(String, u64)], name: &str) -> Option<u64> {
    links.iter().find(|(n, _)| n == name).map(|(_, a)| *a)
}

fn attr(attrs: &[Attribute], name: &str) -> Option<String> {
    attrs.iter().find(|a| a.name == name).map(Attribute::text).filter(|s| !s.is_empty())
}

fn attr_f64(attrs: &[Attribute], name: &str) -> Option<f64> {
    attr(attrs, name)?.split_whitespace().next()?.parse().ok()
}

/// Nanometres per unit of a length unit name.
fn unit_nm(unit: &str) -> Option<f64> {
    Some(match unit.trim() {
        "" | "um" | "µm" | "micron" | "microns" => 1e3,
        "nm" => 1.0,
        "mm" => 1e6,
        "m" => 1e9,
        _ => return None,
    })
}

/// Seconds since the epoch of `YYYY-MM-DD HH:MM:SS[.fff]` (local time,
/// treated as UTC; only differences are used).
fn seconds(s: &str) -> Option<f64> {
    let (date, time) = s.trim().split_once(' ')?;
    let d: Vec<i64> = date.split('-').filter_map(|v| v.parse().ok()).collect();
    let [y, m, day] = d[..] else { return None };
    let mut t = time.split(':');
    let (hh, mm, ss): (i64, i64, f64) = (t.next()?.parse().ok()?, t.next()?.parse().ok()?, t.next()?.parse().ok()?);
    // Days from civil (proleptic Gregorian).
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86400 + hh * 3600 + mm * 60) as f64 + ss)
}

impl Ims {
    /// Opens an Imaris 5 file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for a missing `DataSet` group and
    /// [`Error::UnsupportedVariant`] for HDF5 features not supported.
    pub fn open(path: &Path, _opts: &OpenOptions) -> Result<Self> {
        let h5 = H5::open(File::open(path)?)?;
        let root = h5.object(h5.root)?;
        let root_links = h5.links(&root)?;
        let dataset = find(&root_links, "DataSet").ok_or_else(|| Error::Corrupt("IMS without DataSet group".into()))?;
        let level_links = h5.links(&h5.object(dataset)?)?;
        let mut levels = Vec::new();
        let mut size_c = 0;
        for addr in numbered(&level_links, "ResolutionLevel ") {
            let timepoints = numbered(&h5.links(&h5.object(addr)?)?, "TimePoint ");
            let Some(&tp0) = timepoints.first() else { break };
            let channels = numbered(&h5.links(&h5.object(tp0)?)?, "Channel ");
            let Some(&ch0) = channels.first() else { break };
            if levels.is_empty() {
                size_c = channels.len();
            }
            let attrs = h5.attributes(&h5.object(ch0)?)?;
            let size = [attr_f64(&attrs, "ImageSizeZ"), attr_f64(&attrs, "ImageSizeY"), attr_f64(&attrs, "ImageSizeX")];
            let size = size.map(|v| v.unwrap_or(0.0).max(0.0) as u64);
            levels.push(Level { timepoints, size });
        }
        let first = levels.first().ok_or_else(|| Error::Corrupt("IMS without resolution levels".into()))?;
        let size_t = first.timepoints.len();
        // Levels need all time points and positive sizes.
        levels.retain(|l| l.timepoints.len() >= size_t);
        let info = find(&root_links, "DataSetInfo");
        let thumbnail = find(&root_links, "Thumbnail").and_then(|t| h5.object(t).ok()).and_then(|o| h5.links(&o).ok()).and_then(|l| find(&l, "Data"));
        let mut ims = Ims {
            h5,
            levels,
            size_c,
            volumes: Vec::new(),
            metas: Vec::new(),
            times: Vec::new(),
            thumbnail,
            associated_kinds: Vec::new(),
            info,
        };
        ims.volumes = (0..ims.levels.len() * size_t * size_c).map(|_| OnceLock::new()).collect();
        // Sizes from the data where the attributes are missing.
        let v0 = ims.volume(0, 0, 0)?;
        let dtype = v0.ds.dtype;
        let pixel_type = match (dtype.kind, dtype.size) {
            (Kind::Uint, 1) => PixelType::U8,
            (Kind::Uint, 2) => PixelType::U16,
            (Kind::Uint, 4) => PixelType::U32,
            (Kind::Int, 1) => PixelType::I8,
            (Kind::Int, 2) => PixelType::I16,
            (Kind::Int, 4) => PixelType::I32,
            (Kind::Float, 4) => PixelType::F32,
            (Kind::Float, 8) => PixelType::F64,
            (k, s) => return Err(Error::UnsupportedVariant(format!("IMS data type {k:?} of {s} bytes"))),
        };
        for l in 0..ims.levels.len() {
            if ims.levels[l].size.contains(&0) {
                let dims = ims.volume(l, 0, 0)?.ds.dims.clone();
                if dims.len() == 3 {
                    ims.levels[l].size = [dims[0], dims[1], dims[2]];
                }
            }
        }
        if ims.levels[0].size.contains(&0) {
            return Err(Error::Corrupt("IMS image size 0".into()));
        }
        ims.metas = vec![ims.series_meta(pixel_type, size_t)?];
        if ims.thumbnail.is_some() {
            ims.associated_kinds.push(AssociatedKind::Thumbnail);
        }
        Ok(ims)
    }

    fn series_meta(&mut self, pixel_type: PixelType, size_t: usize) -> Result<SeriesMeta> {
        let base = self.levels[0].size;
        let levels: Vec<LevelMeta> = (0..self.levels.len())
            .map(|l| {
                let s = self.levels[l].size;
                let tile = self.volume(l, 0, 0).ok().and_then(|v| matches!(v.ds.layout, Layout::Chunked { .. }).then(|| (v.chunk[2] as u32, v.chunk[1] as u32)));
                LevelMeta { size_x: s[2], size_y: s[1], tile, downsample: base[2] as f64 / s[2].max(1) as f64 }
            })
            .collect();
        let mut meta = SeriesMeta {
            size_c: self.size_c as u32,
            size_z: base[0] as u32,
            size_t: size_t as u32,
            pixel_type,
            samples_per_pixel: 1,
            levels,
            channels: vec![ChannelMeta::default(); self.size_c],
            ..Default::default()
        };
        let Some(info) = self.info else { return Ok(meta) };
        let links = self.h5.links(&self.h5.object(info)?)?;
        let group = |name: &str| -> Vec<Attribute> {
            find(&links, name).and_then(|a| self.h5.object(a).ok()).and_then(|o| self.h5.attributes(&o).ok()).unwrap_or_default()
        };
        let image = group("Image");
        let nm = unit_nm(&attr(&image, "Unit").unwrap_or_default());
        let extent = |axis: usize, size: u64| -> Option<f64> {
            let (lo, hi) = (attr_f64(&image, &format!("ExtMin{axis}"))?, attr_f64(&image, &format!("ExtMax{axis}"))?);
            let v = (hi - lo).abs() / size as f64 * nm?;
            (v.is_finite() && v > 0.0).then_some(v)
        };
        meta.physical_size = PhysicalSize { x_nm: extent(0, base[2]), y_nm: extent(1, base[1]), z_nm: extent(2, base[0]).filter(|_| base[0] > 1) };
        meta.name = attr(&image, "Name").map(|n| n.rsplit(['/', '\\']).next().unwrap_or(&n).to_string());
        meta.description = attr(&image, "Description");
        meta.acquisition_date = attr(&image, "RecordingDate").map(|d| d.replacen(' ', "T", 1));
        let mag = attr_f64(&image, "LensPower").filter(|v| *v > 0.0);
        let na = attr_f64(&image, "NumericalAperture").filter(|v| *v > 0.0);
        if mag.is_some() || na.is_some() {
            meta.objective = Some(ObjectiveMeta { magnification: mag, numerical_aperture: na, ..Default::default() });
        }
        for (c, ch) in meta.channels.iter_mut().enumerate() {
            let a = group(&format!("Channel {c}"));
            ch.name = attr(&a, "Name");
            ch.color = attr(&a, "Color").and_then(|s| {
                let v: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                (v.len() >= 3).then(|| [(v[0] * 255.0).round() as u8, (v[1] * 255.0).round() as u8, (v[2] * 255.0).round() as u8, 255])
            });
            ch.excitation_wavelength_nm = attr_f64(&a, "LSMExcitationWavelength").filter(|v| *v > 0.0);
            ch.emission_wavelength_nm = attr_f64(&a, "LSMEmissionWavelength").filter(|v| *v > 0.0);
        }
        let times = group("TimeInfo");
        let t: Vec<f64> = (1..=size_t).filter_map(|i| attr(&times, &format!("TimePoint{i}")).and_then(|s| seconds(&s))).collect();
        if t.len() == size_t && size_t > 1 {
            meta.time_increment_s = Some((t[size_t - 1] - t[0]) / (size_t - 1) as f64).filter(|v| *v > 0.0);
            self.times = t.iter().map(|v| v - t[0]).collect();
        }
        Ok(meta)
    }

    /// The `Data` dataset of (level, t, c), resolved on first use.
    fn volume(&self, level: usize, t: usize, c: usize) -> Result<&Volume> {
        let size_t = self.levels[0].timepoints.len();
        let slot = self.volumes.get((level * size_t + t) * self.size_c + c).ok_or_else(|| Error::OutOfRange("IMS plane".into()))?;
        slot.get_or_init(|| self.resolve(level, t, c).map_err(|e| e.to_string())).as_ref().map_err(|e| Error::Corrupt(e.clone()))
    }

    fn resolve(&self, level: usize, t: usize, c: usize) -> Result<Volume> {
        let h = &self.h5;
        let tp = *self.levels[level].timepoints.get(t).ok_or_else(|| Error::Corrupt("IMS time point missing".into()))?;
        let channels = numbered(&h.links(&h.object(tp)?)?, "Channel ");
        let ch = *channels.get(c).ok_or_else(|| Error::Corrupt(format!("IMS channel {c} missing")))?;
        let data = find(&h.links(&h.object(ch)?)?, "Data").ok_or_else(|| Error::Corrupt("IMS channel without Data".into()))?;
        Volume::open(h, data)
    }
}

impl Backend for Ims {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn plane_meta(&self, _series: usize, plane: Plane) -> Result<PlaneMeta> {
        Ok(PlaneMeta { delta_t_s: self.times.get(plane.t as usize).copied(), ..Default::default() })
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        let mut meta = OriginalMeta::default();
        let h = &self.h5;
        let root = h.object(h.root)?;
        for a in h.attributes(&root)? {
            meta.pairs.push((a.name.clone(), a.text()));
        }
        if let Some(info) = self.info {
            for (name, addr) in h.links(&h.object(info)?)? {
                if let Ok(obj) = h.object(addr) {
                    for a in h.attributes(&obj).unwrap_or_default() {
                        meta.pairs.push((format!("{name}/{}", a.name), a.text()));
                    }
                }
            }
        }
        Ok(meta)
    }

    fn associated_images(&self) -> &[AssociatedKind] {
        &self.associated_kinds
    }

    fn read_associated_native(&self, kind: &AssociatedKind) -> Result<NativeImage> {
        let (Some(addr), AssociatedKind::Thumbnail) = (self.thumbnail, kind) else {
            return Err(Error::OutOfRange(format!("no associated image {kind:?}")));
        };
        let ds = self.h5.dataset(&self.h5.object(addr)?)?;
        // (height, width * 4) RGBA bytes.
        let [h, w4] = ds.dims[..] else { return Err(Error::UnsupportedVariant("IMS thumbnail layout".into())) };
        if ds.dtype.size != 1 || w4 % 4 != 0 {
            return Err(Error::UnsupportedVariant("IMS thumbnail layout".into()));
        }
        let Layout::Contiguous { address, .. } = ds.layout else {
            return Err(Error::UnsupportedVariant("IMS chunked thumbnail".into()));
        };
        let len = h * w4;
        let at = self.h5.absolute(address, len)?;
        let region = Region::new(0, 0, w4 / 4, h);
        native_image(region, 4, PixelType::U8, |sink| {
            for y in 0..h as usize {
                sink.write_with(y, 0, (w4 / 4) as usize, |d| read_exact_at(&self.h5.file, d, at + y as u64 * w4))?;
            }
            Ok(())
        })
    }

    fn read_native(&self, _series: usize, level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        let v = self.volume(level, plane.t as usize, plane.c as usize)?;
        // Levels may hold fewer z planes than level 0: nearest plane.
        let (z_level, z_base) = (self.levels[level].size[0].max(1), self.levels[0].size[0].max(1));
        let z = (plane.z as u64 * z_level / z_base).min(z_level - 1);
        v.read_plane(&self.h5, z, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_units() {
        let a = seconds("2019-03-05 10:39:51.582").unwrap();
        let b = seconds("2019-03-05 10:40:01.582").unwrap();
        assert!((b - a - 10.0).abs() < 1e-9);
        assert_eq!(unit_nm("um"), Some(1e3));
        assert_eq!(unit_nm("pc"), None);
    }
}

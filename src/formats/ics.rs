//! # ics
//!
//! Image Cytometry Standard reader: ICS 1.0 (`.ics` header + `.ids` data,
//! optionally `.ids.gz`) and ICS 2.0 (data after the header's `end` line,
//! or in the file named by `source file`).
//!
//! The header is text: the first line gives the field and record
//! separators, then `category subcategory values...` lines. `layout order`
//! names the dimensions (after `bits`), fastest first, with
//! `layout sizes`; `representation` gives the number format (`integer`,
//! `real`), `sign`, `byte_order` (positions of the bytes, `1 2` little
//! endian, `2 1` big endian) and `compression` (`uncompressed`, `gzip`);
//! `parameter scale` / `units` give the sample spacing. Dimensions `x` and
//! `y` are the plane, `z` the z planes, `t` the time points; `ch`, `c`,
//! `channel`, `p` (probe) and every other dimension are channels (in
//! layout order, fastest first). With `layout coordinates cartesian` the
//! origin is the lower left pixel and rows are returned from the top.
//! Samples are read with strides, so any dimension order works; gzip data
//! are decompressed into memory on first read. `sensor` parameters give the
//! wavelengths and numerical aperture.
//!
//! Provenance: Dean et al., "Proposed standard for image cytometry data
//! files", Cytometry 11 (1990); libics documentation (Ics_Header,
//! Ics_DataRepresentation, Ics_ImelRepresentation; documentation only, the
//! LGPL code was not consulted) and the SVI ICS page in `doc/ics/`; own
//! analysis of the ICS samples on the OME sample server (CC BY 4.0);
//! checked against Bio-Formats and bioformats-rs as black boxes. No GPL or
//! LGPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-04
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::strided::{Data, Strided};
use crate::io::read_exact_at;
use crate::pixel::Sink;
use crate::reader::Backend;
use crate::{
    ChannelMeta, Error, FormatId, FormatInfo, LevelMeta, ObjectiveMeta, OriginalMeta, PhysicalSize, PixelType, Plane,
    Region, Result, SeriesMeta,
};

pub(super) static FORMAT: FormatInfo = FormatInfo::new(FormatId::Ics, "ICS (Image Cytometry Standard)", "1").extensions(&["ics", "ids", "ids.gz"]);

/// Largest header parsed.
const MAX_HEADER: u64 = 16 << 20;
/// Largest decompressed gzip data held in memory.
const MAX_DECOMPRESSED: u64 = 4 << 30;

fn lower_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// The `.ics` header of an `.ics`, `.ids` or `.ids.gz` path.
fn header_path(path: &Path) -> Option<PathBuf> {
    let name = lower_name(path);
    let stem = name.strip_suffix(".gz").unwrap_or(&name);
    if stem.ends_with(".ics") {
        return Some(path.to_path_buf());
    }
    let base = path.file_name()?.to_string_lossy();
    let cut = base.len() - (name.len() - stem.len()) - 4;
    stem.ends_with(".ids").then(|| ["ics", "ICS"].iter().map(|e| path.with_file_name(format!("{}.{e}", &base[..cut]))).find(|p| p.is_file())).flatten()
}

fn looks_like_ics(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    text.contains("layout") && (text.contains("ics_version") || text.contains("order"))
}

/// Returns `true` for an ICS header or its data file.
pub(super) fn matches(path: &Path, head: &[u8]) -> bool {
    let Some(hp) = header_path(path) else { return false };
    if hp == path {
        return looks_like_ics(head);
    }
    let mut h = Vec::with_capacity(512);
    File::open(&hp).and_then(|f| std::io::Read::read_to_end(&mut std::io::Read::take(f, 512), &mut h)).is_ok() && looks_like_ics(&h)
}

/// Header lines as fields, and the offset after the `end` line (ICS 2.0).
fn parse_header(raw: &[u8]) -> (Vec<Vec<String>>, Option<usize>) {
    // The first line holds the field and record separators.
    let (fs, rs, start) = match raw {
        [f, r, ..] if !f.is_ascii_alphanumeric() && !r.is_ascii_alphanumeric() => (*f, *r, 2),
        _ => (b'\t', b'\n', 0),
    };
    let mut lines = Vec::new();
    let mut at = start;
    while at < raw.len() {
        let end = raw[at..].iter().position(|&b| b == rs).map_or(raw.len(), |p| at + p);
        let line = String::from_utf8_lossy(&raw[at..end]);
        let line = line.trim_end_matches('\r');
        let next = end + 1;
        if line.trim() == "end" {
            return (lines, Some(next.min(raw.len())));
        }
        let fields: Vec<String> = line.split(fs as char).flat_map(|f| if fs == b'\t' { vec![f] } else { f.split('\t').collect() }).map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect();
        if !fields.is_empty() {
            lines.push(fields);
        }
        at = next;
    }
    (lines, None)
}

/// Axis roles of the stored dimensions (after `bits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    X,
    Y,
    Z,
    C,
    T,
}

/// ICS image.
#[derive(Debug)]
pub(super) struct Ics {
    samples: Strided,
    metas: Vec<SeriesMeta>,
    original: OriginalMeta,
}

impl Ics {
    /// Opens an ICS header or its data file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Corrupt`] for an invalid header or short data and
    /// [`Error::UnsupportedVariant`] for complex samples, other byte orders
    /// or compression methods.
    pub fn open(path: &Path) -> Result<Self> {
        let hp = header_path(path).ok_or_else(|| Error::UnsupportedFormat(path.display().to_string()))?;
        let file = File::open(&hp)?;
        let len = file.metadata()?.len();
        let mut raw = vec![0u8; len.min(MAX_HEADER) as usize];
        read_exact_at(&file, &mut raw, 0)?;
        let (lines, data_at) = parse_header(&raw);
        let get = |a: &str, b: &str| lines.iter().find(|l| l.len() >= 2 && l[0] == a && l[1] == b).map(|l| &l[2..]);
        let order: Vec<String> = get("layout", "order").map(|v| v.iter().map(|s| s.to_ascii_lowercase()).collect()).unwrap_or_default();
        let sizes: Vec<u64> = get("layout", "sizes").map(|v| v.iter().filter_map(|s| s.parse().ok()).collect()).unwrap_or_default();
        if order.len() < 3 || sizes.len() != order.len() || order[0] != "bits" {
            return Err(Error::Corrupt("ICS header without a valid layout".into()));
        }
        let bits = sizes[0];
        let format = get("representation", "format").and_then(|v| v.first()).map(|s| s.to_ascii_lowercase()).unwrap_or_else(|| "integer".into());
        let signed = get("representation", "sign").and_then(|v| v.first()).map_or(format != "integer", |s| s.eq_ignore_ascii_case("signed"));
        let pixel_type = match (format.as_str(), bits, signed) {
            ("integer", 8, false) => PixelType::U8,
            ("integer", 8, true) => PixelType::I8,
            ("integer", 16, false) => PixelType::U16,
            ("integer", 16, true) => PixelType::I16,
            ("integer", 32, false) => PixelType::U32,
            ("integer", 32, true) => PixelType::I32,
            ("real" | "float", 32, _) => PixelType::F32,
            ("real" | "float", 64, _) => PixelType::F64,
            (f, b, _) => return Err(Error::UnsupportedVariant(format!("ICS {f} samples of {b} bits"))),
        };
        let bytes = pixel_type.bytes();
        let little = match get("representation", "byte_order").map(|v| v.iter().filter_map(|s| s.parse::<usize>().ok()).collect::<Vec<_>>()) {
            None => true,
            Some(o) if o.len() <= 1 || o.iter().copied().eq(1..=o.len()) => true,
            Some(o) if o.iter().copied().eq((1..=o.len()).rev()) => false,
            Some(o) => return Err(Error::UnsupportedVariant(format!("ICS byte order {o:?}"))),
        };
        let gzip = match get("representation", "compression").and_then(|v| v.first()).map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("uncompressed") => false,
            Some("gzip" | "compress") => true,
            Some(c) => return Err(Error::UnsupportedVariant(format!("ICS compression {c}"))),
        };
        // Strides and roles of the dimensions after `bits`.
        let mut stride = 1u64;
        let (mut sx, mut sy, mut sz) = (None, None, 0u64);
        let (mut nx, mut ny, mut nz) = (1u64, 1u64, 1u64);
        let (mut c_axes, mut t_axes) = (Vec::new(), Vec::new());
        let scales: Vec<f64> = get("parameter", "scale").map(|v| v.iter().filter_map(|s| s.parse().ok()).collect()).unwrap_or_default();
        let units: Vec<String> = get("parameter", "units").map(|v| v.to_vec()).unwrap_or_default();
        let mut spacing = [None; 4];
        for (i, (name, &size)) in order.iter().zip(&sizes).enumerate().skip(1) {
            let size = size.max(1);
            let role = match name.as_str() {
                "x" => Role::X,
                "y" => Role::Y,
                "z" => Role::Z,
                "t" => Role::T,
                _ => Role::C,
            };
            let scale = scales.get(i).copied().filter(|v| v.is_finite() && *v > 0.0);
            let unit = units.get(i).map(|u| u.to_ascii_lowercase()).unwrap_or_default();
            match role {
                Role::X if sx.is_none() => (sx, nx, spacing[0]) = (Some(stride), size, scale.and_then(|s| length_nm(s, &unit))),
                Role::Y if sy.is_none() => (sy, ny, spacing[1]) = (Some(stride), size, scale.and_then(|s| length_nm(s, &unit))),
                Role::Z if nz == 1 => (sz, nz, spacing[2]) = (stride, size, scale.and_then(|s| length_nm(s, &unit))),
                Role::T => {
                    spacing[3] = scale.and_then(|s| time_s(s, &unit));
                    t_axes.push((stride, size))
                }
                // A second x, y or z axis counts as channels too.
                _ => c_axes.push((stride, size)),
            }
            stride = stride.checked_mul(size).ok_or_else(|| Error::Corrupt("ICS dimensions too large".into()))?;
        }
        let (Some(sx), Some(sy)) = (sx, sy) else {
            return Err(Error::Corrupt("ICS layout without x and y".into()));
        };
        let total = stride.checked_mul(bytes as u64).ok_or_else(|| Error::Corrupt("ICS data too large".into()))?;
        let size_c: u64 = c_axes.iter().map(|a| a.1).product();
        let size_t: u64 = t_axes.iter().map(|a| a.1).product();
        if [nx, ny, nz, size_c, size_t].iter().any(|&v| v > u32::MAX as u64) {
            return Err(Error::Corrupt("ICS dimensions too large".into()));
        }

        // Data: after `end` (2.0), a named source file, or the .ids file (1.0).
        let version2 = lines.iter().any(|l| l[0] == "ics_version" && l.get(1).is_some_and(|v| v.starts_with('2')));
        let source = lines.iter().find(|l| l.len() >= 3 && l[0] == "source" && l[1] == "file");
        let dir = hp.parent().unwrap_or(Path::new("."));
        let (data_path, offset) = if let Some(s) = source {
            let name = Path::new(&s[2].replace('\\', "/")).file_name().map(|n| dir.join(n)).unwrap_or_default();
            let offset = s.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
            (if name.is_file() { name } else { hp.clone() }, offset)
        } else if version2 {
            (hp.clone(), data_at.ok_or_else(|| Error::Corrupt("ICS 2.0 header without end line".into()))? as u64)
        } else {
            let stem = hp.with_extension("");
            let ids = ["ids", "IDS", "ids.gz", "IDS.GZ"].iter().map(|e| stem.with_extension(e)).find(|p| p.is_file());
            (ids.ok_or_else(|| Error::Corrupt(format!("ICS data file of {} missing", hp.display())))?, 0)
        };
        let gz_file = lower_name(&data_path).ends_with(".gz");
        let data = if gzip || gz_file {
            if total > MAX_DECOMPRESSED {
                return Err(Error::UnsupportedVariant("compressed ICS data above 4 GiB".into()));
            }
            Data::gzip(&data_path, offset, 0, total, "ICS")
        } else {
            Data::file(&data_path, offset, total, "ICS")?
        };

        // Sensor parameters: values per channel.
        let sensor = |key: &str| -> Vec<f64> {
            lines.iter().find(|l| l.len() >= 4 && l[0] == "sensor" && l[1] == "s_params" && l[2] == key).map(|l| l[3..].iter().filter_map(|v| v.parse().ok()).collect()).unwrap_or_default()
        };
        let (ex, em) = (sensor("LambdaEx"), sensor("LambdaEm"));
        let channels = (0..size_c as usize)
            .map(|c| ChannelMeta {
                excitation_wavelength_nm: ex.get(c).copied().filter(|v| *v > 0.0),
                emission_wavelength_nm: em.get(c).copied().filter(|v| *v > 0.0),
                ..Default::default()
            })
            .collect();
        let na = sensor("NumAperture").first().copied().filter(|v| *v > 0.0);
        let meta = SeriesMeta {
            name: hp.file_stem().map(|n| n.to_string_lossy().into_owned()),
            size_c: size_c as u32,
            size_z: nz as u32,
            size_t: size_t as u32,
            pixel_type,
            significant_bits: get("layout", "significant_bits").and_then(|v| v.first()).and_then(|v| v.parse::<u8>().ok()).filter(|&b| (b as usize) < bytes * 8),
            samples_per_pixel: 1,
            levels: vec![LevelMeta { size_x: nx, size_y: ny, tile: None, downsample: 1.0 }],
            physical_size: PhysicalSize { x_nm: spacing[0], y_nm: spacing[1], z_nm: spacing[2].filter(|_| nz > 1) },
            time_increment_s: spacing[3].filter(|_| size_t > 1),
            channels,
            objective: na.map(|na| ObjectiveMeta { numerical_aperture: Some(na), ..Default::default() }),
            ..Default::default()
        };
        let original = OriginalMeta { pairs: lines.iter().filter(|l| l.len() >= 2).map(|l| (l[..2.min(l.len())].join(" "), l[2..].join(" "))).collect(), documents: Vec::new() };
        let flip_y = get("layout", "coordinates").and_then(|v| v.first()).is_some_and(|c| c.eq_ignore_ascii_case("cartesian"));
        let samples = Strided { data, sx, sy, sz, c_axes, t_axes, bytes, little, flip_y, height: ny, total, what: "ICS" };
        Ok(Ics { samples, metas: vec![meta], original })
    }
}

/// A spacing in nanometres; micrometres if the unit is missing.
fn length_nm(v: f64, unit: &str) -> Option<f64> {
    let f = match unit {
        "" | "micrometers" | "micrometer" | "microns" | "micron" | "um" | "µm" => 1e3,
        "nanometers" | "nanometer" | "nm" => 1.0,
        "millimeters" | "millimeter" | "mm" => 1e6,
        "meters" | "meter" | "m" => 1e9,
        _ => return None,
    };
    Some(v * f)
}

fn time_s(v: f64, unit: &str) -> Option<f64> {
    Some(v * match unit {
        "seconds" | "second" | "s" | "sec" => 1.0,
        "milliseconds" | "ms" => 1e-3,
        "microseconds" | "us" => 1e-6,
        _ => return None,
    })
}

impl Backend for Ics {
    fn format(&self) -> &'static FormatInfo {
        &FORMAT
    }

    fn series(&self) -> &[SeriesMeta] {
        &self.metas
    }

    fn original_metadata(&self, _series: usize) -> Result<OriginalMeta> {
        Ok(self.original.clone())
    }

    fn read_native(&self, _series: usize, _level: usize, plane: Plane, region: Region, sink: &mut Sink) -> Result<()> {
        self.samples.read_plane(plane, region, sink)
    }
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_separators_and_end() {
        let raw = b"\t\nics_version\t2.0\nlayout\torder\tbits\tx\ty\nend\n\x01\x02";
        let (lines, end) = parse_header(raw);
        assert_eq!(lines[1], ["layout", "order", "bits", "x", "y"]);
        assert_eq!(end, Some(raw.len() - 2));
    }
}

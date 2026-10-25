//! # scopeio_info
//!
//! Reads every plane of a file and prints metadata and a pixel checksum.
//! Used by `tests/scripts/oracle.py` to compare against reference readers
//! and to measure CPU time and memory.
//!
//! Usage: `cargo run --release --example scopeio_info -- [--no-checksum] <file>`
//!
//! With `--no-checksum` only the pixels are read, for timing the reader.
//! `--dump <file>` writes the raw pixels of series 0 and `--meta` prints the
//! series metadata, both for debugging. `--series N` reads only series N and
//! `--crop x,y,w,h` only that region of level 0.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

#[path = "../tests/common/crc32.rs"]
mod crc32;

use scopeio::{ChannelSelect, OutputSpec, PixelBuf, PixelViewMut, Region, RegionRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let checksum = !args.iter().any(|a| a == "--no-checksum");
    args.retain(|a| a != "--no-checksum");
    // `--dump <file>` writes the raw pixels of all planes of series 0 (or the
    // one `--series` names).
    let dump = args.iter().position(|a| a == "--dump").map(|i| {
        let path = args.get(i + 1).cloned().unwrap_or_default();
        args.drain(i..(i + 2).min(args.len()));
        path
    });
    let mut dumped = Vec::new();
    // `--meta` prints the series metadata.
    let show_meta = args.iter().any(|a| a == "--meta");
    args.retain(|a| a != "--meta");
    // `--series N` reads one series, `--level L` one pyramid level (default
    // 0), `--crop x,y,w,h` one region of that level.
    let mut option = |name: &str| {
        args.iter().position(|a| a == name).map(|i| {
            let v = args.get(i + 1).cloned().unwrap_or_default();
            args.drain(i..(i + 2).min(args.len()));
            v
        })
    };
    let only_series = option("--series").map(|v| v.parse::<usize>()).transpose()?;
    let level_index = option("--level").map(|v| v.parse::<usize>()).transpose()?.unwrap_or(0);
    let crop = option("--crop")
        .map(|v| {
            let n: Vec<u64> = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            if n.len() == 4 { Ok(Region::new(n[0], n[1], n[2], n[3])) } else { Err("crop must be x,y,w,h") }
        })
        .transpose()?;
    let path = args.first().ok_or("usage: scopeio_info [--no-checksum] [--meta] [--dump <file>] <file>")?;
    let reader = scopeio::open(path.as_ref())?;
    for series in 0..reader.series_count() {
        if only_series.is_some_and(|s| s != series) {
            continue;
        }
        let meta = reader.series_meta(series)?;
        if show_meta {
            println!("{meta:#?}");
            let mut original = reader.original_metadata(series)?;
            original.documents.iter_mut().for_each(|(_, d)| d.truncate(200));
            println!("{original:#?}");
        }
        let Some(level) = meta.levels.get(level_index) else { continue };
        let mut req = RegionRequest::new(series, level_index, crop.unwrap_or(Region::new(0, 0, level.size_x, level.size_y)));
        req.output = OutputSpec { keep_alpha: true, ..Default::default() };
        let shape = reader.output_shape(&req)?;
        // One buffer, reused for every plane.
        let mut buf = PixelBuf::zeros(shape.pixel_type, shape.len());
        let mut crc = crc32::Crc32::new();
        for t in 0..meta.size_t {
            for z in 0..meta.size_z {
                for c in 0..meta.size_c {
                    req.channels = ChannelSelect::Channel(c);
                    (req.z, req.t) = (z, t);
                    let view = PixelViewMut::contiguous(buf.as_slice_mut(), shape.width, shape.height, shape.channels, shape.layout);
                    reader.read_region_into(&req, view)?;
                    if checksum {
                        crc.update(buf.as_bytes());
                    }
                    if dump.is_some() && series == only_series.unwrap_or(0) {
                        dumped.extend_from_slice(buf.as_bytes());
                    }
                }
            }
        }
        let nm = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.3}"));
        println!(
            "series={series}\twidth={}\theight={}\tsamples={}\tsize_c={}\tsize_z={}\tsize_t={}\tpixel_type={:?}\tphys_x_nm={}\tphys_y_nm={}\tcrc32={:08x}",
            req.region.width,
            req.region.height,
            meta.samples_per_pixel,
            meta.size_c,
            meta.size_z,
            meta.size_t,
            meta.pixel_type,
            nm(meta.physical_size.x_nm),
            nm(meta.physical_size.y_nm),
            crc.finish()
        );
    }
    if let Some(path) = dump {
        std::fs::write(path, dumped)?;
    }
    Ok(())
}

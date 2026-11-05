//! # extract_region
//!
//! Reads one region of one plane and saves it as an 8-bit PGM (gray) or
//! PPM (RGB) image, scaled from the sample range of the region.
//!
//! Usage: `cargo run --release --example extract_region -- <file> <x> <y>
//! <width> <height> [--series N] [--level L] [--c C] [--z Z] [--t T]
//! [--out region.pgm]`
//!
//! The region is in pixels of the chosen level and is clipped to it.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-05
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::io::Write;

use scopeio::{ChannelSelect, OutputSpec, OutputType, Region, RegionRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut option = |name: &str| -> Option<String> {
        let i = args.iter().position(|a| a == name)?;
        let value = args.get(i + 1).cloned();
        args.drain(i..(i + 2).min(args.len()));
        value
    };
    let number = |v: Option<String>| -> Result<u64, Box<dyn std::error::Error>> { Ok(v.as_deref().unwrap_or("0").parse()?) };
    let series = number(option("--series"))? as usize;
    let level = number(option("--level"))? as usize;
    let (c, z, t) = (number(option("--c"))? as u32, number(option("--z"))? as u32, number(option("--t"))? as u32);
    let out = option("--out");
    let [path, x, y, w, h] = &args[..] else {
        return Err("usage: extract_region <file> <x> <y> <width> <height> [--series N] [--level L] [--c C] [--z Z] [--t T] [--out FILE]".into());
    };

    let reader = scopeio::open(path.as_ref())?;
    let meta = reader.series_meta(series)?;
    let lv = meta.levels.get(level).ok_or("no such level")?;
    let (x, y) = (x.parse::<u64>()?.min(lv.size_x - 1), y.parse::<u64>()?.min(lv.size_y - 1));
    let (w, h) = (w.parse::<u64>()?.min(lv.size_x - x), h.parse::<u64>()?.min(lv.size_y - y));
    println!("{}: series {series} level {level}, {}x{} px, region {w}x{h} at ({x}, {y})", reader.format().name, lv.size_x, lv.size_y);

    // Samples as f32, then stretched to 0..255 over the region's range.
    let mut req = RegionRequest::new(series, level, Region::new(x, y, w, h));
    (req.z, req.t, req.channels) = (z, t, ChannelSelect::Channel(c));
    req.output = OutputSpec { dtype: OutputType::F32, ..Default::default() };
    let image = reader.read_region(&req)?;
    let channels = image.channels as usize;
    let samples: Vec<f32> = image.into_vec()?;
    let (lo, hi) = samples.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let scale = if hi > lo { 255.0 / (hi - lo) } else { 0.0 };
    let bytes: Vec<u8> = samples.iter().map(|&v| ((v - lo) * scale).round() as u8).collect();

    let (magic, keep) = match channels {
        1 => ("P5", 1),
        _ => ("P6", 3),
    };
    let pixels: Vec<u8> = if channels == keep {
        bytes
    } else {
        // Two-sample or alpha images: the first samples, gray replicated.
        bytes.chunks_exact(channels).flat_map(|p| [p[0], p[1.min(channels - 1)], p[2.min(channels - 1)]]).collect()
    };
    let out = out.unwrap_or_else(|| format!("region.{}", if keep == 1 { "pgm" } else { "ppm" }));
    let mut file = std::io::BufWriter::new(std::fs::File::create(&out)?);
    write!(file, "{magic}\n{w} {h}\n255\n")?;
    file.write_all(&pixels)?;
    file.flush()?;
    println!("samples {lo} .. {hi} -> {out}");
    Ok(())
}

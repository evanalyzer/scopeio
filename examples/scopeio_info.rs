//! # scopeio_info
//!
//! Reads every plane of a file and prints metadata and a pixel checksum.
//! Used by `tests/scripts/oracle.py` to compare against reference readers
//! and to measure CPU time and memory.
//!
//! Usage: `cargo run --release --example scopeio_info -- [--no-checksum] <file>`
//!
//! With `--no-checksum` only the pixels are read, for timing the reader.
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

use scopeio::{Plane, Region};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let checksum = !args.iter().any(|a| a == "--no-checksum");
    args.retain(|a| a != "--no-checksum");
    let path = args.first().ok_or("usage: scopeio_info [--no-checksum] <file>")?;
    let reader = scopeio::open(&path)?;
    for series in 0..reader.series_count() {
        let meta = reader.series_meta(series)?;
        let level = &meta.levels[0];
        let region = Region::new(0, 0, level.width as u32, level.height as u32);
        let mut buf = vec![0u8; meta.buffer_size(region)];
        let mut crc = crc32::Crc32::new();
        for t in 0..meta.size_t {
            for z in 0..meta.size_z {
                for c in 0..meta.size_c {
                    reader.read_region_into(series, 0, Plane::new(c, z, t), region, &mut buf)?;
                    if checksum {
                        crc.update(&buf);
                    }
                }
            }
        }
        let nm = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.3}"));
        println!(
            "series={series}\twidth={}\theight={}\tsamples={}\tsize_c={}\tsize_z={}\tsize_t={}\tpixel_type={:?}\tphys_x_nm={}\tphys_y_nm={}\tcrc32={:08x}",
            level.width,
            level.height,
            meta.color.samples(),
            meta.size_c,
            meta.size_z,
            meta.size_t,
            meta.pixel_type,
            nm(meta.physical_size.x_nm),
            nm(meta.physical_size.y_nm),
            crc.finish()
        );
    }
    Ok(())
}

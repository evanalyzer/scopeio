//! # output_bench
//!
//! Reads one region in a given way, for measuring CPU time and peak RAM of
//! output options with `tests/scripts/bench_output.py` (one process per
//! run, so the peak RSS belongs to that scenario alone).
//!
//! Usage: `output_bench <scenario> <file> <x,y,w,h> [z]`
//!
//! Scenarios:
//! - `native`: native samples.
//! - `native-convert`: native samples, then converted by the caller to f32
//!   divided by 4095 (12-bit data) into a second buffer.
//! - `f32-normalized`: f32 divided by the type maximum.
//! - `f32-significant-bits`: f32 divided by 2^significant_bits - 1.
//! - `f32-scaled`: f32 times 1/4095.
//! - `rgb-3-sample-reads`: three `ChannelSelect::Sample` reads (R, G, B)
//!   into the planes of one Chw buffer.
//! - `rgb-channel-chw`: one `ChannelSelect::Channel` read (Chw).
//!
//! Prints a checksum of the output so the reads cannot be optimised away.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-25
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use scopeio::{ChannelSelect, Layout, OutputSpec, OutputType, PixelViewMut, Region, RegionRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [scenario, path, region, rest @ ..] = args.as_slice() else {
        return Err("usage: output_bench <scenario> <file> <x,y,w,h> [z]".into());
    };
    let r: Vec<u64> = region.split(',').map(|v| v.parse()).collect::<Result<_, _>>()?;
    let [x, y, w, h] = r[..] else { return Err("region is x,y,w,h".into()) };
    let z: u32 = rest.first().map(|v| v.parse()).transpose()?.unwrap_or(0);
    let reader = scopeio::open(path.as_ref())?;
    let mut req = RegionRequest::new(0, 0, Region::new(x, y, w, h));
    req.z = z;
    let plane = (w * h) as usize;
    let sum: f64 = match scenario.as_str() {
        "native" => reader.read_region(&req)?.into_vec::<u16>()?.iter().map(|&v| v as f64).sum(),
        "native-convert" => {
            let native = reader.read_region(&req)?.into_vec::<u16>()?;
            let f: Vec<f32> = native.iter().map(|&v| v as f32 / 4095.0).collect();
            f.iter().map(|&v| v as f64).sum()
        }
        "f32-normalized" | "f32-significant-bits" | "f32-scaled" => {
            req.output.dtype = match scenario.as_str() {
                "f32-normalized" => OutputType::F32Normalized,
                "f32-significant-bits" => OutputType::F32SignificantBits,
                _ => OutputType::F32Scaled { scale: 1.0 / 4095.0, offset: 0.0 },
            };
            reader.read_region(&req)?.into_vec::<f32>()?.iter().map(|&v| v as f64).sum()
        }
        "rgb-3-sample-reads" => {
            req.output = OutputSpec { layout: Layout::Chw, ..Default::default() };
            let mut out = vec![0u8; 3 * plane];
            for (s, chunk) in out.chunks_exact_mut(plane).enumerate() {
                req.channels = ChannelSelect::Sample { channel: 0, sample: s as u8 };
                reader.read_region_into(&req, PixelViewMut::contiguous(chunk, w, h, 1, Layout::Chw))?;
            }
            out.iter().map(|&v| v as f64).sum()
        }
        "rgb-channel-chw" => {
            req.channels = ChannelSelect::Channel(0);
            req.output = OutputSpec { layout: Layout::Chw, ..Default::default() };
            let mut out = vec![0u8; 3 * plane];
            reader.read_region_into(&req, PixelViewMut::contiguous(out.as_mut_slice(), w, h, 3, Layout::Chw))?;
            out.iter().map(|&v| v as f64).sum()
        }
        s => return Err(format!("unknown scenario {s}").into()),
    };
    println!("{scenario}\t{sum}");
    Ok(())
}

//! # api
//!
//! Acceptance tests of the public API: output layouts, colour order, dtype
//! conversion, channel selection, tiles, validation and concurrent reads.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

mod common;

use std::sync::Arc;

use scopeio::{
    ChannelSelect, ColorOrder, Error, ImageBuf, ImageReader, Layout, OutputSpec, OutputType, PixelBuf, PixelViewMut,
    Region, RegionRequest, TileRequest,
};

use common::root;

fn open(rel: &str) -> Arc<dyn ImageReader> {
    scopeio::open(&root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn full(reader: &dyn ImageReader, series: usize) -> Region {
    let l = &reader.series_meta(series).unwrap().levels[0];
    Region::new(0, 0, l.size_x, l.size_y)
}

fn read(reader: &dyn ImageReader, channels: ChannelSelect, output: OutputSpec) -> ImageBuf {
    let mut req = RegionRequest::new(0, 0, full(reader, 0));
    req.channels = channels;
    req.output = output;
    reader.read_region(&req).unwrap()
}

/// Element `(y, x, c)` of a buffer as f64, independent of layout.
fn at(img: &ImageBuf, y: usize, x: usize, c: usize) -> f64 {
    let (w, h, n) = (img.width as usize, img.height as usize, img.channels as usize);
    let i = match img.layout {
        Layout::Hwc => (y * w + x) * n + c,
        Layout::Chw => c * w * h + y * w + x,
    };
    match &img.data {
        PixelBuf::U8(v) => v[i] as f64,
        PixelBuf::U16(v) => v[i] as f64,
        PixelBuf::F32(v) => v[i] as f64,
        other => panic!("unexpected buffer type {:?}", other.pixel_type()),
    }
}

#[test]
fn storage_variants_give_identical_buffers() {
    let names = [
        "same_rgb16_le_chunky.tif",
        "same_rgb16_be_planar.tif",
        "same_rgb16_le_tiles_deflate_pred.tif",
        "same_rgb16_be_bigtiff_lzw.tif",
    ];
    for layout in [Layout::Hwc, Layout::Chw] {
        let spec = OutputSpec { layout, ..Default::default() };
        let reference = read(open(&format!("tests/data/tiff/{}", names[0])).as_ref(), ChannelSelect::Channel(0), spec);
        for name in &names[1..] {
            let img = read(open(&format!("tests/data/tiff/{name}")).as_ref(), ChannelSelect::Channel(0), spec);
            assert_eq!(img, reference, "{name} {layout:?}");
        }
    }
}

#[test]
fn hwc_and_chw_hold_the_same_values() {
    for (file, select) in [
        ("tests/data/tiff/same_rgb16_le_chunky.tif", ChannelSelect::Channel(0)),
        ("tests/data/tiff/gray3_planar.tif", ChannelSelect::Channels(vec![2, 0, 1])),
    ] {
        let r = open(file);
        let hwc = read(r.as_ref(), select.clone(), OutputSpec::default());
        let chw = read(r.as_ref(), select, OutputSpec { layout: Layout::Chw, ..Default::default() });
        assert_eq!(hwc.shape(), [chw.shape()[1], chw.shape()[2], chw.shape()[0]]);
        for y in 0..hwc.height as usize {
            for x in 0..hwc.width as usize {
                for c in 0..hwc.channels as usize {
                    assert_eq!(at(&hwc, y, x, c), at(&chw, y, x, c), "{file} ({y}, {x}, {c})");
                }
            }
        }
    }
}

#[test]
fn channels_selection_matches_single_channels() {
    let r = open("tests/data/tiff/gray3_planar.tif");
    let stacked = read(r.as_ref(), ChannelSelect::Channels(vec![2, 0]), OutputSpec { layout: Layout::Chw, ..Default::default() });
    let c2 = read(r.as_ref(), ChannelSelect::Channel(2), OutputSpec::default());
    let c0 = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec::default());
    let plane = c2.as_bytes().len();
    assert_eq!(&stacked.as_bytes()[..plane], c2.as_bytes());
    assert_eq!(&stacked.as_bytes()[plane..], c0.as_bytes());
}

#[test]
fn rgb_bgr_and_sample_selection() {
    let r = open("tests/data/tiff/same_rgb16_le_chunky.tif");
    let rgb = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec::default());
    let bgr = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec { color_order: ColorOrder::Bgr, ..Default::default() });
    assert_eq!(bgr.color_order, ColorOrder::Bgr);
    for y in 0..rgb.height as usize {
        for x in 0..rgb.width as usize {
            for c in 0..3 {
                assert_eq!(at(&rgb, y, x, c), at(&bgr, y, x, 2 - c));
            }
        }
    }
    for sample in 0..3u8 {
        let one = read(r.as_ref(), ChannelSelect::Sample { channel: 0, sample }, OutputSpec::default());
        assert_eq!(one.channels, 1);
        for y in 0..rgb.height as usize {
            for x in 0..rgb.width as usize {
                assert_eq!(at(&one, y, x, 0), at(&rgb, y, x, sample as usize));
            }
        }
    }
}

#[test]
fn alpha_is_dropped_unless_kept() {
    let r = open("tests/data/tiff/rgba16_be.tif");
    assert_eq!(read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec::default()).channels, 3);
    let kept = OutputSpec { keep_alpha: true, ..Default::default() };
    assert_eq!(read(r.as_ref(), ChannelSelect::Channel(0), kept).channels, 4);
}

#[test]
fn float_outputs_match_native_values() {
    let r = open("tests/data/tiff/same_rgb16_le_chunky.tif");
    let native = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec::default());
    let f32 = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec { dtype: OutputType::F32, ..Default::default() });
    let norm = read(r.as_ref(), ChannelSelect::Channel(0), OutputSpec { dtype: OutputType::F32Normalized, ..Default::default() });
    let range = OutputSpec { dtype: OutputType::F32Range { min: 1000.0, max: 2000.0 }, ..Default::default() };
    let ranged = read(r.as_ref(), ChannelSelect::Channel(0), range);
    for y in 0..native.height as usize {
        for x in 0..native.width as usize {
            for c in 0..3 {
                let v = at(&native, y, x, c);
                assert_eq!(at(&f32, y, x, c), v);
                assert!((at(&norm, y, x, c) - v / 65535.0).abs() < 1e-6);
                let expected = ((v - 1000.0) / 1000.0).clamp(0.0, 1.0);
                assert!((at(&ranged, y, x, c) - expected).abs() < 1e-6);
            }
        }
    }
}

#[test]
fn tiles_assemble_to_the_region() {
    for file in ["tests/data/tiff/rgb8_tiles.tif", "tests/data/tiff/u8_lzw.tif", "tests/data/ome-tiff/pyramid_u16.ome.tif"] {
        let r = open(file);
        let meta = r.series_meta(0).unwrap();
        for (level_index, level) in meta.levels.iter().enumerate() {
            let mut req = RegionRequest::new(0, level_index, Region::new(0, 0, level.size_x, level.size_y));
            req.output.keep_alpha = true;
            let whole = r.read_region(&req).unwrap();
            let shape = r.output_shape(&req).unwrap();
            let mut assembled = PixelBuf::zeros(shape.pixel_type, shape.len());
            let (across, down) = level.tile_counts();
            let row = level.size_x as usize * shape.channels as usize;
            for ty in 0..down {
                for tx in 0..across {
                    let rect = level.tile_rect(tx, ty).unwrap();
                    let tile = TileRequest { level: level_index, tile_x: tx, tile_y: ty, output: req.output, ..Default::default() };
                    // A strided view into the assembled image.
                    let start = rect.y as usize * row + rect.x as usize * shape.channels as usize;
                    let data: scopeio::PixelSliceMut = match &mut assembled {
                        PixelBuf::U8(v) => (&mut v[start..]).into(),
                        PixelBuf::U16(v) => (&mut v[start..]).into(),
                        other => panic!("unexpected {:?}", other.pixel_type()),
                    };
                    let view = PixelViewMut::strided(data, rect.width, rect.height, shape.channels, Layout::Hwc, row, 0);
                    r.read_tile_into(&tile, view).unwrap();
                }
            }
            assert_eq!(assembled, whole.data, "{file} level {level_index}");
        }
    }
}

#[test]
fn requests_are_validated() {
    let r = open("tests/data/ome-tiff/pyramid_u16.ome.tif");
    let level1 = r.series_meta(0).unwrap().levels[1].clone();
    // Validated against the requested level, not level 0.
    let mut req = RegionRequest::new(0, 1, Region::new(0, 0, level1.size_x + 1, 1));
    assert!(matches!(r.read_region(&req), Err(Error::OutOfRange(_))));
    req.region = Region::new(0, 0, level1.size_x, 1);
    assert!(r.read_region(&req).is_ok());
    for bad in [
        RegionRequest { level: 9, ..req.clone() },
        RegionRequest { series: 1, ..req.clone() },
        RegionRequest { z: 1, ..req.clone() },
        RegionRequest { channels: ChannelSelect::Channel(1), ..req.clone() },
        RegionRequest { channels: ChannelSelect::Sample { channel: 0, sample: 1 }, ..req.clone() },
        RegionRequest { channels: ChannelSelect::Channels(vec![]), ..req.clone() },
    ] {
        assert!(matches!(r.read_region(&bad), Err(Error::OutOfRange(_))), "{bad:?}");
    }
    // Wrong element type and wrong shape.
    let mut wrong = vec![0u8; level1.size_x as usize];
    let view = PixelViewMut::contiguous(&mut wrong[..], level1.size_x, 1, 1, Layout::Hwc);
    assert!(matches!(r.read_region_into(&req, view), Err(Error::TypeMismatch(_))));
    let mut short = vec![0u16; level1.size_x as usize - 1];
    let view = PixelViewMut::strided(&mut short[..], level1.size_x, 1, 1, Layout::Hwc, level1.size_x as usize, 0);
    assert!(matches!(r.read_region_into(&req, view), Err(Error::TypeMismatch(_))));
}

#[test]
fn concurrent_reads_are_identical_to_sequential() {
    for file in ["tests/data/ome-tiff/pyramid_u16.ome.tif", "tests/data/tiff/rgb8_tiles.tif", "tests/data/tiff_im/rgb8_jpeg_tiles.tif"] {
        let r = open(file);
        let meta = r.series_meta(0).unwrap();
        let mut requests = Vec::new();
        for (level, l) in meta.levels.iter().enumerate() {
            let (across, down) = l.tile_counts();
            for ty in 0..down {
                for tx in 0..across {
                    requests.push(TileRequest { level, tile_x: tx, tile_y: ty, ..Default::default() });
                }
            }
        }
        let read_tile = |r: &dyn ImageReader, t: &TileRequest| {
            let rect = r.series_meta(0).unwrap().levels[t.level].tile_rect(t.tile_x, t.tile_y).unwrap();
            let req = RegionRequest { level: t.level, region: rect, ..Default::default() };
            r.read_region(&req).unwrap()
        };
        let sequential: Vec<ImageBuf> = requests.iter().map(|t| read_tile(r.as_ref(), t)).collect();
        std::thread::scope(|scope| {
            for thread in 0..32usize {
                let (r, requests, sequential) = (r.clone(), &requests, &sequential);
                scope.spawn(move || {
                    // Different threads walk the tiles in different orders.
                    for i in 0..requests.len() * 4 {
                        let k = (i * 7 + thread * 13) % requests.len();
                        assert_eq!(read_tile(r.as_ref(), &requests[k]), sequential[k], "{file} tile {k}");
                    }
                });
            }
        });
    }
}

#[test]
fn format_info_and_probe() {
    let r = open("tests/data/ome-tiff/pyramid_u16.ome.tif");
    assert_eq!(r.format().id, scopeio::FormatId::OmeTiff);
    assert_eq!(r.format().crate_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(scopeio::probe(&root().join("tests/data/bmp/rle8.bmp")).unwrap(), Some(scopeio::FormatId::Bmp));
    assert_eq!(scopeio::probe(&root().join("tests/data/tiff/u8_le.tif")).unwrap(), Some(scopeio::FormatId::Tiff));
    assert_eq!(scopeio::probe(&root().join("tests/data/tiff/truth.tsv")).unwrap(), None);
    assert!(scopeio::supported_formats().len() >= 3);
    let original = r.original_metadata(0).unwrap();
    assert!(original.documents.iter().any(|(name, xml)| name == "OME-XML" && xml.contains("<OME")));
}

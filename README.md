# microSCOPE IO

A pure-Rust reader for microscopy and whole-slide images: fast, low on memory
and easy to use. One API for TIFF, OME-Zarr and the common vendor formats; no
Java, no C libraries, no unsafe code outside the I/O layer.

- **Clean-room**: written from format specifications and sample files.
- **Lazy**: opening reads metadata only; pixels are decoded per tile on
  request, so gigapixel slides need little RAM.
- **Ready-to-use output**: typed, native-endian samples in HWC or CHW layout,
  optionally normalised to `f32`, for OpenCV, ndarray or tch.
- **Thread-safe**: share one reader between threads and read tiles in
  parallel.

## Usage

```rust
use scopeio::{ChannelSelect, Layout, OutputSpec, Region, RegionRequest};

let reader = scopeio::open("image.ome.tif".as_ref())?;
let meta = reader.series_meta(0)?;
println!("{} x {} px, {} channels", meta.levels[0].size_x, meta.levels[0].size_y, meta.size_c);

// A 512 x 512 region of level 0, channels 0..3 as CHW.
let mut req = RegionRequest::new(0, 0, Region::new(0, 0, 512, 512));
req.channels = ChannelSelect::Channels(vec![0, 1, 2]);
req.output = OutputSpec { layout: Layout::Chw, ..Default::default() };
let data: Vec<u16> = reader.read_region(&req)?.into_vec()?;
```

Lengths are in nanometres and times in seconds throughout. Each format and
each optional codec is a cargo feature; all are on by default.

### Output options

`OutputSpec::dtype` converts in the same pass as the read, without a native
buffer:

| `OutputType` | Value |
|---|---|
| `Native` | stored type, lossless |
| `F32` | value as f32 |
| `F32Normalized` | value / type maximum (int8: -128 -> -1.008, 127 -> 1.0) |
| `F32SignificantBits` | value / (2^significant_bits - 1): 12-bit data in u16 -> 0..1 |
| `F32NormalizedShifted` | (value - type min) / (type max - type min), like ImageJ (int8: -128 -> 0.0, 127 -> 1.0) |
| `F32Range { min, max }` | (value - min) / (max - min), clamped to 0..1 |
| `F32Scaled { scale, offset }` | value * scale + offset, not clamped |

Float data is passed through unchanged by the normalizing types. Reading a
3554 x 2540 u16 plane as `F32SignificantBits` peaks at 41 MiB, against 58 MiB
for reading it natively and converting afterwards, at equal CPU time.

`ChannelSelect::Samples { channel, samples }` returns selected samples of a
colour channel (stored order: 0 red, 1 green, 2 blue) as separate output
channels from one decode of each tile, e.g. red and blue as a `[2, H, W]`
tensor with `Layout::Chw`:

```rust
req.channels = ChannelSelect::Samples { channel: 0, samples: vec![0, 2] };
req.output = OutputSpec { layout: Layout::Chw, ..Default::default() };
```

For a 4096 x 4096 region of an RGB JPEG slide (CMU-1.svs) one `Samples`
request needs 0.09 s CPU, three `Sample` requests 0.16 s (0.29 s before this
change); peak RAM is the same (53 MiB, mostly the output). Numbers for all
options: `tests/expected/output_bench.md`.

`supported_formats()` lists each format with its `extensions` (lower case,
multi-part ones such as `ome.tif` and `nii.gz` included), fixed
`file_names` (e.g. `Index.idx.xml`) and `opens_folder` (OME-Zarr), so a
file dialog can be filtered without a list of its own:

```rust
let ok = scopeio::supported_formats().iter().any(|f| f.matches_name("cells.ome.tif"));
```

### Examples

- `cargo run --release --example scopeio_info -- --meta <file>`: series
  metadata and a checksum of every plane.
- `cargo run --release --example extract_region -- <file> <x> <y> <w> <h>
  [--series N] [--level L] [--c C] [--z Z] [--t T] [--out region.pgm]`: one
  region as an 8-bit PGM / PPM.

## Performance

CPU time and peak RAM of scopeio against Bio-Formats 8.5.0, bioformats-rs
0.1.8 and OpenSlide 3.4.1 on public sample files (Intel Core Ultra 9 185H,
median of 3 runs, each a fresh process). An "op" is one measured read: a
whole file, a crop or a pyramid level. Factors are how much more CPU or RAM
the other reader needs for the same op (geometric mean); "–" means not
compared, "n/a" means the reader cannot open the format.

| Format | Ops | scopeio CPU (median) | scopeio RAM (median) | Bio-Formats CPU / RAM | bioformats-rs CPU / RAM | OpenSlide CPU / RAM |
|---|---:|---:|---:|---:|---:|---:|
| TIFF | 80 | 1 ms | 5.5 MiB | 1089× / 21× | 3.0× / 2.3× | – |
| OME-TIFF | 96 | 1 ms | 5.7 MiB | 1198× / 23× | 3.9× / 2.4× | – |
| OME-Zarr | 14 | 1 ms | 5.2 MiB | n/a | 3.5× / 4.1× | – |
| Olympus VSI | 6 | 16 ms | 14 MiB | 111× / 22× | 2.2× / 2.1× | – |
| Nikon ND2 | 11 | 43 ms | 7.9 MiB | 139× / 36× | 3.2× / 4.8× | – |
| Zeiss CZI | 35 | 7 ms | 7.7 MiB | 462× / 44× | 2.0× / 2.7× | – |
| Leica LIF | 12 | 4 ms | 6.8 MiB | 1619× / 79× | 2.2× / 5.0× | – |
| Aperio SVS | 13 | 217 ms | 25 MiB | 5.8× / 2.2× | 11.3× / 1.9× | – |
| Hamamatsu NDPI | 20 | 96 ms | 28 MiB | 11.7× / 4.4× | 5.4× / 3.6× | – |
| 3DHistech MRXS | 13 | 8 ms | 8.7 MiB | n/a | n/a | 4.7× / 1.9× |
| Zeiss LSM | 6 | 8 ms | 7.0 MiB | 229× / 24× | 5.3× / 5.1× | – |
| Akoya QPTIFF | 9 | 190 ms | 19 MiB | 5.7× / 5.9× | 1.3× / 5.8× | – |
| Leica SCN | 7 | 227 ms | 35 MiB | 19.3× / 5.1× | – | – |
| Ventana BIF | 10 | 889 ms | 28 MiB | 4.2× / 2.1× | – | 1.1× / 2.6× |
| Imaris IMS | 4 | 42 ms | 8.8 MiB | 147× / 33× | 4.9× / 14.0× | – |
| Windows BMP | 17 | 1 ms | 4.4 MiB | 757× / 22× | 2.0× / 2.9× | – |
| PNG | 12 | 1 ms | 3.6 MiB | 374× / 22× | 2.3× / 3.1× | – |
| JPEG | 3 | 1 ms | 3.4 MiB | 228× / 16× | 1.9× / 2.5× | – |
| JPEG 2000 | 3 | 1 ms | 3.7 MiB | 147× / 10× | 2.2× / 2.0× | – |
| Olympus OIB/OIF | 3 | 1 ms | 5.6 MiB | 270× / 55× | 7.3× / 3.1× | – |
| Olympus OIR | 15 | 18 ms | 12 MiB | 252× / 46× | 2.9× / 5.6× | – |
| DICOM | 17 | 56 ms | 20 MiB | 39× / 9.2× | 2.6× / 1.9× | – |
| Zeiss ZVI | 14 | 7 ms | 8.1 MiB | 227× / 27× | 3.1× / 4.2× | – |
| Leica LOF/XLEF | 65 | 29 ms | 8.3 MiB | 157× / 42× | 4.2× / 10.9× | – |
| MetaMorph STK/ND | 4 | 16 ms | 6.7 MiB | 378× / 51× | 6.8× / 11.4× | – |
| DeltaVision | 7 | 2 ms | 5.5 MiB | 353× / 49× | 2.3× / 7.3× | – |
| PerkinElmer Operetta | 7 | 64 ms | 20 MiB | 22× / 14.0× | 1.2× / 1.3× | – |
| MRC | 7 | 1 ms | 5.0 MiB | 868× / 24× | 2.7× / 2.4× | – |
| Olympus SIS TIFF | 4 | 6 ms | 15 MiB | 478× / 14.8× | 4.2× / 1.9× | – |
| Olympus FluoView TIFF | 2 | 1 ms | 5.5 MiB | 1377× / 19.6× | 3.5× / 1.8× | – |
| NIfTI | 16 | 3 ms | 5.1 MiB | 436× / 33× | 3.8× / 3.1× | – |
| Olympus scan^R | 2 | 157 ms | 8.0 MiB | 214× / 28× | see below | – |
| Nikon NIS-Elements TIFF | 5 | 5 ms | 13 MiB | 333× / 14.7× | 4.1× / 1.9× | – |
| Nikon NEF (uncompressed) | 2 | 18 ms | 17 MiB | 224× / 18.3× | 5.8× / 3.8× | – |
| Olympus OMP2INFO | 1 | 927 ms | 96 MiB | 387× / 6.2× | 201× / 65× | – |
| Micro-Manager | 11 | 2 ms | 5.6 MiB | 456× / 31× | 2.9× / 5.5× | – |
| BigDataViewer XML/HDF5 | 3 | 9.62 s | 60 MiB | n/a | 4.0× / 18.1× | – |
| Analyze 7.5 | 4 | 1 ms | 5.0 MiB | 985× / 21× | 3.0× / 2.4× | – |
| Hamamatsu VMS | 4 | 358 ms | 19 MiB | 13.9× / 11.0× | n/a | – |
| OME-XML | 46 | 1 ms | 5.6 MiB | 986× / 25× | 2.6× / 2.0× | – |
| Zeiss Xradia XRM | 3 | 1 ms | 5.1 MiB | 211× / 38× | n/a | – |
| ICS | 5 | 1 ms | 5.2 MiB | 404× / 20× | 4.1× / 3.5× | – |
| Imspector OBF | 6 | 19 ms | 7.2 MiB | 140× / 22× | 2.2× / 3.9× | – |
| IN Cell Analyzer | 4 | 46 ms | 6.8 MiB | 94× / 53× | n/a | – |
| Becker & Hickl SDT | 4 | 51 ms | 28 MiB | 284× / 19.5× | 3.0× / 2.2× | – |
| Gatan DigitalMicrograph | 6 | 5 ms | 9.0 MiB | 334× / 14.5× | 2.1× / 2.1× | – |
| Opera Flex | 4 | 9 ms | 8.4 MiB | 261× / 27× | 2.4× / 3.8× | – |
| Yokogawa CV7000 | 4 | 12 ms | 7.8 MiB | 158× / 25× | n/a | – |
| MetaXpress | 6 | 13 ms | 8.1 MiB | 165× / 26× | 0.8× / 2.7× | – |
| NRRD | 10 | 4 ms | 5.2 MiB | 213× / 40× | 11.2× / 6.6× | – |
| Keller Lab Block | 7 | 15 ms | 6.6 MiB | 98× / 14.6× | 3.0× / 1.7× | – |
| AVI | 16 | 1 ms | 5.2 MiB | 853× / 24× | 2.2× / 2.3× | – |
| AmiraMesh | 14 | 1 ms | 5.3 MiB | 655× / 22× | 2.5× / 2.3× | – |
| Hamamatsu DCIMG | 12 | 1 ms | 6.2 MiB | 416× / 24× | 3.7× / 5.8× | – |
| Philips TIFF | 4 | 29 ms | 20 MiB | 148× / 14.1× | – | – |
| ScanImage TIFF | 18 | 1 ms | 5.5 MiB | 1840× / 28× | 6.0× / 2.2× | – |
| Bruker Prairie View | 14 | 1 ms | 5.6 MiB | 1116× / 21× | 3.0× / 2.3× | – |
| CEOS Panta Rhei | 6 | 1 ms | 5.6 MiB | n/a | n/a | – |

- Bio-Formats' large factors on small files are mostly JVM start-up (about
  8 s CPU and 550 MB per process); on whole-slide formats it needs 4–19×
  the CPU and 2–7× the RAM.
- The reference readers also write the pixels to disk (ICS output);
  scopeio only decodes.
- bioformats-rs failed on 98 of the ops that scopeio read.
- Large plain images (24-megapixel RGB): PNG 0.22 s / 74 MiB (Bio-Formats
  1.95 s / 354 MiB, bioformats-rs 0.57 s / 216 MiB), JPEG 0.16 s / 147 MiB
  (3.89 s / 601 MiB, 0.50 s / 220 MiB), lossless JPEG 2000 2.9 s / 579 MiB
  (14.5 s / 716 MiB, 4.6 s / 401 MiB).
- Operetta: bioformats-rs converts only the first series of the 29-series
  omer export (108 of 3132 planes), so its factor is from the export it reads
  whole; per plane scopeio needs 8 ms, bioformats-rs 11 ms.
- Olympus FluoView TIFF: no public sample was found; measured on the
  synthetic files only (checked against Bio-Formats).
- scan^R: bioformats-rs converts only the first of the 357 series of the
  idr0009 plate; per plane scopeio needs 0.6 ms, bioformats-rs 14 ms. The
  plate was measured in one run per reader.
- NEF: scopeio returns the raw sensor (CFA) values; Bio-Formats and
  bioformats-rs demosaic to RGB. Nikon-compressed NEF (the camera default)
  is not supported.
- OMP2INFO: scopeio reads the vendor stitch and all 25 tiles; the reference
  readers build their own mosaic instead (see Format notes).
- Micro-Manager: the reference readers read NDTiff files as plain TIFF
  (pages as time points) and ignore the index; scopeio's NDTiff output is
  checked against synthetic files instead. On the 4.6 GB two-file stack
  of the OME samples (8820 planes) Bio-Formats fails on OME-XML naming
  files that do not exist and bioformats-rs exceeds 8 GiB of RAM; scopeio
  reads it from the index maps in 2.6 s CPU and 5.1 MiB.
- BigDataViewer: Bio-Formats fails on both public samples; bioformats-rs
  converts only the first setup (scopeio: all 2 and 9 setups), so per
  setup scopeio needs about a third of its CPU time.
- Analyze 7.5: no public image pair with a clear licence was found;
  measured on the synthetic files only (checked against both readers).
- Hamamatsu VMS: crops of 1-2 megapixels and the 122-megapixel map; the
  first read of a JPEG file indexes its restart markers (0.3 s for 300 MB).
  bioformats-rs has no crop or series option and cannot convert the
  7.8-gigapixel slide whole.
- Imspector OBF: bioformats-rs reads one plane of the first stack only,
  so its factor compares a single plane with scopeio's whole file.
- IN Cell Analyzer: bioformats-rs cannot parse the Latin-1 `.xdce` files.
- Becker & Hickl SDT: a ZIP-compressed image block stores the time bins
  fastest, so scopeio decompresses it once into memory (2 GiB for the
  512 x 512 x 4096 sample) and needs about twice Bio-Formats' RAM there.
- CEOS Panta Rhei: Bio-Formats and bioformats-rs cannot open the format.
  The one real sample (2048 x 2048 uint16, 8 MiB) reads in 6 ms CPU and
  13.8 MiB; RosettaSciIO 0.15.0 needs 14 ms for the read alone, plus 1.9 s
  CPU to import, and 72 MiB peak (`tests/expected/prz_rsciio_bench.md`).
- Peaks above the medians (up to a few GB) come from reading a whole
  gigapixel level into one output buffer, not from the reader itself.
- scopeio rows re-measured 2026-11-20 (reference rows from their original
  runs). The RAM floor of about 5 MiB on small files includes the pages of
  the executable itself, which vary by about 1 MiB with the page cache.

Per-file numbers: `tests/expected/*_bench.md`; how to regenerate them:
[tests/README.md](tests/README.md).

## Supported formats

✅ = supported by scopeio.

| Format Name | File Extension(s) |Supported|
| --- | --- | --- |
| **3DHistech MIRAX** | `.mrxs` | ✅ |
| **3i SlideBook** | `.sld` |  |
| **3i SlideBook 7** | `.sldy` |  |
| **Adobe Photoshop PSD** | `.psd` |  |
| **AIM** | `.aim` |  |
| **Alicona 3D** | — |  |
| **Amersham Biosciences Gel** | `.gel` |  |
| **Amira Mesh** | `.am, .amiramesh, .grey, .hx, .labels` | ✅ |
| **Amnis FlowSight** | `.cif` |  |
| **Analyze 7.5** | `.img, .hdr` | ✅ |
| **Andor Bio-Imaging Division (ABD) TIFF** | `.tif` |  |
| **Andor SIF** | `.sif` |  |
| **Animated PNG** | `.png` |  |
| **Aperio AFI** | `.afi, .svs` | ❌ not possible yet (see Format notes) |
| **Aperio SVS TIFF** | `.svs` | ✅ |
| **Applied Precision CellWorX** | `.htd, .pnl` |  |
| **AVI (Audio Video Interleave)** | `.avi` | ✅ uncompressed, MSRLE, Motion JPEG (lossy, see notes) |
| **Axon Raw Format** | `.arf` |  |
| **BD Pathway** | `.exp, .tif` |  |
| **BDV** | `.xml, .h5` | ✅ |
| **Becker & Hickl SPC FIFO** | `.spc` |  |
| **Becker & Hickl SPCImage** | `.sdt` | ✅ |
| **Bio-Rad Gel** | `.1sc` |  |
| **Bio-Rad PIC** | `.pic, .raw, .xml` |  |
| **Bio-Rad SCN** | `.scn` |  |
| **BioTek Cytation** | `.tif, .tiff` |  |
| **Bitplane Imaris** | `.ims` | ✅ |
| **Bruker MRI** | — |  |
| **Burleigh** | — |  |
| **Canon DNG** | `.cr2, .crw` |  |
| **CellH5** | `.ch5` |  |
| **CEOS Panta Rhei** | `.prz` | ✅ |
| **Cellomics** | — |  |
| **CellVoyager** | `.xml, .tif` |  |
| **CV7000** | `.wpi, .tif` | ✅ |
| **DeltaVision** | `.dv, .r3d` | ✅ |
| **DICOM** (incl. whole-slide imaging) | `.dcm, .dicom` | ✅ |
| **ECAT7** | `.v` |  |
| **EPS (Encapsulated PostScript)** | `.eps, .epsi, .ps` |  |
| **Evotec/PerkinElmer Opera Flex** | `.flex, .mea, .res` | ✅ `.flex` |
| **FEI** | `.img` |  |
| **FEI TIFF** | `.tiff` |  |
| **FITS (Flexible Image Transport System)** | `.fits` |  |
| **Gatan Digital Micrograph** | `.dm3, .dm4` | ✅ |
| **Gatan Digital Micrograph 2** | `.dm2` |  |
| **GIF (Graphics Interchange Format)** | `.gif` |  |
| **Hamamatsu Aquacosmos NAF** | `.naf` |  |
| **Hamamatsu DCIMG** | `.dcimg` | ✅ |
| **Hamamatsu HIS** | — |  |
| **Hamamatsu ndpi** | `.ndpi, .ndpis` | ✅ |
| **Hamamatsu VMS** | `.vms` | ✅ |
| **Hitachi S-4800** | `.txt, .tif, .bmp, .jpg` |  |
| **I2I** | `.i2i` |  |
| **ICS (Image Cytometry Standard)** | `.ics, .ids` | ✅ |
| **Imacon** | `.fff` |  |
| **ImagePro Sequence** | — |  |
| **ImagePro Workspace** | — |  |
| **IMAGIC** | — |  |
| **IMOD** | `.mod` |  |
| **Improvision Openlab LIFF** | — |  |
| **Improvision Openlab Raw** | — |  |
| **Improvision TIFF** | `.tif` |  |
| **Imspector OBF** | `.obf, .msr` | ✅ (MSR only with OBF header) |
| **InCell 1000/2000** | `.xdce, .tif` | ✅ |
| **InCell 3000** | `.frm` |  |
| **INR** | — |  |
| **Inveon** | `.hdr` |  |
| **Ionpath MIBI** | `.tif, .tiff` |  |
| **IPLab** | `.ipl` |  |
| **IVision** | `.ipm` |  |
| **JDCE** | `.jdce, .tif` |  |
| **JEOL** | `.dat, .img, .par` |  |
| **JPEG** | `.jpg, .jpeg` | ✅ |
| **JPEG 2000** | `.jp2, .j2k` | ✅ |
| **JPK** | — |  |
| **JPX** | — |  |
| **Keller Lab Block** | `.klb` | ✅ |
| **Khoros VIFF Bitmap** | — |  |
| **Kodak BIP** | — |  |
| **Lambert Instruments FLIM** | — |  |
| **LaVision Imspector** | — |  |
| **Leica LAS AF LIF** | `.lif` | ✅ |
| **Leica LCS LEI** | — |  |
| **Leica LOF** | `.lof` | ✅ |
| **Leica SCN** | `.scn` | ✅ |
| **Leica XLEF** | `.xlef, .xlcf, .xlif` | ✅ |
| **LEO** | `.sxm, .tif, .tiff` |  |
| **Li-Cor L2D** | `.l2d, .tif, .scn` |  |
| **LIM (Laboratory Imaging/Nikon)** | `.lim` |  |
| **MetaMorph 7.5 TIFF** | `.tiff` |  |
| **MetaMorph Stack (STK)** | `.stk, .nd` | ✅ |
| **MetaXpress** | `.htd, .tif` | ✅ |
| **MIAS (Maia Scientific)** | `.tif` |  |
| **Micro-Manager** (image stacks, separate files, NDTiff) | `.tif, .txt, .index` | ✅ |
| **Mikroscan TIFF** | `.tif, .tiff` |  |
| **MINC MRI** | `.mnc` |  |
| **Minolta MRW** | — |  |
| **MNG (Multiple-image Network Graphics)** | `.mng` |  |
| **Molecular Imaging** | `.stp` |  |
| **MRC (Medical Research Council)** | `.mrc, .st, .ali, .map, .rec, .mrcs` | ✅ |
| **NEF (Nikon Electronic Format)** | `.nef, .tif` | ✅ uncompressed raw only |
| **NIfTI** | `.img, .hdr, .nii, .nii.gz` | ✅ |
| **Nikon Elements TIFF** | `.tiff` | ✅ |
| **Nikon EZ-C1 TIFF** | `.tiff` | ❌ not possible (see Format notes) |
| **Nikon NIS-Elements ND2** | `.nd2` | ✅ |
| **NRRD (Nearly Raw Raster Data)** | `.nrrd, .nhdr, .raw, .txt` | ✅ |
| **Olympus CellR/APL** | `.apl, .mtb, .tnb, .tif, .obsep` |  |
| **Olympus FluoView FV1000** | `.oib, .oif` | ✅ |
| **Olympus FluoView TIFF** | `.tif` | ✅ |
| **Olympus OIR** | `.oir` | ✅ |
| **Olympus OMP2INFO** | `.omp2info` | ✅ |
| **Olympus ScanR** | `.xml, .dat, .tif` | ✅ |
| **Olympus SIS TIFF** | `.tiff` | ✅ |
| **Olympus/Evident VSI** | `.vsi, .ets` | ✅ |
| **OME-TIFF** | `.ome.tiff, .ome.tif, .ome.tf2, .ome.tf8, .ome.btf` | ✅ |
| **OME-XML** | `.ome, .ome.xml` | ✅ |
| **OME-Zarr (NGFF 0.4 / 0.5)** | `.zarr` | ✅ |
| **OMERO Pyramid** | — |  |
| **Oxford Instruments** | `.top` |  |
| **PCORAW** | `.pcoraw, .rec` |  |
| **PCX (PC Paintbrush)** | `.pcx` |  |
| **Perkin Elmer Densitometer** | `.pds` |  |
| **PerkinElmer Columbus** | — |  |
| **PerkinElmer Nuance** | `.im3` |  |
| **PerkinElmer Operetta** | `.tiff, .xml` | ✅ |
| **PerkinElmer UltraVIEW** | — |  |
| **PerkinElmer Vectra/QPTIFF** | `.tif, .tiff, .qptiff` | ✅ |
| **Photoshop TIFF** | `.tif, .tiff` |  |
| **PicoQuant Bin** | `.bin` |  |
| **PICT (Macintosh Picture)** | `.pict` |  |
| **PNG (Portable Network Graphics)** | `.png` | ✅ |
| **Portable Any Map** | `.pbm, .pgm, .ppm` |  |
| **Philips TIFF** | `.tiff` | ✅ |
| **Prairie Technologies TIFF** (Bruker Prairie View) | `.tif, .xml, .cfg` | ✅ `.xml` |
| **Princeton Instruments SPE** | `.spe` |  |
| **Quesant** | — |  |
| **QuickTime Movie** | — |  |
| **RHK** | `.sm2, .sm3` |  |
| **SBIG** | — |  |
| **ScanImage TIFF** | `.tif` | ✅ |
| **Seiko** | `.xqd, .xqf` |  |
| **SimplePCI & HCImage** | `.cxd` |  |
| **SM Camera** | — |  |
| **SPIDER** | — |  |
| **Targa** | `.tga` |  |
| **Tecan Spark Cyto Workspace** | `.db, .tif` |  |
| **Text** | `.txt` |  |
| **TIFF (Tagged Image File Format)** | `.tiff, .tif, .tf2, .tf8, .btf` | ✅ |
| **TillPhotonics TillVision** | `.vws` |  |
| **TissueGnostics TFCYTO** | `.aqproj, .tfcyto` |  |
| **Topometrix** | `.tfr, .ffr, .zfr, .zfp, .2fl` |  |
| **Trestle** | `.tif, .sld, .jpg` |  |
| **UBM** | `.pr3` |  |
| **Unisoku** | `.dat, .hdr` |  |
| **Varian FDF** | `.fdf` |  |
| **Vectra QPTIFF** | `.qptiff, .tif` | ✅ |
| **Veeco AFM** | — |  |
| **Ventana BIF** | `.bif` | ✅ |
| **VG SAM** | — |  |
| **VisiTech XYS** | — |  |
| **Volocity** | `.mvd2` |  |
| **Volocity Library Clipping** | — |  |
| **WA-TOP** | — |  |
| **Windows Bitmap** | `.bmp` | ✅ |
| **Zeiss Axio CSM** | `.lms` |  |
| **Zeiss AxioVision TIFF** | `.xml, .tif` |  |
| **Zeiss AxioVision ZVI** | `.zvi` | ✅ |
| **Zeiss CZI** | `.czi` | ✅ |
| **Zeiss LSM 510/710** | `.lsm` | ✅ |
| **Zeiss XRM** | `.txm, .txrm, .xrm` | ✅ |

### Format notes

**Olympus OMP2INFO (multi-area / tiling acquisitions).** An `.omp2info` file
lists the tile OIR files of a FLUOVIEW tiling acquisition and their grid
indices. Olympus stitches the tiles by image matching (phase correlation)
when the acquisition is saved; the matched tile positions are stored
nowhere, only the result `Stitch_<area>_<group>.oir`. scopeio therefore
returns:

- **series 0: the vendor stitch** `Stitch_<area>_<group>.oir`, if it is in
  the folder: exactly the mosaic Olympus produced;
- **then one series per tile**, in the order of the `.omp2info` file, named
  `tile x<i> y<j> (<file>)`, with the tile's nominal stage position (grid
  origin plus index times tile size less the overlap) as `stage_position`.

scopeio does not stitch tiles itself. Bio-Formats returns a single mosaic on
a grid of its own (466 × 462 px pitch for the public sample, 3308 × 2360
px), which matches neither the nominal grid (461 px) nor the vendor stitch
(3554 × 2540 px).

**Micro-Manager.** Any file of a dataset opens the whole acquisition: an
image stack file (`*_MMStack*.ome.tif`, also a continuation file `_1`,
`_2`, ... or the `_metadata.txt` companion), the `metadata.txt` or an
`img_*.tif` of a separate-files acquisition, or an NDTiff folder, its
`NDTiff.index` or a data file. Each stage position is one series (for
separate files: each `Pos<n>` folder). In NDTiff, every axis besides
`channel`, `z` and `time` (e.g. Magellan `row` / `column` tiles) also
selects the series; tiles are not stitched and the downsampled pyramid
folders are not read. NDTiff RGB images are not supported yet.

**BigDataViewer.** Setups that differ only in their `channel` attribute
form one series; every other attribute combination (angle, illumination,
tile) is a series of its own, with the time points as T and the mipmap
levels as pyramid levels. Registrations are not applied, so fused and
deconvolved setups are separate series too. Partitioned HDF5 and the N5 /
OME-Zarr loaders are not supported. Compressed chunks spanning several z
planes (BDV and Imaris) are decoded once and kept in a cache of at most
64 MiB per file when one plane's chunks of all channels fit in it, so a z
stack read plane by plane decodes each chunk once; a chunk leaves the
cache after its last plane is read, and the cache switches itself off when
chunks are evicted before they are reused. Uncompressed chunks are read one
plane at a time.

**Hamamatsu VMS.** Level 0 is the grid of JPEG files, level 1 the map
file (when it scales the image by a whole factor); the macro image is an
associated image. `PhysicalWidth` / `PhysicalHeight` are nanometres: the
public CMU-1 slide has 0.228 µm per pixel at 40x, Bio-Formats reports 228
µm. Bio-Formats' map series of that slide does not match the slide
(scopeio's map agrees with the block means of level 0). VMU slides (NGR
files) are not supported.

**Aperio AFI: not supported yet.** An `.afi` file is said to be XML
listing one Aperio SVS file per fluorescence channel (each SVS file opens
on its own as Aperio SVS), but its layout is not documented (the Aperio
data interchange document does not cover it) and no public sample file
could be found (searched: OME sample server, OpenSlide test data, Zenodo).
Support can be added once a sample with a clear licence is available.

**OME-XML.** Files with the pixels inline (`BinData`: base64, `none` or
`zlib` compression, either byte order; type `bit` as 0 / 1 bytes) are
read as OME-XML; files with `TiffData` are OME-TIFF companions. Empty
`BinData` elements and metadata-only images read as zeros. With several
`Pixels` per image (schema 2008-09) scopeio follows `DefaultPixels`; the
reference readers take the first. Both reference readers fail on the
synthetic zlib, big-endian and `bit` files.

**Zeiss Xradia XRM.** `.txm` (reconstructed volume), `.txrm` (projection
series) and `.xrm` (single image) return their images as z planes, rows as
stored; Bio-Formats returns every image flipped vertically (same pixels
otherwise). Projection angles are in the original metadata, stage
positions and exposures in the plane metadata. bioformats-rs reads none of
the test files.

**ICS.** ICS 1.0 (`.ics` + `.ids` or `.ids.gz`) and 2.0 (one file, or a
`source file`). Dimensions `x`, `y`, `z` and `t` keep their role; every
other dimension (`ch`, `p` and unnamed ones) adds channels, as both
reference readers do. With `layout coordinates cartesian` (origin at the
lower left, libics documentation) rows are returned from the top.
bioformats-rs returns big-endian samples unswapped.

**Imspector OBF.** Each stack is a series; axes 0 and 1 are x and y, a
further axis is z (label `Z` or a length unit), t (seconds) or channels.
Version 6 chunked stacks and short writes (missing samples read as zeros)
are supported. Not supported yet: `.msr` files in the older archive layout
(no OBF header, `CDataStack`). In the one public sample the 16-bit planes
follow the axis labels directly, but the rest of the layout is
undocumented; more samples are needed to read it reliably.

**IN Cell Analyzer.** Each (well, field) is a series with the waves as
channels; plate rows, columns and wells are in `plates()`. A wave imaged in
2-D within a z stack serves its plane at every z, as Bio-Formats does.
Older `.xdce` files that list no images (`number="0"`) are read from the
image file names.

**Becker & Hickl SDT.** Each data block is a series of the image size
with the TCSPC time bins as T (time increment = bin width), as Bio-Formats
reports them; Bio-Formats adds a second, empty channel per block.
ZIP-compressed blocks are inflated as a stream once per requested region,
keeping only that region's pixels with all their time bins, plane by
plane (a 64 x 64 crop of a 512 x 512 x 4096-bin block holds 33 MB, not the
2 GB block; further time bins of the region are served from it).
LZ4-compressed blocks are not supported yet.

**Gatan DigitalMicrograph.** DM3 and DM4: every `ImageList` image except
the thumbnails is a series (the reference readers return one image per
file). A third dimension is channels for an energy axis (spectrum images),
z planes for a length axis, else time points (`Meta Data.IsSequence`).
RGBA images keep their four samples. Not supported: complex, packed
complex and RGB (type 8) data, and DM5 (HDF5) files.

**Opera Flex.** The `.flex` files of a folder are the wells of a plate:
any of them opens all, one series per well and field (as Bio-Formats
does); channels are the (exposure, camera) pairs. No public specification
exists (own analysis of IDR files). Not supported: LuraWave-compressed
images and the `.mea` / `.res` measurement files.

**Yokogawa CV7000 / CV8000.** Open the plate file (`.wpi`) or
`MeasurementData.mlf` / `MeasurementDetail.mrf`. One series per well and
field; channels are the (action, channel) pairs of the image records. A
2-D channel next to a z stack (e.g. fluorescence with a brightfield stack)
repeats its plane at every z, as Bio-Formats does; at later time points
Bio-Formats repeats the first time point's plane, scopeio that time
point's own. No public specification exists (own analysis of IDR and
Cell Painting Gallery plates). bioformats-rs cannot open these plates.

**MetaXpress.** Open the `.HTD` file (in the plate folder or a
`TimePoint_<n>` folder). One series per selected well and site; channels
are the waves, time points and z steps the `TimePoint_<n>` / `ZStep_<n>`
folders (`ZStep_0` projections are skipped). Images are placed by their
file names (`<plate>_<well>_s<site>_w<wave>`), so a missing image reads as
zeros without shifting the others (Bio-Formats shifts the later planes).
Pixel size from the MetaMorph calibration of the TIFF files.

**Keller Lab Block.** One `.klb` file is one 5-D image (x, y, z,
channels, time) of bzip2-compressed or uncompressed blocks; a region read
decompresses only the blocks it touches (with a small block cache). Files
are opened one at a time: the reference readers group the files of a
time-lapse by their `TM…` / `CHN…` names, scopeio does not; within one
file they return only the first channel and time point. On a 4.4
gigapixel idr0044 stack scopeio needs 53 s CPU and 86 MB, Bio-Formats
306 s and 742 MB; bioformats-rs exceeds 8 GB. Not supported: 64-bit
integer samples.

**AVI.** Each video stream is a series with one time point per frame
(counted from the index, not `avih.dwTotalFrames`); audio is ignored.
Codecs from `strf.biCompression`: uncompressed 8-bit palette (gray
palettes as one sample, colour palettes as RGB), 16/24/32-bit (32-bit
alpha only when the first frame's alpha is not constant 0 or 255),
`Y800` / `GREY` gray, MSRLE (RLE8 / RLE4 with delta frames) and Motion
JPEG (frames without Huffman tables get the T.81 Annex K tables). OpenDML
files above 1 GB (`indx` / `ix##`, `RIFF AVIX`) are read in full; empty
chunks (dropped frames) repeat the previous frame. Uncompressed frames are
read row by row from the file; compressed frames are decoded whole and the
last one is cached. **Motion JPEG is lossy (typically 4:2:0 chroma
subsampling): do not use it for quantitative intensity analysis.**
Inter-frame codecs (H.264, MPEG-4, Cinepak, …) are rejected with a hint to
convert the file. The reference readers read only the first RIFF chunk of
OpenDML files and fail on several valid variants (see
`tests/expected/avi_divergences.tsv`).
On 1000 frames of 1024 x 1024 scopeio reads a 1 GB uncompressed OpenDML
file in 0.37 s CPU with 6 MiB RAM (Bio-Formats: 8.7 s and 906 MiB for
half of the frames) and Motion JPEG in 2.9 s with 17 MiB (Bio-Formats
12.7 s / 739 MiB, bioformats-rs 8.7 s / 6 GiB); see BENCHMARK.md.

**AmiraMesh.** Regular 2D / 3D `Lattice` fields of AmiraMesh and Avizo
files (`# AmiraMesh` / `# Avizo` headers): `BINARY` (big endian),
`BINARY-LITTLE-ENDIAN` and `ASCII` data, `HxZip` and `HxByteRLE`
compression, the types byte, sbyte, short, ushort, int, uint, float and
double. Vector fields (`float[3]`) have one channel per component; several
lattice fields of one type (e.g. `Labels` and `Probability` of a label
field) are the channels of one series, fields of other types further
series. Pixel size from the top-level `BoundingBox` of uniform lattices
(node positions: span over n - 1) or `BoundingBoxXY` of stacked ones, in
the unit of `Parameters { Units { Coordinates } }` (micrometres if none).
Raw data is read plane by plane from the file; compressed and ASCII data
is decoded once on first read. Surfaces and tetrahedral grids are not
images and are rejected. The reference readers read double and vector
fields as uint8, and Bio-Formats decodes HxByteRLE of single slices only
(see `tests/expected/amira_divergences.tsv`). A 227 MB tomogram reads in
0.08 s CPU with 7 MiB RAM (Bio-Formats: 2.3 s / 332 MiB; bioformats-rs
fails to open it).

**Hamamatsu DCIMG.** One camera stream (HCImage and other DCAM-API
applications) as one series whose frames are time points; both known
layouts (version 7 with a footer, version 0x1000000 with per-frame
trailers), 8- and 16-bit. Frames are read row band by row band straight
from the file, so memory does not grow with the number of frames. The camera's frame
counters and time stamps give the acquisition date (UTC), the time
increment and each frame's time; camera model, serial number and
sensor sub-array go to the original metadata. ORCA-Flash4.0 cameras
overwrite four pixels of the sensor's centre row with a marker and store
their real values separately: each frame's own values are put back.
Planes are returned bottom row first, as Bio-Formats and bioformats-rs
return them (the `dcimg` Python package returns the stored order).
Truncated files open with their complete frames. Each file is read on its
own: Bio-Formats groups all `.dcimg` files of a folder into one series.
Both reference readers put frame 0's stored values into every frame of
version-7 files and ignore their row padding; bioformats-rs also ignores
the frame trailers of version-0x1000000 files and does not restore their
stored pixels (see `tests/expected/dcimg_divergences.tsv`). A 1.6 GB
stream of 2000 frames reads in 0.20 s CPU with 6 MiB RAM (Bio-Formats:
7.8 s / 356 MiB, bioformats-rs: 2.3 s / 3.1 GiB); see BENCHMARK.md.

**Philips TIFF.** Whole-slide exports of the Philips IntelliSite
pathology solution (Software tag `Philips DP`, `DPUfsImport` XML in the
first ImageDescription). The TIFF tags give level sizes padded to whole
tiles; the reduced levels get their true size from the full-resolution
size and the level's `DICOM_PIXEL_SPACING`, as OpenSlide reports them.
Tiles that are not stored (sparse slides) read as white, the background
OpenSlide renders; Bio-Formats returns black. The label and macro images
come from the base64 JPEGs in the XML or from the `Label` / `Macro` TIFF
directories; the barcode, the acquisition date and all XML attributes
(`philips.*`) are in the metadata. Bio-Formats reads these files as plain
TIFF (full resolution only); its decoder upsamples the 4:2:0 JPEG chroma
differently (block means within 0.7, see
`tests/expected/philips_divergences.tsv`). A 2048 × 2048 crop reads in
0.03 s CPU with 19 MiB RAM (Bio-Formats: 3.3 s / 264 MiB).

**ScanImage TIFF.** Laser scanning recordings of ScanImage 3.x to
2023 (all five metadata layouts: `state.*`, `scanimage.SI4.*`, file
format 1 and 2, and the binary header of formats 3 and 4). The pages are
mapped to channels (saved channels), z and time from the acquisition
settings: planar recordings as time series, fast z stacks with the
slices of each volume as z and their flyback frames skipped, motor
stacks with the saved frames of each slice as time points. Files that
hold less than one volume (a stack stopped early, a recording split over
files) keep their complete slices; acquisitions stopped before the first
frame are rejected. Multi-ROI pages (mesoscopes) are split at the fly-to
lines into one series per region of interest, named after the ROI.
Pixel sizes from the TIFF resolution (written in centimetres by recent
releases), the imaging field of view or the ROI scan fields (calibrations
giving pixels of a millimetre or more are dropped); arbitrary z stacks
give each slice's depth, every page its frame time. Each file is read on
its own; a recording split over files is several readers. The placement
of every page is checked with the synthetic pixel values of the
CatalystNeuro test files. Bio-Formats and bioformats-rs read every page
as a time point of one channel; their pixels equal ours for single-channel
planar files.

**Bruker Prairie View.** Recordings of Prairie View 5.x (two-photon and
confocal Ultima / Investigator systems), opened through their XML file.
Z series map the frames of each cycle to z (bidirectional cycles put back
in depth order) and the cycles to time; timed and brightness-over-time
sequences map all frames to time points. Single- and multi-page TIFF files
(Prairie View 5.7 and later) are read lazily, with a few files kept open;
a missing file reads as zeros. Pixel size from `micronsPerPixel`, the z
step from the frame positions, each plane's time from `absoluteTime`,
channel names, objective and laser wavelength from the settings. The nine
CatalystNeuro test recordings Bio-Formats opens (it fails on the
brightness-over-time one) match its pixels and pixel sizes; with several timed sequences Bio-Formats maps the frames to
z, and it returns a series per cycle for bidirectional z series (see
`tests/expected/prairie_divergences.tsv`). bioformats-rs cannot open the
recordings.

**CEOS Panta Rhei.** A `.prz` file is a ZIP archive of NumPy arrays:
`data.npy` holds the image (2-D, or 3-D with the frames returned as time
points; any byte order, C or Fortran order) and `meta_data_json.npy` the
metadata as JSON (a list with one object per frame). Stored entries are
read in place, deflated ones decoded into memory on first read; ZIP64
archives are supported. The pixel size is `value * pixel_factor` of the
`device.calib` entries (x first; the unbinned specimen pixel times the
binning); acquisition time, exposure, stage position, microscope name,
camera (detector model) and acquisition session (`repo_id` as dataset) are
mapped, each frame of a stack gets its own time, exposure and stage position
(plane metadata, plus the frame interval when evenly spaced), and every
metadata key of every frame is kept in the original metadata. The pickled `meta_data.npy` / `data_model.npy` copies are never
read. Bio-Formats and bioformats-rs cannot open the format; RosettaSciIO
0.15.0 (run as a black box) returns the same pixels, shape, pixel type and
pixel sizes for all test files, and every original-metadata value it reports.
Differences: RosettaSciIO keeps only frame 0's metadata of a stack, stores
`device.calib` (without its trailing `null`) and `ref_size` reversed (y, x), takes the title
from `repo_id` (scopeio: the file name, which is unique per image) and maps
beam energy, convergence angle and magnification to structured fields
(scopeio: original metadata only, as `SeriesMeta` has no such fields); it
does not map acquisition time, exposure or stage position.

**NRRD.** Attached (`.nrrd`) and detached (`.nhdr`) headers with one data
file; raw, gzip, ASCII and hex encodings, line / byte skip (also `-1`).
Axis roles follow `kinds` (space / domain axes are x, y, z, `time` the
time points, others channels); without kinds a `none` space direction or
a NaN spacing marks a channel axis. The reference readers always read the
fastest axis as channels, so scalar volumes with `kinds` differ. Not
supported: bzip2, several data files (`LIST`, format strings), block and
64-bit integer samples.

**Nikon EZ-C1 TIFF: not supported.** No format documentation is published,
and no public sample file could be found (searched: OME sample server,
Zenodo, Figshare). The only implementation is Bio-Formats'
GPL-licensed reader, which this clean-room project must not consult. Files
written by the newer NIS-Elements software are read as Nikon NIS-Elements
TIFF. Support can be added once sample files with a clear licence are
available.

## License

scopeio is licensed under the GNU Affero General Public License version 3
only, with an additional permission for EVAnalyzer (see `LICENSE`,
`LICENSE-EXCEPTION`, `NOTICE`). Commercial licenses are available from the
copyright holder. Contributions require the [Contributor License
Agreement](CLA.md); see [CONTRIBUTING.md](CONTRIBUTING.md).

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

## Performance

CPU time and peak RAM of scopeio against Bio-Formats 8.5.0, bioformats-rs
0.1.8 and OpenSlide 3.4.1 on public sample files (Intel Core Ultra 9 185H,
median of 3 runs, each a fresh process). An "op" is one measured read: a
whole file, a crop or a pyramid level. Factors are how much more CPU or RAM
the other reader needs for the same op (geometric mean); "–" means not
compared, "n/a" means the reader cannot open the format.

| Format | Ops | scopeio CPU (median) | scopeio RAM (median) | Bio-Formats CPU / RAM | bioformats-rs CPU / RAM | OpenSlide CPU / RAM |
|---|---:|---:|---:|---:|---:|---:|
| TIFF | 80 | 1 ms | 2.8 MiB | 1090× / 42× | 3.0× / 4.5× | – |
| OME-TIFF | 96 | 1 ms | 2.8 MiB | 1231× / 46× | 4.0× / 4.8× | – |
| OME-Zarr | 14 | 1 ms | 2.9 MiB | n/a | 3.5× / 7.2× | – |
| Olympus VSI | 6 | 18 ms | 11 MiB | 89× / 26× | 1.7× / 2.4× | – |
| Nikon ND2 | 11 | 45 ms | 5.3 MiB | 122× / 51× | 2.7× / 5.9× | – |
| Zeiss CZI | 35 | 7 ms | 5.8 MiB | 494× / 57× | 2.2× / 3.5× | – |
| Leica LIF | 12 | 7 ms | 4.3 MiB | 1021× / 128× | 0.9× / 8.1× | – |
| Aperio SVS | 13 | 227 ms | 22 MiB | 6.0× / 2.4× | 13.8× / 2.1× | – |
| Hamamatsu NDPI | 20 | 86 ms | 43 MiB | 14.5× / 4.7× | 4.3× / 2.9× | – |
| 3DHistech MRXS | 13 | 7 ms | 6.8 MiB | n/a | n/a | 6.1× / 2.3× |
| Zeiss LSM | 6 | 10 ms | 4.7 MiB | 156× / 33× | 3.8× / 7.1× | – |
| Akoya QPTIFF | 9 | 128 ms | 17 MiB | 6.2× / 6.6× | 1.2× / 6.7× | – |
| Leica SCN | 7 | 225 ms | 32 MiB | 19× / 5.6× | – | – |
| Ventana BIF | 10 | 1.07 s | 25 MiB | 4.3× / 2.3× | – | 1.0× / 2.9× |
| Imaris IMS | 4 | 56 ms | 6.8 MiB | 115× / 42× | 3.3× / 18× | – |
| Windows BMP | 17 | 1 ms | 2.5 MiB | 757× / 37× | 2.0× / 4.9× | – |
| PNG | 12 | 1 ms | 3.6 MiB | 374× / 22× | 2.3× / 3.1× | – |
| JPEG | 3 | 1 ms | 3.4 MiB | 228× / 16× | 1.9× / 2.5× | – |
| JPEG 2000 | 3 | 1 ms | 3.7 MiB | 147× / 10× | 2.2× / 2.0× | – |
| Olympus OIB/OIF | 3 | 1 ms | 3.7 MiB | 244× / 64× | 7.0× / 4.3× | – |
| Olympus OIR | 15 | 18 ms | 11 MiB | 244× / 51× | 2.8× / 6.1× | – |
| DICOM | 17 | 55 ms | 18 MiB | 38× / 10× | 2.6× / 2.5× | – |
| Zeiss ZVI | 14 | 8 ms | 6.7 MiB | 230× / 33× | 3.2× / 5.0× | – |
| Leica LOF/XLEF | 65 | 49 ms | 6.6 MiB | 122× / 50× | 2.5× / 14× | – |
| MetaMorph STK/ND | 4 | 23 ms | 5.0 MiB | 228× / 66× | 4.1× / 15× | – |
| DeltaVision | 7 | 2 ms | 4.2 MiB | 398× / 62× | 2.6× / 9.2× | – |

- Bio-Formats' large factors on small files are mostly JVM start-up (about
  8 s CPU and 550 MB per process); on whole-slide formats it needs 4–19×
  the CPU and 2–7× the RAM.
- The reference readers also write the pixels to disk (ICS output);
  scopeio only decodes.
- bioformats-rs failed on 86 of the ops that scopeio read.
- Large plain images (24-megapixel RGB): PNG 0.31 s / 72 MiB (Bio-Formats
  1.95 s / 354 MiB, bioformats-rs 0.57 s / 216 MiB), JPEG 0.29 s / 145 MiB
  (3.89 s / 601 MiB, 0.50 s / 220 MiB), lossless JPEG 2000 4.9 s / 577 MiB
  (14.5 s / 716 MiB, 4.6 s / 401 MiB).
- Peaks above the medians (up to a few GB) come from reading a whole
  gigapixel level into one output buffer, not from the reader itself.

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
| **Amira Mesh** | `.am, .amiramesh, .grey, .hx, .labels` |  |
| **Amnis FlowSight** | `.cif` |  |
| **Analyze 7.5** | `.img, .hdr` |  |
| **Andor Bio-Imaging Division (ABD) TIFF** | `.tif` |  |
| **Andor SIF** | `.sif` |  |
| **Animated PNG** | `.png` |  |
| **Aperio AFI** | `.afi, .svs` |  |
| **Aperio SVS TIFF** | `.svs` | ✅ |
| **Applied Precision CellWorX** | `.htd, .pnl` |  |
| **AVI (Audio Video Interleave)** | `.avi` |  |
| **Axon Raw Format** | `.arf` |  |
| **BD Pathway** | `.exp, .tif` |  |
| **BDV** | `.xml, .h5` |  |
| **Becker & Hickl SPC FIFO** | `.spc` |  |
| **Becker & Hickl SPCImage** | `.sdt` |  |
| **Bio-Rad Gel** | `.1sc` |  |
| **Bio-Rad PIC** | `.pic, .raw, .xml` |  |
| **Bio-Rad SCN** | `.scn` |  |
| **BioTek Cytation** | `.tif, .tiff` |  |
| **Bitplane Imaris** | `.ims` | ✅ |
| **Bruker MRI** | — |  |
| **Burleigh** | — |  |
| **Canon DNG** | `.cr2, .crw` |  |
| **CellH5** | `.ch5` |  |
| **Cellomics** | — |  |
| **CellVoyager** | `.xml, .tif` |  |
| **CV7000** | `.wpi, .tif` |  |
| **DeltaVision** | `.dv, .r3d` | ✅ |
| **DICOM** (incl. whole-slide imaging) | `.dcm, .dicom` | ✅ |
| **ECAT7** | `.v` |  |
| **EPS (Encapsulated PostScript)** | `.eps, .epsi, .ps` |  |
| **Evotec/PerkinElmer Opera Flex** | `.flex, .mea, .res` |  |
| **FEI** | `.img` |  |
| **FEI TIFF** | `.tiff` |  |
| **FITS (Flexible Image Transport System)** | `.fits` |  |
| **Gatan Digital Micrograph** | `.dm3, .dm4` |  |
| **Gatan Digital Micrograph 2** | `.dm2` |  |
| **GIF (Graphics Interchange Format)** | `.gif` |  |
| **Hamamatsu Aquacosmos NAF** | `.naf` |  |
| **Hamamatsu DCIMG** | — |  |
| **Hamamatsu HIS** | — |  |
| **Hamamatsu ndpi** | `.ndpi, .ndpis` | ✅ |
| **Hamamatsu VMS** | `.vms` |  |
| **Hitachi S-4800** | `.txt, .tif, .bmp, .jpg` |  |
| **I2I** | `.i2i` |  |
| **ICS (Image Cytometry Standard)** | `.ics, .ids` |  |
| **Imacon** | `.fff` |  |
| **ImagePro Sequence** | — |  |
| **ImagePro Workspace** | — |  |
| **IMAGIC** | — |  |
| **IMOD** | `.mod` |  |
| **Improvision Openlab LIFF** | — |  |
| **Improvision Openlab Raw** | — |  |
| **Improvision TIFF** | `.tif` |  |
| **Imspector OBF** | `.obf, .msr` |  |
| **InCell 1000/2000** | `.xdce, .tif` |  |
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
| **Keller Lab Block** | — |  |
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
| **MetaXpress** | — |  |
| **MIAS (Maia Scientific)** | `.tif` |  |
| **Micro-Manager** | `.tif, .txt, .xml` |  |
| **Mikroscan TIFF** | `.tif, .tiff` |  |
| **MINC MRI** | `.mnc` |  |
| **Minolta MRW** | — |  |
| **MNG (Multiple-image Network Graphics)** | `.mng` |  |
| **Molecular Imaging** | `.stp` |  |
| **MRC (Medical Research Council)** | `.mrc, .st, .ali, .map, .rec, .mrcs` |  |
| **NEF (Nikon Electronic Format)** | `.nef, .tif` |  |
| **NIfTI** | `.img, .hdr, .nii, .nii.gz` |  |
| **Nikon Elements TIFF** | `.tiff` |  |
| **Nikon EZ-C1 TIFF** | `.tiff` |  |
| **Nikon NIS-Elements ND2** | `.nd2` | ✅ |
| **NRRD (Nearly Raw Raster Data)** | `.nrrd, .nhdr, .raw, .txt` |  |
| **Olympus CellR/APL** | `.apl, .mtb, .tnb, .tif, .obsep` |  |
| **Olympus FluoView FV1000** | `.oib, .oif` | ✅ |
| **Olympus FluoView TIFF** | `.tif` |  |
| **Olympus OIR** | `.oir` | ✅ |
| **Olympus OMP2INFO** | `.omp2info` |  |
| **Olympus ScanR** | `.xml, .dat, .tif` |  |
| **Olympus SIS TIFF** | `.tiff` |  |
| **Olympus/Evident VSI** | `.vsi, .ets` | ✅ |
| **OME-TIFF** | `.ome.tiff, .ome.tif, .ome.tf2, .ome.tf8, .ome.btf` | ✅ |
| **OME-XML** | `.ome, .ome.xml` |  |
| **OME-Zarr (NGFF 0.4 / 0.5)** | `.zarr` | ✅ |
| **OMERO Pyramid** | — |  |
| **Oxford Instruments** | `.top` |  |
| **PCORAW** | `.pcoraw, .rec` |  |
| **PCX (PC Paintbrush)** | `.pcx` |  |
| **Perkin Elmer Densitometer** | `.pds` |  |
| **PerkinElmer Columbus** | — |  |
| **PerkinElmer Nuance** | `.im3` |  |
| **PerkinElmer Operetta** | `.tiff, .xml` |  |
| **PerkinElmer UltraVIEW** | — |  |
| **PerkinElmer Vectra/QPTIFF** | `.tif, .tiff, .qptiff` | ✅ |
| **Photoshop TIFF** | `.tif, .tiff` |  |
| **PicoQuant Bin** | `.bin` |  |
| **PICT (Macintosh Picture)** | `.pict` |  |
| **PNG (Portable Network Graphics)** | `.png` | ✅ |
| **Portable Any Map** | `.pbm, .pgm, .ppm` |  |
| **Prairie Technologies TIFF** | `.tif, .xml, .cfg` |  |
| **Princeton Instruments SPE** | `.spe` |  |
| **Quesant** | — |  |
| **QuickTime Movie** | — |  |
| **RHK** | `.sm2, .sm3` |  |
| **SBIG** | — |  |
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
| **Zeiss XRM** | `.txm, .txrm` |  |

## License

scopeio is licensed under the GNU Affero General Public License version 3
only, with an additional permission for EVAnalyzer (see `LICENSE`,
`LICENSE-EXCEPTION`, `NOTICE`). Commercial licenses are available from the
copyright holder. Contributions require the [Contributor License
Agreement](CLA.md); see [CONTRIBUTING.md](CONTRIBUTING.md).

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
| **DeltaVision** | `.dv, .r3d` |  |
| **DICOM** | `.dcm, .dicom` |  |
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
| **JPEG** | `.jpg` |  |
| **JPEG 2000** | `.jp2, .j2k, .jpf` |  |
| **JPK** | — |  |
| **JPX** | — |  |
| **Keller Lab Block** | — |  |
| **Khoros VIFF Bitmap** | — |  |
| **Kodak BIP** | — |  |
| **Lambert Instruments FLIM** | — |  |
| **LaVision Imspector** | — |  |
| **Leica LAS AF LIF** | `.lif` | ✅ |
| **Leica LCS LEI** | — |  |
| **Leica LOF** | — |  |
| **Leica SCN** | `.scn` | ✅ |
| **Leica XLEF** | — |  |
| **LEO** | `.sxm, .tif, .tiff` |  |
| **Li-Cor L2D** | `.l2d, .tif, .scn` |  |
| **LIM (Laboratory Imaging/Nikon)** | `.lim` |  |
| **MetaMorph 7.5 TIFF** | `.tiff` |  |
| **MetaMorph Stack (STK)** | `.stk, .nd` |  |
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
| **Olympus FluoView FV1000** | `.oib, .oif` |  |
| **Olympus FluoView TIFF** | `.tif` |  |
| **Olympus OIR** | `.oir` |  |
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
| **PNG (Portable Network Graphics)** | `.png` |  |
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
| **Zeiss AxioVision ZVI** | `.zvi` |  |
| **Zeiss CZI** | `.czi` | ✅ |
| **Zeiss LSM 510/710** | `.lsm` | ✅ |
| **Zeiss XRM** | `.txm, .txrm` |  |

## License

scopeio is licensed under the GNU Affero General Public License version 3
only, with an additional permission for EVAnalyzer (see `LICENSE`,
`LICENSE-EXCEPTION`, `NOTICE`). Commercial licenses are available from the
copyright holder. Contributions require the [Contributor License
Agreement](CLA.md); see [CONTRIBUTING.md](CONTRIBUTING.md).

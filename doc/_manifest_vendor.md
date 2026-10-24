# Vendor microscope / camera format documentation: manifest

Collected 2026-10-24 for the scopeio clean-room reader. All paths are relative to `doc/`.
Nothing here was downloaded from Bio-Formats/OME (no source, javadoc or format pages). No source-code
repositories or tarballs were downloaded; only individual documentation files (HTML/PDF/MD/RST/DOC/TXT) plus
the license files that cover them.

Total size of this group: about 28.6 MB (the bh TCSPC handbook is 18 MB of that).

## Downloaded documents

| Format | File (relative path) | Source URL | Publisher | License/terms (as stated, or "unknown") | Notes |
|---|---|---|---|---|---|
| Nikon ND2 | nikon-nd2/openreadout-format-nd2.md | https://github.com/openreadout/openreadout/blob/HEAD/docs/formats/nd2.md | OpenReadout project (community) | MIT OR Apache-2.0 (see _openreadout/) | Community clean-room description. See the community section below |
| Nikon ND2 | nikon-nd2/openreadout-provenance-nd2.md | https://github.com/openreadout/openreadout/blob/HEAD/docs/provenance/nd2.md | OpenReadout project | MIT OR Apache-2.0 | Provenance log: says what was derived from what |
| Nikon ND2 | nikon-nd2/nd2-rs-DATASTRUCTURE.md | https://github.com/keejkrej/nd2-rs/blob/HEAD/DATASTRUCTURE.md | keejkrej/nd2-rs (community) | MIT (nikon-nd2/LICENSE-nd2-rs-MIT.txt) | Chunk map, CLX Lite, ImageDataSeq. **Ported from the `nd2` Python package (BSD-3)**. Has Rust code snippets |
| Nikon ND2 | nikon-nd2/LICENSE-nd2-rs-MIT.txt | https://github.com/keejkrej/nd2-rs/blob/HEAD/LICENSE-MIT | keejkrej | MIT | License for the file above |
| Nikon ND2 | nikon-nd2/nis-elements-supported-file-formats.html | https://www.nisoftware.net/NikonSaleApplication/Help/Docs-D/eng_d/image.formats.html | Nikon / Laboratory Imaging | unknown (vendor help page) | User-level overview only, no byte layout |
| Leica LIF / LOF / XLEF / XLIF | leica-lif/openreadout-format-lif.md | https://github.com/openreadout/openreadout/blob/HEAD/docs/formats/lif.md | OpenReadout | MIT OR Apache-2.0 | Covers LIF plus LOF/XLEF/XLIF (community) |
| Leica LIF | leica-lif/openreadout-provenance-lif.md | .../docs/provenance/lif.md | OpenReadout | MIT OR Apache-2.0 | Says it was derived from liffile (BSD-3) docs and source plus sample files |
| Leica LCS LEI | leica-lcs-lei/unc-leica-sp2-lei-layout.html | http://microscopy.unc.edu/how-to/leicasp2/default.html | UNC Michael Hooker Microscopy Facility | unknown | User-level: an LEI series is a folder of TIFFs plus an .lei database plus a .txt copy |
| Leica LCS raw (.info) | leica-lcs-lei/goldensoftware-leica-confocal-INFO-description.html | https://surferhelp.goldensoftware.com/subsys/subsys_leica_confocal_scanning_file_descr.htm | Golden Software (Surfer help) | unknown | Leica confocal native raw + .INFO description (older LCS-era format) |
| Olympus VSI/ETS | olympus-vsi/bioio-vsi-format.md | https://github.com/szarma/bioio-vsi/blob/main/docs/format.md | bioio-vsi (community) | BSD-3-Clause (olympus-vsi/LICENSE-bioio-vsi.txt) | Reverse-engineered from files. Some layout taken from slideio's BSD-3 VSI driver. States "Bio-Formats source was not read" |
| Olympus VSI/ETS | olympus-vsi/bioio-vsi-samples.md | https://github.com/szarma/bioio-vsi/blob/main/docs/samples.md | bioio-vsi | BSD-3-Clause | Sample files the notes were derived from |
| Olympus VSI/ETS | olympus-vsi/LICENSE-bioio-vsi.txt | https://github.com/szarma/bioio-vsi/blob/main/LICENSE | bioio-vsi | BSD-3-Clause | |
| Olympus VSI/ETS | olympus-vsi/openreadout-format-vsi.md, olympus-vsi/openreadout-provenance-vsi.md | .../docs/formats/vsi.md, .../docs/provenance/vsi.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: slideio (BSD-3) and tifffile (BSD-3) |
| Olympus VSI | olympus-vsi/svi-huygens-OlympusVSI.html | https://svi.nl/OlympusVSI | Scientific Volume Imaging | unknown | User-level notes on ETS compression variants and folder layout |
| Olympus OIF/OIB | olympus-oif-oib/openreadout-format-oif.md, olympus-oif-oib/openreadout-provenance-oif.md | .../docs/formats/oif.md, .../docs/provenance/oif.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: oiffile (BSD-3) |
| Olympus OIR | olympus-oir/openreadout-format-oir.md, olympus-oir/openreadout-provenance-oir.md | .../docs/formats/oir.md, .../docs/provenance/oir.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: oirfile (BSD-3) |
| Olympus ScanR | olympus-scanr/scanR-Analysis-Software-Manual-v2.1.0.pdf | https://www.ntnu.edu/documents/10351683/21256833/scan%5ER_Analysis_Software_Manual_v2.1.0.pdf | Olympus (hosted by NTNU) | unknown (vendor manual, third-party mirror) | Describes the folder layout: Data/*.tif (16-bit), experiment_descriptor.xml, Acquisitionlog.dat. No XML schema |
| Zeiss CZI | zeiss-czi/libczi-docs-*.rst / *.md (9 files) + zeiss-czi/images/*.png | https://github.com/ZEISS/libczi/tree/main/docs/source | Carl Zeiss Microscopy GmbH | **MIT** for docs/* per REUSE dep5 (zeiss-czi/LICENSE-libczi-reuse-dep5.txt). The library code is LGPL-3.0 | Official ZEISS docs: coordinate systems, image-document concept, zstd "chunked" compression, resolution protocol, valid-pixel mask, stream objects, reader compatibility. Not the full ZISRAW spec |
| Zeiss CZI | zeiss-czi/LICENSE-libczi-reuse-dep5.txt, zeiss-czi/LICENSE-libczi-COPYING.txt | https://github.com/ZEISS/libczi/blob/main/.reuse/dep5, .../COPYING | ZEISS | REUSE mapping / LGPL notice | Kept so the docs' license is on record |
| Zeiss CZI | zeiss-czi/zeiss-czi-image-file-format-page.html | https://www.zeiss.com/microscopy/en/products/software/zeiss-zen/czi-image-file-format.html | ZEISS | unknown (vendor web page) | Says the full CZI format documentation is provided **on request** through a contact form |
| Zeiss CZI | zeiss-czi/openreadout-format-czi.md, zeiss-czi/openreadout-provenance-czi.md | .../docs/formats/czi.md, .../docs/provenance/czi.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: czifile (BSD-3). libCZI/pylibCZIrw used only as black-box oracles under its policy |
| Zeiss AxioVision ZVI | zeiss-zvi/openreadout-format-zvi.md, zeiss-zvi/openreadout-provenance-zvi.md | .../docs/formats/zvi.md, .../docs/provenance/zvi.md | OpenReadout | MIT OR Apache-2.0 | Community. OLE2 container (olefile, BSD) plus sample files |
| Zeiss LSM (5xx) | zeiss-lsm/Lsmfile.doc | https://web.archive.org/web/20090410172215/http://ibb.gsf.de:80/homepage/karsten.rodenacker/IDL/Lsmfile.doc | Apparently Carl Zeiss ("File Format Description - LSM 5xx, Release 2.0"), rehosted by K. Rodenacker (GSF) | **unknown**. No copyright or confidential marking found in the text strings. It is a vendor document on a third-party site | Old (LSM 510 era) but covers CZ_LSMINFO and the .mdb tables. WaveMetrics links to it as the "unofficial" LSM doc |
| Zeiss XRM/TXM/TXRM | zeiss-xrm-txrm/loc-fdd000581-TXM.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000581.shtml | Library of Congress | US Government work (public domain) | High-level: OLE2 compound file. No stream-level layout |
| Bitplane Imaris IMS | bitplane-imaris-ims/imaris-file-format.html | https://imaris.oxinst.com/support/imaris-file-format | Oxford Instruments / Bitplane | unknown. Bitplane announced the IMS format as "open source" in 2015 | **Official spec** (HDF5 groups DataSet/DataSetInfo/Thumbnail, attributes) |
| Bitplane Imaris IMS | bitplane-imaris-ims/arxiv-2008.10311-ImarisWriter.pdf | https://arxiv.org/abs/2008.10311 | Beati et al. (Bitplane), arXiv | arXiv default license (non-exclusive distribution) | ImarisWriter paper. Describes the blockwise multi-resolution IMS layout |
| Bitplane Imaris IMS | bitplane-imaris-ims/aicjanelia-understanding-imaris-5.5-file-format.html | https://knowledge.aicjanelia.org/posts/20200416-understanding-the-imaris-file-format/ | AIC Janelia (community) | unknown | Explanatory write-up |
| Bitplane Imaris IMS | bitplane-imaris-ims/openreadout-format-ims.md, ...-provenance-ims.md | .../docs/formats/ims.md, .../docs/provenance/ims.md | OpenReadout | MIT OR Apache-2.0 | Community |
| Imspector OBF/MSR | imspector-obf/imspector-docs-fileformat.html | https://imspectordocs.readthedocs.io/en/latest/fileformat.html | MPI for Biophysical Chemistry & Abberior Instruments | "(c) Copyright 2016 MPI-BPC & Abberior Instruments". No license given | **Official spec** of OBF v1+ and the MSR wrapper |
| PicoQuant PTU/PHU (and the record formats) | picoquant-ptu/ReadMe.html, apx_support.html, apx_tagdictionary.html, apx_tagtypes.html, apx_tttrrecords.html, pq.png, linkicon.png, Readme.txt | https://github.com/PicoQuant/PicoQuant-Time-Tagged-File-Format-Demos/tree/master/doc | PicoQuant GmbH | No license file in the repo. Readme has a warranty disclaimer only. Published so that "users write their own file reading programs" | **Official spec**: tag dictionary, tag types, TTTR record layouts (T2/T3 for PicoHarp, HydraHarp, TimeHarp, MultiHarp). No PT3/legacy BIN document was found |
| Princeton Instruments SPE 2.x / 3.0 | princeton-spe/SPE-3.0-File-Format-Specification.pdf | https://raw.githubusercontent.com/hududed/pyControl/master/Manual/LightField/Add-in%20and%20Automation%20SDK/SPE%203.0%20File%20Format%20Specification.pdf (original: ftp://ftp.princetoninstruments.com/Public/Manuals/Princeton%20Instruments/SPE%203.0%20File%20Format%20Specification.pdf, which did not respond) | Teledyne Princeton Instruments | "(c) 2012-2019 Teledyne Princeton Instruments, All rights reserved". Publicly published by the vendor on its FTP site. This copy is a third-party mirror (from the LightField SDK docs folder) | **Official spec**, Issue 3 (2019). Appendix A has the full SPE 2.x 4100-byte header |
| Becker & Hickl SDT/SPC | becker-hickl-sdt/bh-TCSPC-Handbook-8th-edition.pdf | https://www.usherbrooke.ca/chimie/fileadmin/sites/chimie/documents/Services/TCSPC/hb-bh-TCSPC-1.pdf | Becker & Hickl GmbH (W. Becker), hosted by Université de Sherbrooke | (c) Becker & Hickl. **Caveat:** bh now puts the current (11th) edition behind a free registration form. This 8th-edition copy is a public university mirror with no login | Has the .sdt structure (file header, file info, setup, measurement description blocks, data blocks). 968 pages, 18 MB. If you want strict "no registration-gated material", delete this file |
| Gatan DM2/DM3/DM4 | gatan-dm/cbb-ntu-dmformat-index.html | https://personal.ntu.edu.sg/cbb/info/dmformat/index.html | Chris Boothroyd (community) | unknown | Reverse-engineered from files. Covers DM2, DM3 and DM4, and notes DM5 is HDF5 |
| Gatan DM3 | gatan-dm/imagej-DM3Format.gj.html | https://imagej.net/ij/plugins/DM3Format.gj.html | Greg Jefferis (community), hosted by ImageJ | unknown | Reverse-engineered DM3 tag structure |
| Gatan DM | gatan-dm/openreadout-format-dm.md, ...-provenance-dm.md | .../docs/formats/dm.md, .../docs/provenance/dm.md | OpenReadout | MIT OR Apache-2.0 | Community |
| DeltaVision DV / Priism IVE | deltavision-dv/priism-IVE4-IM_ref2-archived.html | https://web.archive.org/web/2018/http://msg.ucsf.edu/IVE/IVE4_HTML/IM_ref2.html | UCSF Sedat/Agard lab (Priism/IVE) | unknown | **Primary format reference** (IM library reference guide: 1024-byte header, extended header, image data). The live site sits behind a Cloudflare JS challenge, so the archive.org copy was used |
| DeltaVision DV | deltavision-dv/openreadout-format-deltavision.md, ...-provenance-deltavision.md | .../docs/formats/deltavision.md, .../docs/provenance/deltavision.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: tlambert03/mrc (BSD-3) |
| Hamamatsu DCIMG | hamamatsu-dcimg/openreadout-format-dcimg.md, ...-provenance-dcimg.md | .../docs/formats/dcimg.md, .../docs/provenance/dcimg.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: lens-biophotonics/dcimg (MIT). The official DCIMG-SDK comes with the DCAM-API and was not used |
| Andor SIF | andor-sif/openreadout-format-andor-sif.md, ...-provenance-andor-sif.md | .../docs/formats/andor-sif.md, .../docs/provenance/andor-sif.md | OpenReadout | MIT OR Apache-2.0 | Community. Prior art: sif_parser and sifreader (MIT / BSD-3) |
| Bio-Rad Image Lab SCN | biorad-scn/openreadout-format-biorad-scn.md, ...-provenance-biorad-scn.md | .../docs/formats/biorad-scn.md, .../docs/provenance/biorad-scn.md | OpenReadout | MIT OR Apache-2.0 | Community |
| Bio-Rad PIC | biorad-pic/mipav-image-formats-descriptions.html | https://mipav.cit.nih.gov/pubwiki/index.php/Image_formats_descriptions | NIH CIT (MIPAV) | US Government work (likely public domain) | Short Bio-Rad PIC overview (76-byte header, images, notes). Also short LSM and LIFF notes. Not byte-level |
| PCO B16 / pcoraw | pco-raw/pco-camware-users-manual-2004.pdf | https://7id.xray.aps.anl.gov/internal/hardware/detector/ccd/sensicam_QE_double_shutter/pro_camware_12_2004.pdf | PCO / The Cooke Corporation (hosted by Argonne APS) | unknown (vendor manual, third-party mirror) | §8.1 "Image File Formats" documents the B16 header ("PCO-", basic and extended header). pcoraw (BigTIFF-based) is not documented here |
| SBIG | sbig/SBIG-Type3-Image-File-Format.pdf | https://web.archive.org/web/20140512212712/http://www.physics.csbsju.edu/370/photometry/manuals/SBIG.file.format.pdf | Santa Barbara Instrument Group (hosted by CSB/SJU) | unknown (vendor application note) | **Official spec** of SBIG Type 3 (2048-byte ASCII header, compressed and uncompressed) |
| Improvision Openlab RAW | improvision-openlab/openlab-raw-technote-344-archived.html | https://web.archive.org/web/20080323171226/http://www.improvision.com/support/tech_notes/detail.php?id=344 | Improvision | unknown | **Official tech note** for Openlab Raw ('OLRW' header, 'rIMG' blocks). LIFF itself is not covered |
| Amersham / Molecular Dynamics GEL | amersham-md-gel/eumenidae-blog-amersham-gel-scaling.html | https://eumenidae.blogspot.com/2013/07/amershams-gel-files-scaling-issues.html | Blog author (community) | unknown | Describes MD_FileTag (33445) and MD_ScalePixel (33446) square-root scaling. GEL is TIFF plus private tags |
| 3i SlideBook SLDY | 3i-slidebook/intelligent-imaging-slidebook-page.html | https://www.intelligent-imaging.com/slidebook | Intelligent Imaging Innovations | unknown (vendor web page) | Vendor description of .sldy (directory of NumPy .npy arrays + YAML metadata) and .sldyz (Zstandard). Not a byte-level spec |
| (shared) | _openreadout/clean-room-policy.md, _openreadout/LICENSE-MIT, _openreadout/LICENSE-APACHE | https://github.com/openreadout/openreadout/tree/HEAD/docs/legal and the repo root | OpenReadout | MIT OR Apache-2.0 | License and clean-room policy that cover all openreadout-*.md files above |

## Not found / not freely available

| Format | Status | URL / note |
|---|---|---|
| Nikon ND2 | No public spec. The official ND2 SDK needs an account at nd2sdk.com (the site was in maintenance) | https://www.nd2sdk.com/ |
| Leica LIF / LOF / XLEF / XLLF | Vendor spec "Leica Image File Formats - LIF, XLEF, XLLF, LOF" v3.2 (2016) plus "Annotations ... LAS X 3.x" v1.4 and "TSC SP8 FALCON File Format Description" exist but have no public download. Ask Leica support | (none) |
| Leica LCS LEI | Vendor spec not public | (none) |
| Olympus/Evident VSI/ETS | Vendor spec (v1.3/v1.6) not public | contact Evident |
| Olympus OIF/OIB, OIR | Vendor specs not public | contact Evident |
| Olympus CellR/APL, OMP2INFO | Nothing found | |
| Olympus ScanR | No XML schema. Only the user manual (downloaded) | |
| Zeiss CZI (full ZISRAW spec) | Available **on request** from ZEISS via contact form. A copy marked "CONFIDENTIAL" is online at physics.hmc.edu; it was **deliberately not downloaded or opened** | https://www.zeiss.com/microscopy/en/products/software/zeiss-zen/czi-image-file-format.html ; (do not use) http://www.physics.hmc.edu/~gerbode/wppriv/wp-content/uploads/2012/06/DS_ZISRAW-FileFormat_Rel_ZEN2011.pdf |
| Zeiss ZVI | Vendor spec (v2.0.x) not public | |
| Zeiss LSM (current v5.5/v6.0 spec) | Not public. Only the old Lsmfile.doc (downloaded) | |
| Zeiss Axio CSM LMS | Nothing found | |
| Zeiss XRM/TXM/TXRM | No spec. Only community readers (source, not downloaded) | |
| Andor SIF | No vendor spec | |
| 3i SlideBook SLD (legacy) / SLDY detail | Vendor spec not public | |
| Hamamatsu DCIMG | DCIMG-SDK is shipped with the DCAM-API installer (not downloaded: SDK headers/binaries) | https://www.hamamatsu.com/eu/en/product/cameras/software/driver-software.html |
| Hamamatsu HIS, Aquacosmos NAF | Nothing found | |
| Becker & Hickl SDT (current docs) | 11th-edition handbook and SPCM-DLL description both sit behind a registration form | https://www.becker-hickl.com/literature/documents/flim/the-bh-tcspc-handbook-11th-edition-2026/ ; https://www.becker-hickl.com/literature/documents/dll-descriptions/spcm-dll/ |
| Gatan DM5 | Gatan has a DM5 documentation page, but it is behind a bot checkpoint (Vercel) and returned HTTP 429. Not bypassed | https://www.gatan.com/dm5-documentation |
| PicoQuant PT3 / legacy BIN | Not in the PicoQuant doc folder. Older format demos exist only as source code | |
| Lambert Instruments FLIM (.fli) | No public spec | |
| PCO pcoraw | No public spec (BigTIFF-based, per vendor manuals) | |
| LaVision Imspector, PerkinElmer UltraVIEW, PerkinElmer Nuance IM3, Volocity MVD2 (Metakit DB), Improvision Openlab LIFF, IPLab, IVision, SimplePCI/HCImage CXD, TillVision VWS, VisiTech XYS, I2I, Imacon, ImagePro seq/workspace, SM Camera, Bio-Rad Gel 1SC, Li-Cor L2D, Molecular Imaging STP, Amnis FlowSight CIF, Hitachi S-4800, LEO, FEI img, JEOL, Axon Raw | No public spec found | Amnis IDEAS user manuals describe .rif/.cif/.daf only at user level (not downloaded) |
| Bio-Rad PIC (original "Introductory Edited Version 1.0", 1993) | Not found online | |
| Molecular Dynamics GEL (original spec) | Not found online. Only the blog post (downloaded) | |
| Library of Congress | Has an entry only for TXM (downloaded). No entries found for the other formats in this group | |

## Reverse-engineered / community-written documents (review before use)

| Document(s) | Author | How it was derived (as stated by the author) | Risk flag |
|---|---|---|---|
| `*/openreadout-format-*.md` and `*/openreadout-provenance-*.md` (ND2, LIF, VSI, OIF/OIB, OIR, CZI, ZVI, IMS, DCIMG, DV, DM, SIF, SCN) | OpenReadout Authors (2026), MIT OR Apache-2.0 | Written under a clean-room policy (`_openreadout/clean-room-policy.md`): built from sample files, hex dumps and **source code/docs of permissively licensed (BSD-3/MIT) readers**, mainly Christoph Gohlke's liffile, czifile, oiffile, oirfile and tifffile, Talley Lambert's `nd2` and `mrc`, slideio, lens-biophotonics/dcimg, sif_parser and olefile. Under that policy, **GPL/LGPL readers (Bio-Formats, libCZI, readlif, ...) were only run as black-box oracles**, and their source was not read. The provenance files often quote Bio-Formats *output* (e.g. "Bio-Formats reports ...") for comparison | Low to medium. Not derived from Bio-Formats source. They are derived from BSD/MIT reader source, which is permissive (keep attribution if text is reused). They do mention Bio-Formats behaviour, which counts as oracle exposure |
| `nikon-nd2/nd2-rs-DATASTRUCTURE.md` | keejkrej (nd2-rs), MIT | **Explicitly aligned with / ported from the `nd2` Python package source (tlambert03/nd2, BSD-3)**: `load_experiment`, `_parse_xy_pos_loop`, `json_from_clx_lite_variant`. Has Rust code snippets. Calls CLX "reverse-engineered" | Medium. Derived from another reader's source (BSD-3, permissive, not Bio-Formats) |
| `olympus-vsi/bioio-vsi-format.md`, `bioio-vsi-samples.md` | szarma (bioio-vsi), BSD-3 | Worked out from files. Some layout/names "taken from slideio's BSD-3 VSI driver" (tagged [sio]). Results checked against Bio-Formats 8.3.0 *output* ([=bf]). States "Bio-Formats source was not read" | Low to medium. Partly derived from slideio source (BSD-3) |
| `gatan-dm/cbb-ntu-dmformat-index.html` | Chris Boothroyd | Obtained by examining file structure. Partly based on Greg Jefferis' page | Low. No other reader's source is cited |
| `gatan-dm/imagej-DM3Format.gj.html` | Greg Jefferis | Reverse-engineered DM3 description | Low |
| `amersham-md-gel/eumenidae-blog-amersham-gel-scaling.html` | Blog author | Says it is based on reading the MD GEL specs | Low |
| `bitplane-imaris-ims/aicjanelia-understanding-imaris-5.5-file-format.html` | AIC Janelia | Explanation of the official Bitplane document | Low |
| `zeiss-lsm/Lsmfile.doc` | Appears to be a Zeiss vendor document, rehosted by a third party | Vendor-authored, not reverse-engineered. Distribution terms unknown | **Unclear license/permission** |
| `becker-hickl-sdt/bh-TCSPC-Handbook-8th-edition.pdf` | Becker & Hickl (vendor) | Vendor-authored. Third-party university mirror. The vendor now registration-gates newer editions | **Unclear**. Delete if registration-gated vendor material should be avoided |
| `princeton-spe/SPE-3.0-File-Format-Specification.pdf` | Teledyne Princeton Instruments (vendor) | Vendor-authored and published publicly on the vendor FTP. This copy came from a GitHub mirror because the FTP was unreachable | Low ("All rights reserved", but public) |

Note on exposure: web-search result snippets during collection sometimes summarised Bio-Formats format pages
(e.g. which spec versions Bio-Formats holds). No Bio-Formats page, source or javadoc was opened or saved.

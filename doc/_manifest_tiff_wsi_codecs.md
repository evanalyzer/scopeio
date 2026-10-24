# Manifest: TIFF family, whole-slide formats, compression codecs

Collected 2026-10-24 for the scopeio clean-room reader. Paths are relative to `doc/`.
Clean-room rules: nothing was fetched from Bio-Formats (github.com/ome/bioformats, docs.openmicroscopy.org/bio-formats, bio-formats.readthedocs.io), and no reader/library source code was downloaded. Every PDF was checked to be a real PDF with the expected page count. Every HTML page was checked to make sure it is not an error page.

## Downloaded documents

| Format | File (relative path) | Source URL | Publisher | License/terms (as stated, or "unknown") | Notes |
|---|---|---|---|---|---|
| TIFF 6.0 | tiff/TIFF6.0_specification.pdf | https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf | Adobe / Aldus (mirrored by ITU-T) | Adobe copyright; the spec allows free use for implementing TIFF | 121 pages. Same content as Adobe's TIFF6.pdf. Covers LZW (sect. 13), PackBits (sect. 9), the old JPEG section (22), Predictor and tiles |
| TIFF Supplement 1 (PageMaker 6) | tiff/Adobe_PageMaker6_TIFF_Technical_Notes.pdf | https://web.archive.org/web/2010id_/http://partners.adobe.com/public/developer/en/tiff/TIFFPM6.pdf | Adobe | Adobe copyright, freely published | SubIFDs, clarifications, JPEG-in-TIFF notes |
| TIFF Tech Note 2 (new JPEG-in-TIFF, Compression=7) | tiff/TIFF_TechNote2_original_draft_1995.txt | https://web.archive.org/web/2012id_/http://www.remotesensing.org/libtiff/TIFFTechNote2.html | Tom Lane / TIFF Advisory Committee (draft, 17 Mar 1995) | unknown (freely circulated draft) | Plain-text original |
| TIFF Tech Note 2 | tiff/TIFF_TechNote2_JPEG_in_TIFF_libtiff.html | https://libtiff.gitlab.io/libtiff/specification/technote2.html | LibTIFF project | LibTIFF docs (c) LibTIFF contributors (BSD-like libtiff license) | Same note, HTML rendering |
| TIFF Tech Note 3 (floating point, Predictor=3) | tiff/Adobe_TIFF_TechNote3_floating_point_draft1.pdf | http://chriscox.org/TIFFTN3d1.pdf | Adobe (Chris Cox, Photoshop team), draft 1, 2005 | unknown (freely published) | Floating-point predictor and 16/24-bit float |
| TIFF (index) | tiff/LoC_fdd000022_TIFF6.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000022.shtml | US Library of Congress | US government work (public domain) | Format description and references |
| TIFF (generic pyramidal TIFF) | tiff/openslide_generic_tiff.html | https://openslide.org/formats/generic-tiff/ | OpenSlide project | CC BY-SA 4.0 | Community documentation |
| BigTIFF | bigtiff/BigTIFF_libtiff_spec.html | https://libtiff.gitlab.io/libtiff/specification/bigtiff.html | LibTIFF project (BigTIFF design by Steve Carlsen / AWare Systems et al.) | (c) LibTIFF contributors | The awaresystems.be original is 404 now |
| BigTIFF (index) | bigtiff/LoC_fdd000328_BigTIFF.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000328.shtml | US Library of Congress | Public domain | |
| Photoshop TIFF | photoshop-tiff/Adobe_Photoshop_TIFF_Technical_Notes.pdf | https://web.archive.org/web/2010id_/http://partners.adobe.com/public/developer/en/tiff/TIFFphotoshop.pdf | Adobe | Adobe copyright, freely published | Deflate (Compression 8/32946), ImageSourceData, Photoshop tags |
| Photoshop TIFF / PSD resources | photoshop-tiff/Adobe_Photoshop_File_Formats_Specification.html | https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/ | Adobe | Adobe copyright; published for third parties to read/write the format | Image Resource Blocks (TIFF tag 34377) and layer data (tag 37724) |
| OME-TIFF | ome-tiff/OME-TIFF_specification.html | https://ome-model.readthedocs.io/en/stable/ome-tiff/specification.html | Open Microscopy Environment | (c) 2000-2025 OME; no license stated on the page (the ome-model repo is BSD-2) | Main spec |
| OME-TIFF | ome-tiff/OME-TIFF_file_structure.html | https://ome-model.readthedocs.io/en/stable/ome-tiff/file-structure.html | OME | as above | |
| OME-TIFF | ome-tiff/OME-TIFF_index.html | https://ome-model.readthedocs.io/en/stable/ome-tiff/index.html | OME | as above | |
| OME-TIFF | ome-tiff/OME-TIFF_sample_data.html | https://ome-model.readthedocs.io/en/stable/ome-tiff/data.html | OME | as above | Lists public sample files (useful for tests) |
| OME-XML | ome-xml/ome_2016-06.xsd | https://www.openmicroscopy.org/Schemas/OME/2016-06/ome.xsd | OME | CC BY 3.0 Unported (stated in the file header), (c) 2002-2016 OME | Current schema. It imports only xml.xsd. BinData is inline |
| OME-XML | ome-xml/OME-2016-06_schema_documentation_ome_xsd.html | https://www.openmicroscopy.org/Schemas/Documentation/Generated/OME-2016-06/ome_xsd.html | OME | as above | Generated element reference, 6.8 MB single page |
| OME-XML | ome-xml/OME-XML_overview.html, ome-xml/ome-model_*.html (13 pages) | https://ome-model.readthedocs.io/en/stable/ (ome-xml/index, developers/*, schemas/*, specifications/*) | OME | (c) OME | Model overview, 6D/7D/8D storage (modulo), compression, units, SPW, annotations, schema versions, minimum/compliant specs |
| OME-Zarr / NGFF | ome-zarr/OME-NGFF_0.5.html | https://ngff.openmicroscopy.org/specifications/0.5/index.html | NGFF community (OME) | (c) 2020-2025 NGFF Community; no license stated on the page | Current spec (Zarr v3) |
| OME-Zarr / NGFF | ome-zarr/OME-NGFF_0.4.html | https://ngff.openmicroscopy.org/0.4/ | NGFF community | as above | Zarr v2-based version, common in the wild |
| Zarr (needed by OME-Zarr) | ome-zarr/zarr_v3_core_spec.html | https://zarr-specs.readthedocs.io/en/latest/v3/core/index.html | Zarr core dev team | CC BY 3.0 Unported | |
| Zarr (needed by OME-Zarr) | ome-zarr/zarr_v2_spec.html | https://zarr-specs.readthedocs.io/en/latest/v2/v2.0.html | Zarr core dev team | CC BY 3.0 (zarr-specs) | |
| Zarr v3 codecs | ome-zarr/zarr_v3_codec_{sharding-indexed,blosc,bytes,gzip,crc32c,transpose}.html | https://zarr-specs.readthedocs.io/en/latest/v3/codecs/<name>/index.html | Zarr core dev team | CC BY 3.0 Unported | Downloaded 2026-10-24; the zstd codec page does not exist (404) |
| numcodecs Blosc | ome-zarr/zarr_v2_numcodecs_blosc.html | https://numcodecs.readthedocs.io/en/stable/compression/blosc.html | numcodecs (Zarr) | MIT (numcodecs docs) | Downloaded 2026-10-24 |
| OMERO pyramid | omero-pyramid/OMERO_pyramid_ome-model.html | https://ome-model.readthedocs.io/en/stable/omero-pyramid/index.html | OME | (c) OME | Short: a TIFF with one JPEG 2000 stream per plane using decomposition levels |
| ImageJ TIFF / hyperstacks | imagej-tiff/ImageJ_UserGuide_1.46r_ch8_stacks_hyperstacks.html | https://imagej.net/ij/docs/guide/146-8.html | T. Ferreira & W. Rasband (ImageJ User Guide) | unknown (freely published) | Describes the hyperstack model (C/Z/T), not the byte format |
| ImageJ TIFF | imagej-tiff/ijtiff_The_ImageJ_Problem.html | https://docs.ropensci.org/ijtiff/articles/the-imagej-problem.html | Rory Nolan (rOpenSci ijtiff) | ijtiff package GPL-3 (article license not stated) | Community write-up of ImageDescription "ImageJ=..." conventions |
| Aperio SVS | aperio-svs/Aperio_Digital_Slides_and_Third-Party_Data_Interchange_2008.pdf | https://web.archive.org/web/20100920094815id_/http://aperio.com/documents/api/Aperio_Digital_Slides_and_Third-party_data_interchange.pdf | Aperio Technologies (now Leica Biosystems) | Vendor copyright, published publicly | Official vendor doc (MAN-0069). 35 pages |
| Aperio SVS | aperio-svs/openslide_aperio_format.html | https://openslide.org/formats/aperio/ | OpenSlide | CC BY-SA 4.0 | Community notes |
| Hamamatsu NDPI + VMS/VMU | hamamatsu-ndpi-vms/openslide_hamamatsu_format.html | https://openslide.org/formats/hamamatsu/ | OpenSlide | CC BY-SA 4.0 | Reverse-engineered, see section 3 |
| Leica SCN | leica-scn/openslide_leica_format.html | https://openslide.org/formats/leica/ | OpenSlide | CC BY-SA 4.0 | Community notes |
| Ventana BIF | ventana-bif/Roche_Digital_Pathology_BIF_Whitepaper.pdf | https://diagnostics.roche.com/content/dam/diagnostics/Blueprint/en/pdf/rmd/Roche-Digital-Pathology-BIF-Whitepaper.pdf | Roche Diagnostics | Vendor copyright; Roche says it makes the BIF spec "publicly available" | Official. Covers DP 200 files only (not iScan Coreo/HT) |
| Ventana BIF | ventana-bif/openslide_ventana_format.html | https://openslide.org/formats/ventana/ | OpenSlide | CC BY-SA 4.0 | Covers older iScan files |
| Trestle | trestle/openslide_trestle_format.html | https://openslide.org/formats/trestle/ | OpenSlide | CC BY-SA 4.0 | Community notes |
| PerkinElmer/Akoya Vectra QPTIFF | perkinelmer-qptiff/Akoya_PhenoImager_HT_Polaris_UserManual_1.0.7_AppendixA_QPTIFF_Spec.pdf | https://www.zmb.uzh.ch/dam/jcr:dd0daeda-a84c-46a1-8542-f0a65b4dc976/PolarisUserManual_1_0_7_rev0.pdf | Akoya Biosciences (hosted by Univ. Zurich ZMB) | Vendor copyright; no license stated | Full user manual (110 pages). **Appendix A "Akoya QPTIFF Specification" (pp. 83-92) is the spec.** The rest is a user guide and EULA |
| MetaMorph STK / TIFF | metamorph-stk/MetaMorph_STK_format_t10243.pdf | https://web.archive.org/web/20111221001857id_/http://support.meta.moleculardevices.com/docs/t10243.pdf | Molecular Devices | Vendor copyright | **Pointer only (1 page).** It links to ftp://ftp.meta.moleculardevices.com/support/stack/STK.doc and "Meta Series TIFF Format.doc". The FTP server is gone and the files are not in the Wayback Machine. See section 2 |
| Micro-Manager formats | micro-manager/Micro-Manager_File_Formats.html | https://micro-manager.org/Micro-Manager_File_Formats | Micro-Manager project (H. Pinkard et al.) | (c) µManager; no explicit license on page | Overview of the image-stack (OME-TIFF), single-file and NDTiff formats |
| Micro-Manager NDTiff | micro-manager/NDTiff_format_README.md | https://raw.githubusercontent.com/micro-manager/NDTiffStorage/main/README.md | Micro-Manager / H. Pinkard | Repo BSD-3-Clause | README only: "Specification for NDTiff v3.x" plus the deprecated v1 spec (header, IFDs, index map, display settings). Contains small usage snippets |
| DNG | dng/Adobe_DNG_Specification_1.7.1.0.pdf | https://web.archive.org/web/2024id_/https://helpx.adobe.com/content/dam/help/en/photoshop/pdf/DNG_Spec_1_7_1_0.pdf | Adobe | Adobe copyright. Adobe offers a royalty-free DNG patent license (per Adobe; see the spec front matter) | 126 pages. The direct Adobe URL also works with a plain curl user agent |
| Canon CRW (CIFF) | canon-crw-cr2/Canon_CIFF_spec_V1R04.pdf | http://xyrion.org/ciff/CIFFspecV1R04.pdf | Canon (CIFF spec v1.0 rev 4, 1997), mirrored by xyrion.org | Canon copyright; mirrored copy, redistribution terms unclear | Official CIFF image-data spec |
| Canon CRW | canon-crw-cr2/exiftool_canon_raw_CRW.html | https://exiftool.org/canon_raw.html | Phil Harvey (ExifTool) | ExifTool docs (Perl Artistic/GPL as part of the distribution); unclear for docs alone | Community / reverse-engineered |
| Canon CR2 | canon-crw-cr2/lclevy_CR2_poster_explained.html | http://lclevy.free.fr/cr2/ | Laurent Clévy | **CC BY-NC-SA 4.0** (non-commercial) | Reverse-engineered |
| Nikon NEF | nikon-nef/exiftool_Nikon_tags.html | https://exiftool.org/TagNames/Nikon.html | Phil Harvey (ExifTool) | as ExifTool above | Tag reference for the Nikon maker note only. Does not cover NEF compression |
| Olympus VSI/ETS | olympus-vsi/bioio-vsi-format.md, olympus-vsi/bioio-vsi-samples.md, olympus-vsi/LICENSE-bioio-vsi.txt | https://raw.githubusercontent.com/szarma/bioio-vsi/main/docs/format.md (and samples.md) | Srdjan Sarikas (bioio-vsi, created 2026-10-16) | BSD-3-Clause | **These files were already in doc/olympus-vsi/, placed by another collector. My identical copies were removed.** Reverse-engineered; see section 3 for provenance caveats |
| Olympus VSI/ETS | olympus-vsi/openreadout-format-vsi.md, olympus-vsi/openreadout-provenance-vsi.md | (placed by another collector; see doc/_openreadout) | OpenReadout (Richard Zimring with Claude) | see doc/_openreadout/LICENSE-* (MIT/Apache) | Not fetched by me. Listed here because VSI is high priority. Includes a provenance log |
| JPEG (baseline/progressive/lossless) | codecs/jpeg/ITU-T_T.81_JPEG.pdf | https://www.w3.org/Graphics/JPEG/itu-t81.pdf | ITU-T (CCITT) Rec. T.81 (1992) | ITU-T Recs are free to download | 186 pages. Includes lossless JPEG (process 14) |
| JFIF | codecs/jpeg/ITU-T_T.871_2011-05_JFIF.pdf | https://www.itu.int/rec/T-REC-T.871-201105-I/en | ITU-T Rec. T.871 | Free ITU-T download | Official JFIF |
| JFIF 1.02 | codecs/jpeg/JFIF_1.02.pdf | https://www.w3.org/Graphics/JPEG/jfif3.pdf | C-Cube (Eric Hamilton), 1992 | unknown (freely circulated) | Original |
| EXIF | codecs/exif/CIPA_DC-008-2023_Exif_3.0.pdf | https://www.cipa.jp/std/documents/download_e.html?DC-008-Translation-2023-E | CIPA / JEITA | Free after accepting a disclaimer on the page (no login) | Exif 3.0 English translation |
| JPEG 2000 Part 1 (+ JP2, Annex I) | codecs/jpeg2000/ITU-T_T.800_2019-06_JPEG2000_Part1.pdf | https://www.itu.int/rec/T-REC-T.800-201906-S/en | ITU-T Rec. T.800 (06/2019, superseded) | Free ITU-T download | The 07/2024 edition (T.800-202407-I) needs an ITU TIES login, so I used the 2019 edition |
| JPEG 2000 Part 2 (+ JPX, Annex M) | codecs/jpeg2000/ITU-T_T.801_2021-06_JPEG2000_Part2_JPX.pdf | https://www.itu.int/rec/T-REC-T.801-202106-S/en | ITU-T Rec. T.801 (06/2021, superseded) | Free | The 2023-08 edition needs a TIES login |
| HTJ2K (JPEG 2000 Part 15) | codecs/jpeg2000/ITU-T_T.814_2019-06_HTJ2K.pdf | https://www.itu.int/rec/T-REC-T.814-201906-I/en | ITU-T Rec. T.814 | Free | Extra: used by newer WSI and DICOM data |
| JPEG XR | codecs/jpeg-xr/ITU-T_T.832_2019-06_JPEG-XR.pdf | https://www.itu.int/rec/T-REC-T.832-201906-I/en | ITU-T Rec. T.832 | Free | Includes the container format (Annex A) |
| JPEG-LS | codecs/jpeg-ls/ITU-T_T.87_1998-06_JPEG-LS.pdf | https://www.itu.int/rec/T-REC-T.87-199806-I/en (ZPF zip) | ITU-T Rec. T.87 | Free | Extracted from the ITU zip. The reference software in that zip was discarded |
| Deflate / zlib / gzip | codecs/deflate/rfc1950_zlib.txt, rfc1951_deflate.txt, rfc1952_gzip.txt | https://www.rfc-editor.org/rfc/rfc1950.txt (1951, 1952) | IETF (P. Deutsch, J-L. Gailly) | RFC: freely redistributable unmodified | |
| Zstandard | codecs/zstd/rfc8878_zstandard.txt | https://www.rfc-editor.org/rfc/rfc8878.txt | IETF | IETF Trust / BSD for code components | |
| LZ4 | codecs/lz4/lz4_Frame_format.md, codecs/lz4/lz4_Block_format.md | https://raw.githubusercontent.com/lz4/lz4/dev/doc/ | Yann Collet | lz4 repo: BSD-2-Clause (lib); doc license not separately stated | Spec documents only |
| Blosc 1 | codecs/blosc/c-blosc1_README_CHUNK_FORMAT.rst | https://raw.githubusercontent.com/Blosc/c-blosc/main/README_CHUNK_FORMAT.rst | Blosc dev team | BSD-3-Clause (repo) | Chunk header format |
| Blosc 2 | codecs/blosc/c-blosc2_README_CHUNK_FORMAT.rst, _CFRAME_FORMAT.rst, _SFRAME_FORMAT.rst | https://raw.githubusercontent.com/Blosc/c-blosc2/main/ | Blosc dev team | BSD-3-Clause (repo) | Chunk, contiguous frame and sparse frame formats |
| WebP | codecs/webp/rfc9649_webp.txt | https://www.rfc-editor.org/rfc/rfc9649.txt | IETF (Google authors) | RFC | Container, lossless and VP8L. The lossy VP8 bitstream is in RFC 6386 (not downloaded) |
| PackBits | codecs/packbits/Apple_TN1023_Understanding_PackBits.html | https://web.archive.org/web/20080705155158id_/http://developer.apple.com/technotes/tn/tn1023.html | Apple | Apple copyright | Also defined in TIFF 6.0, section 9 |
| LZW (TIFF) | tiff/TIFF6.0_specification.pdf (section 13) | (see TIFF 6.0) | Adobe | | No separate document needed |
| JPEG XL | codecs/jpeg-xl/libjxl_format_overview.md | https://raw.githubusercontent.com/libjxl/libjxl/main/doc/format_overview.md | libjxl authors (Google / Cloudinary) | BSD-3-Clause | Informative overview only. The normative spec is ISO/IEC 18181 (paywalled) |
| JPEG XL | codecs/jpeg-xl/JPEG_XL_whitepaper.pdf | https://ds.jpeg.org/whitepapers/jpeg-xl-whitepaper.pdf (via web.archive.org) | JPEG committee | unknown | Informative overview, complete (3.0 MB) |

## 2. Not found / not freely available

| Format | Status | URL / details |
|---|---|---|
| ISO/IEC 18181 (JPEG XL) | Paywalled ISO standard | https://www.iso.org/standard/85066.html |
| ITU-T T.800 (07/2024), T.801 (08/2023) | Latest editions need an ITU TIES login | https://www.itu.int/rec/T-REC-T.800-202407-I ; https://www.itu.int/rec/T-REC-T.801-202308-I . The older free editions were downloaded instead |
| ITU-T T.870 (JPEG-LS Part 2) | Only offered as a software zip | https://www.itu.int/rec/T-REC-T.870-200203-I . Skipped |
| Aperio AFI | No public spec found | AFI is an XML file listing per-channel SVS files. The Aperio 2008 PDF does not describe it |
| Hamamatsu NDPI / VMS official | Vendor SDK (NDP.SDK) only under agreement or registration | Use the OpenSlide community docs |
| Leica SCN official | No public spec found. The XML namespace is undocumented | Use the OpenSlide community docs |
| Mikroscan TIFF | No public spec found | |
| Zeiss LSM 510/710 (CZ_LSMINFO) | Zeiss "Image File Format Description LSM 5/7 Release 6.0" exists but is distributed by Zeiss on request; no official public copy found | Note: doc/zeiss-lsm/Lsmfile.doc was placed by another collector. I did not fetch or check it, so its provenance and license should be verified |
| MetaMorph STK / Meta Series TIFF / ND | The original Molecular Devices docs (STK.doc, "Meta Series TIFF Format.doc") were on ftp.meta.moleculardevices.com, which is gone. The current knowledge-base article (https://support.moleculardevices.com/s/article/MetaMorph-Software-TIFF-and-STK-file-formats) is a JavaScript-rendered Salesforce page whose Word attachment I could not reach without a browser | Possibly freely available through that page in a real browser |
| Olympus FluoView TIFF (MM_HEADER/MM_STAMP) | Vendor spec not public | |
| Olympus SIS TIFF | No public spec found | |
| Olympus VSI/ETS official | Vendor spec (v1.3/1.6) not public | Community docs only (see above) |
| Prairie TIFF (Bruker PrairieView) | No public spec found | |
| Andor ABD TIFF / Improvision TIFF / Nikon Elements TIFF / Nikon EZ-C1 TIFF | No public specs found | |
| FEI TIFF (tags 34680 FEI_SFEG / 34682 FEI_HELIOS, INI text) | No vendor spec found | |
| Ionpath MIBI(tiff) | Vendor spec not public | ionpath/mibilib is source code and was not downloaded |
| Nikon NEF (compression and Huffman tables) | No official spec. Only the ExifTool tag reference was fetched | |
| Canon CR2 official | Not published by Canon. Community doc only | |
| ImageJ TIFF byte-level conventions | No formal spec. Only the ImageJ source (public domain) defines it, and source was excluded by the rules. The User Guide and the ijtiff write-up were fetched | |
| Aperio original 2005 "Aperio_File_Format_and_Third_Party_Data_Interchange.pdf" | Older revision on the Wayback Machine (www.aperio.com/Documents/…, 2005-2006). Not fetched; the 2008 revision supersedes it | |
| Akoya Phenochart User Guide 2.2.0 (also has a QPTIFF appendix) | HTTP 403 from quanterix.com for non-browser clients | https://www.quanterix.com/wp-content/uploads/2026/01/Phenochart-User-Guide-for-version-2.2.0.pdf |

## 3. Reverse-engineered / community-written documents

| File | Author | Notes |
|---|---|---|
| aperio-svs/openslide_aperio_format.html, hamamatsu-ndpi-vms/openslide_hamamatsu_format.html, leica-scn/openslide_leica_format.html, ventana-bif/openslide_ventana_format.html, trestle/openslide_trestle_format.html, tiff/openslide_generic_tiff.html | OpenSlide project (Carnegie Mellon; Goode, Gilbert, Satyanarayanan et al.) | CC BY-SA 4.0 documentation. OpenSlide code is LGPL-2.1; only the website docs were taken. The Hamamatsu, Leica and Trestle pages are largely reverse-engineered |
| canon-crw-cr2/lclevy_CR2_poster_explained.html | Laurent Clévy | Reverse-engineered. **CC BY-NC-SA 4.0**: the non-commercial clause may matter |
| canon-crw-cr2/exiftool_canon_raw_CRW.html, nikon-nef/exiftool_Nikon_tags.html | Phil Harvey (ExifTool) | Reverse-engineered maker-note and tag documentation. ExifTool is Perl Artistic/GPL; the license of the HTML docs is not stated separately |
| imagej-tiff/ijtiff_The_ImageJ_Problem.html | Rory Nolan (rOpenSci) | Community explanation of the ImageJ TIFF conventions |
| olympus-vsi/bioio-vsi-format.md, olympus-vsi/bioio-vsi-samples.md | Srdjan Sarikas (bioio-vsi, BSD-3, Oct 2026) | Reverse-engineered from files. The doc says some layout and names were "taken from slideio's BSD-3 VSI driver" and that values were compared against Bio-Formats 8.3.0 **output** as a black-box oracle. It states "Bio-Formats source was not read". The author also read the Bio-Formats format page. **Clean-room review advised before relying on it** |
| olympus-vsi/openreadout-*.md | OpenReadout (Richard Zimring with Claude) | Not collected by me. It says it was derived by hex dump plus black-box comparison with Bio-Formats 8.5.0, and it has a provenance log |
| micro-manager/NDTiff_format_README.md | Henry Pinkard (Micro-Manager) | Written by the format author, so a first-party spec, though informal |

# Manifest: generic raster/video, high-content screening, scanning probe formats

Collected 2026-10-24 for the scopeio clean-room effort. Paths are relative to `doc/`.
Nothing here was taken from Bio-Formats (no source, docs, javadoc or format pages were opened).
Licenses are given as the documents state them, or "unknown".

## Raster and video formats

| Format | File (relative path) | Source URL | Publisher | License/terms | Notes |
|---|---|---|---|---|---|
| PNG | png/png-3rd-edition-w3c.html | https://www.w3.org/TR/png-3/ | W3C | W3C Document License | PNG 3rd edition; includes APNG chunks (acTL/fcTL/fdAT), cICP, mDCV, cLLI |
| PNG | png/png-2nd-edition-w3c-rec-20031110.html | https://www.w3.org/TR/2003/REC-PNG-20031110/ | W3C / ISO 15948 | W3C Document License | 2nd edition, for reference |
| PNG | png/loc-fdd000153.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000153.shtml | Library of Congress | US Gov work (public domain) | Format description |
| APNG | apng/apng-mozilla-wiki.html | https://wiki.mozilla.org/APNG_Specification | Mozilla | MozillaWiki content license (see page footer) | Original APNG 1.0 spec; now superseded by PNG 3rd ed. |
| MNG | mng/mng-1.0-spec.html | http://www.libpng.org/pub/mng/spec/mng-1.0-20010209-pdg.html | PNG Development Group | "Distribution of this memo is unlimited" | MNG 1.0 (2001-02-09) |
| GIF87a | gif/gif87a-spec.txt | https://www.w3.org/Graphics/GIF/spec-gif87.txt | CompuServe (hosted by W3C) | Copyright CompuServe; royalty-free for use as stated in the text | |
| GIF89a | gif/gif89a-spec.txt | https://www.w3.org/Graphics/GIF/spec-gif89a.txt | CompuServe (hosted by W3C) | Copyright CompuServe; same terms as above | |
| GIF | gif/loc-fdd000133.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000133.shtml | Library of Congress | public domain | |
| BMP/DIB | bmp/ms-bitmap-storage.html | https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-storage | Microsoft Learn | Microsoft Learn terms of use | File layout overview (`file` says "JavaScript source" because the page starts with a script; it is HTML) |
| BMP/DIB | bmp/ms-bitmap-header-types.html | https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-header-types | Microsoft Learn | MS Learn ToU | |
| BMP/DIB | bmp/ms-bitmapfileheader.html, ms-bitmapcoreheader.html, ms-bitmapinfoheader.html, ms-bitmapv4header.html, ms-bitmapv5header.html, ms-rgbquad.html | https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-{bitmapfileheader,bitmapcoreheader,bitmapinfoheader,bitmapv4header,bitmapv5header,rgbquad} | Microsoft Learn | MS Learn ToU | Struct references |
| BMP/DIB | bmp/ms-bitmap-compression.html | https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-compression | Microsoft Learn | MS Learn ToU | RLE4/RLE8 |
| BMP | bmp/loc-fdd000189.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000189.shtml | Library of Congress | public domain | BMP v5 |
| PCX | pcx/zsoft-pcx-tech-ref.txt | http://bespin.org/~qz/pc-gpe/pcx.txt | ZSoft Corp. (PC-GPE mirror) | Copyright 1985-1991 ZSoft; no license stated | ZSoft's own technical reference. Includes ZSoft's own sample C routines as part of the spec |
| TGA | tga/truevision-tga-2.0-spec.pdf | https://raw.githubusercontent.com/DEAKSoftware/Truevision-TGA/master/truevision-tga.pdf | Truevision Inc. (PDF made from the original PostScript by D. Deák) | Copyright 1989-1991 Truevision; no license stated | Technical Manual v2.2, Jan 1991, 31 pp. Only the PDF was downloaded from GitHub, no code |
| TGA | tga/truevision-tga-2.0-spec-readme-deaksoftware.md | https://raw.githubusercontent.com/DEAKSoftware/Truevision-TGA/master/readme.md | Dominik Deák | unknown | Provenance of the PDF above |
| TGA | tga/loc-fdd000180.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000180.shtml | Library of Congress | public domain | |
| Netpbm PBM/PGM/PPM/PNM/PAM/PFM | netpbm/{pbm,pgm,ppm,pnm,pam,pfm}.html | https://netpbm.sourceforge.net/doc/{pbm,pgm,ppm,pnm,pam,pfm}.html | Netpbm project | unknown (no license on pages) | Format man pages only, no code |
| PICT | pict/imaging-with-quickdraw.pdf | https://developer.apple.com/library/archive/documentation/mac/pdf/ImagingWithQuickDraw.pdf | Apple Computer | Copyright Apple 1994; no license stated | Inside Macintosh: Imaging With QuickDraw, full book. Appendix A has the picture opcodes; the "Pictures" chapter covers the PICT layout |
| PSD/PSB | photoshop-tiff/Adobe_Photoshop_File_Formats_Specification.html (shared copy) | https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/ | Adobe | Adobe copyright; published "for 3rd parties to read and write" the format | Same document as photoshop-tiff/Adobe_Photoshop_File_Formats_Specification.html (fetched by another agent) |
| PSD | psd/loc-fdd000523.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000523.shtml | Library of Congress | public domain | |
| AVI | avi/ms-avi-riff-file-reference.html | https://learn.microsoft.com/en-us/windows/win32/directshow/avi-riff-file-reference | Microsoft Learn | MS Learn ToU | Main AVI RIFF layout |
| AVI | avi/ms-aviriff-{avimainheader,avistreamheader,avioldindex,avisuperindex,avistdindex,avimetaindex}.html | https://learn.microsoft.com/en-us/previous-versions/windows/desktop/api/aviriff/ns-aviriff-* | Microsoft Learn | MS Learn ToU | Struct references, including the OpenDML index structs |
| AVI | avi/ms-dv-data-in-avi.html | https://learn.microsoft.com/en-us/windows/win32/directshow/dv-data-in-the-avi-file-format | Microsoft Learn | MS Learn ToU | |
| AVI 2.0 / OpenDML | avi/opendml-avi-2.0.pdf | http://www.jmcgowan.com/odmlff2.pdf | OpenDML AVI M-JPEG File Format Subcommittee (hosted by John McGowan) | unknown | v1.02, 1996; 42 pp |
| AVI | avi/loc-fdd000059.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000059.shtml | Library of Congress | public domain | |
| RIFF | riff/microsoft-ibm-multimedia-data-spec-1.0-riffmci.pdf | https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/Docs/riffmci.pdf | IBM & Microsoft (McGill mirror) | unknown | "Multimedia Programming Interface and Data Specifications 1.0", Aug 1991; the original RIFF definition |
| RIFF | riff/ms-resource-interchange-file-format.html | https://learn.microsoft.com/en-us/windows/win32/xaudio2/resource-interchange-file-format--riff- | Microsoft Learn | MS Learn ToU | Short overview |
| QuickTime (QTFF) | quicktime/qtff-apple.pdf | https://developer.apple.com/standards/qtff-2001.pdf | Apple | Copyright Apple 2001/2002; no license stated | QTFF 2001-03-01, 274 pp. The current QTFF is a JS-rendered page at developer.apple.com/documentation/quicktime-file-format and was not saved |
| QuickTime | quicktime/loc-fdd000052.html, quicktime/loc-fdd000369.html | https://www.loc.gov/preservation/digital/formats/fdd/fdd000052.shtml, .../fdd000369.shtml | Library of Congress | public domain | QTFF, plus QTFF with yuv2 |

## High-content screening (plate) formats

| Format | File (relative path) | Source URL | Publisher | License/terms | Notes |
|---|---|---|---|---|---|
| GE/Cytiva InCell (xdce) | incell/incell-analyzer-2200-user-manual.pdf | https://download.cytivalifesciences.com/cellanalysis/download_data/incell/incell_docs/IN%20Cell%20Analyzer%202200%20User%20Manual.pdf | GE Healthcare / Cytiva | Vendor copyright; no license stated | User manual, not a format spec. Covers the image stack (.xdce) and the per-image TIFF output, plus FFC files |
| InCell (xdce) | incell/release-notes-v5.1.pdf | https://download.cytivalifesciences.com/cellanalysis/download_data/incell/incell_docs/INCell_Release_Notes_V5.1.pdf | Cytiva | Vendor copyright | Names some XDCE XML tags (serial_number, software_label), timestamp/timezone changes, and burst-mode Z quirks. Other release notes (V4.5–V7.4) are in the same folder |
| MetaXpress / CellWorX HTD | metaxpress-htd/metaxpress-6-batch-importing-exporting-images.pdf | https://www.moleculardevices.com/sites/default/files/en/assets/training-material/dd/img/metaxpress-6-software-batch-importing-and-exporting-images.pdf | Molecular Devices | Vendor copyright | Covers the .HTD plate description file, the naming convention and the TimePoint folder layout. Not a byte-level spec |
| MetaXpress | metaxpress-htd/metaxpress-analysis-guide-5039429B.pdf | https://med.stanford.edu/content/dam/sm/htbc/documents/eq/MetaXpress_Analysis_Guide.pdf | Molecular Devices (Stanford mirror) | Vendor copyright | Covers HTD import/export and creating HTD from Cellomics DIB. Marginal |
| Yokogawa CV7000/CV8000 | yokogawa-cv/cv7000-user-guide-nci-2016.pdf | https://content.ilabsolutions.com/wp-content/uploads/2017/09/CellVoyager-CV7000-User-Guide-for-NCI.pdf | Yokogawa (NCI training guide) | Vendor copyright | Names MeasurementDetail.mrf and MeasurementData.mlf and the folder layout. No schema |

## Scanning probe / surface formats

| Format | File (relative path) | Source URL | Publisher | License/terms | Notes |
|---|---|---|---|---|---|
| Veeco/DI Nanoscope (v2, v3, v4.x) | afm-nanoscope/arizona-di-fileformats.html | https://web.archive.org/web/20020406212022/http://www.physics.arizona.edu/~smanne/DI/software/fileformats.html | Srin Manne lab, Univ. of Arizona (archive.org copy) | unknown | Community-written. Covers header sections, Data offset/length and versions |
| Nanoscope v2 header | afm-nanoscope/arizona-di-v2header.html | https://web.archive.org/web/20030512200616/http://www.physics.arizona.edu/~smanne/DI/software/v2header.html | same | unknown | Community-written |
| Nanoscope v4.3x header | afm-nanoscope/arizona-di-v43header.html | https://web.archive.org/web/20020725212846/http://www.physics.arizona.edu/~smanne/DI/software/v43header.html | same | unknown | Community-written. Covers the `\@` CIAO parameters and Z scaling |
| Nanoscope | afm-nanoscope/arizona-spmdigest-header-qa.txt | https://web.archive.org/web/20160311095347/http://www.physics.arizona.edu/~smanne/DI/software/SPMdigest_header_q&a.txt | SPM Digest mailing list Q&A (posters incl. DI staff) | unknown | Z-scale formula (hard value × soft scale / 2^(8·bpp)) |
| Multi-vendor SPM headers (DI Nanoscope 2/3/4, Molecular Imaging, Burleigh) | spm-headers-weizmann/{index,nano2,nano3,nano4,mi,burl}.html | https://web.archive.org/web/2004*/http://www.weizmann.ac.il/Chemical_Research_Support/surflab/peter/headers/ | Peter Markiewicz (Weizmann Inst., archive.org copy) | unknown | Community-written. Vendor/version table plus sample headers. The index also links to reader programs (VB and C: hdfc, hdfvb, imgvb, nano2vb, nano4vb, stpvb, topoc), which were not saved (see the flag below) |
| JPK (.jpk, .jpk-force) | jpk/jpk-data-processing-manual-4.2.pdf | https://www.nanophys.kth.se/nanolab/afm/jpk/manuf-manuals/DPmanual.4.2.pdf | JPK Instruments AG (KTH mirror) | © 2009 JPK, all rights reserved | Vendor user manual. Describes file types and exports, not the binary layout |
| JPK archives | jpk/jpkfile-structure-of-jpk-archives.html | https://jpkfile.readthedocs.io/en/latest/structure.html | Ilyas Kuhlemann (jpkfile docs) | © 2019 I. Kuhlemann; license unknown | Community-written. Covers the zip layout and header.properties. Docs page only, not the library source |
| SPM overview | gwyddion-user-guide/file-formats.html | https://gwyddion.net/documentation/user-guide-en/file-formats.html | Gwyddion project | unknown (no license stated on the page) | Only a table of formats and extensions (Nanoscope, JPK, RHK SM2/3/4, Burleigh, Quesant, Seiko, Unisoku, Topometrix/SPMLab, Alicona...). No layouts. Useful to map extensions to vendors |

## Reverse-engineered / community-written documents

| Document | Author | Derived from another reader's source? |
|---|---|---|
| afm-nanoscope/arizona-*.html, arizona-spmdigest-header-qa.txt | Srin Manne lab (Univ. of Arizona), with mailing-list posters | No. Written from inspecting files and from DI correspondence. No reader source is cited |
| spm-headers-weizmann/* | Peter Markiewicz (Weizmann) | The saved pages are sample headers and a table. The same site also hosts the author's own reader programs (VB/C), which were not saved. Before I identified them as reader code, I opened topoc.html and hdfc.html (Topometrix v3 and Park HDF) in a scratch area. I then deleted them and saved nothing from them |
| jpk/jpkfile-structure-of-jpk-archives.html | Ilyas Kuhlemann | Not stated. Appears to come from inspecting archives. The author also wrote the jpkfile Python library (not downloaded) |
| tga/truevision-tga-2.0-spec.pdf | Truevision original; PDF conversion by Dominik Deák | No. A re-rendering of the vendor PostScript |

No document in this group says it was derived from Bio-Formats or any other reader's source.

## Not found / not freely available

| Format | What was looked for | Status / URL |
|---|---|---|
| InCell XDCE schema / .frm | Official XDCE XML schema | None public. Release notes say the full tag list is only in the software's "Properties" page. Cytiva doc index: https://download.cytivalifesciences.com/cellanalysis/download_data/incell/incell_docs/incell_docs.htm (also has the 6000 manual, workstation and remote-control docs, not downloaded) |
| PerkinElmer Opera Flex (.flex/.mea/.res) | Spec | None public. Flex planes may use proprietary LuraWave wavelet compression |
| PerkinElmer Operetta / Harmony / Columbus export (Index.idx.xml, Index.xml, Index.ref.xml, ImageIndex.ColumbusIDX.xml) | Spec / schema | None public. Only forum posts and third-party readers. Revvity Harmony manuals are behind customer login |
| Molecular Devices HTD (text) | Official article "Screening file format and naming convention" | https://support.moleculardevices.com/s/article/Screening-file-format-and-naming-convention . Has an annotated HTD example, but the Salesforce page renders only with JS and could not be saved statically. Worth saving by hand from a browser |
| Thermo Cellomics C01 / DIB | Spec | None public |
| Yokogawa CV7000/CV8000 .mrf/.mlf/.mes | Schema | None public beyond the user guide |
| BD Pathway | Spec | Nothing found |
| BioTek/Agilent Cytation (Gen5) | Gen5 File Export technical overview | https://www.agilent.com/cs/library/technicaloverviews/public/Gen5-file-export-5994-3431EN-agilent.pdf returned HTTP 403. The Cytation 5 manual (https://biomedcorefacilities.brown.edu/sites/default/files/biotekcytation5usermanual.pdf) and Gen5 Getting Started guides exist but were not downloaded (user docs, not format docs) |
| Tecan Spark Cyto, TissueGnostics TFCYTO, MIAS (Maia Scientific), JDCE | Spec | Nothing found |
| Bruker/Veeco Nanoscope official format chapter | "File format" appendix of the Command Reference Manual (004-122-000) | Not found online. The NanoScope 6.13 user guide and NanoScope V controller manual were checked and discarded (no file-format content) |
| RHK SM2/SM3/SM4 | RHK PDFs "SM4 Data File Format for R9 V5.0", "Parsing an SM4 file" | Not publicly hosted. An XPMPro 2.0 user manual with an SM4 section exists only on manualzz.com (JS viewer, third-party re-host; not downloaded) |
| Topometrix, Quesant, Burleigh (binary), Unisoku, WA-TOP, Seiko SPM, Oxford Instruments .top, Vectra | Spec | Nothing found except reader source code (not used) |
| Alicona AL3D | "AL3D specification v1.0 (2003)" | Not publicly hosted. Contact Alicona/Bruker |
| UBM .pr3 | Spec | Nothing found |
| Perkin Elmer Densitometer PDS | Spec | Nothing found |
| ISO/IEC 14496-12 (ISO BMFF, relevant to QTFF/MP4) | Free copy | On the ISO "Publicly Available Standards" list, but needs a click-through license. Not downloaded |
| Text image formats | n/a | Skipped as instructed |

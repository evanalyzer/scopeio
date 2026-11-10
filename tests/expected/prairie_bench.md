# prairie: CPU and RAM compared with the reference readers

Generated 2026-11-10 by `tests/scripts/oracle.py`.

- Machine: Intel(R) Core(TM) Ultra 9 185H, 22 threads, Linux 7.0.0-42-generic
- Bio-Formats 8.5.0 (bftools, Java 21.0.11), bioformats-rs 0.1.8
- Median of 3 runs, each a fresh process.
- scopeio: `scopeio_info --no-checksum` example, reads every plane of every series.
- CPU time and peak RAM via `tests/scripts/measure.rs` (wait4); its own footprint of about 2 MiB is included. Runs above 8192 MiB RSS are killed.
- Reference readers: convert the file to ICS, which also writes the pixels to disk. Bio-Formats times include JVM start-up.

| File | Size | Reader | Status | CPU s | Wall s | Peak RAM MiB |
|---|---:|---|---|---:|---:|---:|
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2022_11_03_IntoTheVoid_t_series-005/NCCR32_2022_11_03_IntoTheVoid_t_series-005.xml | 17 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2022_11_03_IntoTheVoid_t_series-005/NCCR32_2022_11_03_IntoTheVoid_t_series-005.xml | 17 KiB | bioformats | ok | 1.147 | 0.578 | 112.2 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2022_11_03_IntoTheVoid_t_series-005/NCCR32_2022_11_03_IntoTheVoid_t_series-005.xml | 17 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.9 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 5.5 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000.xml | 8 KiB | bioformats | ok | 1.172 | 0.581 | 111.6 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000/NCCR32_2023_02_20_Into_the_void_t_series_baseline-000.xml | 8 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000.xml | 10 KiB | scopeio | ok | 0.003 | 0.004 | 5.9 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000.xml | 10 KiB | bioformats | ok | 1.700 | 0.710 | 200.1 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000.xml | 10 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.002 | 0.003 | 12.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion.xml | 7 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion.xml | 7 KiB | bioformats | ok | 1.152 | 0.597 | 111.5 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion/NCCR62_2023_07_06_IntoTheVoid_t_series_Dual_color-000_with_companion.xml | 7 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-02022026-001_multisequence/TSeries-02022026-001_multisequence.xml | 15 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-02022026-001_multisequence/TSeries-02022026-001_multisequence.xml | 15 KiB | bioformats | ok | 2.287 | 0.766 | 182.2 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-02022026-001_multisequence/TSeries-02022026-001_multisequence.xml | 15 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002/TSeries-08162024-1918-002.xml | 7 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002/TSeries-08162024-1918-002.xml | 7 KiB | bioformats | ok | 1.090 | 0.574 | 107.9 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002/TSeries-08162024-1918-002.xml | 7 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002_multisequence/TSeries-08162024-1918-002_multisequence.xml | 10 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002_multisequence/TSeries-08162024-1918-002_multisequence.xml | 10 KiB | bioformats | ok | 1.142 | 0.586 | 111.2 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-08162024-1918-002_multisequence/TSeries-08162024-1918-002_multisequence.xml | 10 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-20240527-001/TSeries-20240527-001.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-20240527-001/TSeries-20240527-001.xml | 8 KiB | bioformats | ok | 1.181 | 0.601 | 111.0 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/TSeries-20240527-001/TSeries-20240527-001.xml | 8 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/ZSeries-20231129-001/ZSeries-20231129-001.xml | 9 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/ZSeries-20231129-001/ZSeries-20231129-001.xml | 9 KiB | bioformats | ok | 1.061 | 0.565 | 107.5 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/ZSeries-20231129-001/ZSeries-20231129-001.xml | 9 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.004 | 12.8 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/brightness_over_time_dual_channel_dodt/brightness_over_time_dual_channel_dodt.xml | 8 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/brightness_over_time_dual_channel_dodt/brightness_over_time_dual_channel_dodt.xml | 8 KiB | bioformats | error: loci.formats.FormatException: org.xml.sax.SAXParseExc | 0.880 | 0.505 | 103.4 |
| tests/reference/gin-ophys/imaging_datasets/BrukerTif/brightness_over_time_dual_channel_dodt/brightness_over_time_dual_channel_dodt.xml | 8 KiB | bioformats-rs | error: Error opening <file>: Format error: Not a Leica TCS X | 0.003 | 0.003 | 12.9 |
| tests/data/prairie/missing/missing.xml | 1 KiB | scopeio | ok | 0.001 | 0.002 | 5.5 |
| tests/data/prairie/missing/missing.xml | 1 KiB | bioformats | error: java.io.FileNotFoundException: /workspaces/scopeio/te | 0.987 | 0.551 | 106.3 |
| tests/data/prairie/missing/missing.xml | 1 KiB | bioformats-rs | error: Error reading plane 1: IO error: No such file or dire | 0.002 | 0.003 | 12.6 |
| tests/data/prairie/multiseq/multiseq.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 5.5 |
| tests/data/prairie/multiseq/multiseq.xml | 2 KiB | bioformats | ok | 1.005 | 0.547 | 107.8 |
| tests/data/prairie/multiseq/multiseq.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.004 | 12.9 |
| tests/data/prairie/timed/timed.xml | 2 KiB | scopeio | ok | 0.001 | 0.002 | 5.7 |
| tests/data/prairie/timed/timed.xml | 2 KiB | bioformats | ok | 1.070 | 0.591 | 109.6 |
| tests/data/prairie/timed/timed.xml | 2 KiB | bioformats-rs | ok | 0.003 | 0.003 | 12.9 |
| tests/data/prairie/zseries/zseries.xml | 4 KiB | scopeio | ok | 0.001 | 0.002 | 5.6 |
| tests/data/prairie/zseries/zseries.xml | 4 KiB | bioformats | ok | 1.110 | 0.587 | 111.3 |
| tests/data/prairie/zseries/zseries.xml | 4 KiB | bioformats-rs | ok | 0.003 | 0.004 | 13.0 |

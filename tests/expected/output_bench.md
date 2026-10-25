# Output options: CPU and RAM

Written by `tests/scripts/bench_output.py`.

## Scaled f32 output and sample selection

2026-10-25, x86_64, 22 threads, median of 9 runs, one process per run, before and after runs alternating.

Read s: wall time of the read (and a caller-side conversion) alone; CPU s and peak RAM: the whole process (open, read, checksum).

| Scenario | File | Region | Read s | CPU s | Wall s | Peak RAM MiB | Checksum |
|---|---|---|---:|---:|---:|---:|---|
| native (before) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.009 | 0.019 | 0.020 | 24.0 | 2484246997 |
| native (after) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.009 | 0.018 | 0.020 | 23.7 | 2484246997 |
| native-convert (before) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.020 | 0.034 | 0.036 | 58.3 | 606653.7393077933 |
| native-convert (after) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.021 | 0.035 | 0.036 | 58.2 | 606653.7393077933 |
| f32-normalized (before) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.024 | 0.037 | 0.038 | 41.2 | 37907.17931029387 |
| f32-normalized (after) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.020 | 0.033 | 0.035 | 41.0 | 37907.17931029387 |
| f32-significant-bits (after) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.019 | 0.032 | 0.033 | 41.0 | 37907.17931029387 |
| f32-scaled (after) | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.019 | 0.033 | 0.034 | 41.0 | 606653.7571305305 |
| rgb-3-sample-reads (before) | CMU-1.svs | 20000,14000,4096,4096 | 0.264 | 0.293 | 0.295 | 53.1 | 12360315113 |
| rgb-3-sample-reads (after) | CMU-1.svs | 20000,14000,4096,4096 | 0.126 | 0.155 | 0.157 | 53.1 | 12360315113 |
| rgb-samples (after) | CMU-1.svs | 20000,14000,4096,4096 | 0.061 | 0.088 | 0.090 | 53.0 | 12360315113 |
| rgb-channel-chw (before) | CMU-1.svs | 20000,14000,4096,4096 | 0.166 | 0.195 | 0.197 | 53.2 | 12360315113 |
| rgb-channel-chw (after) | CMU-1.svs | 20000,14000,4096,4096 | 0.060 | 0.089 | 0.091 | 53.1 | 12360315113 |
| ndpi:rgb-3-sample-reads (before) | CMU-1.ndpi | 20000,14000,4096,4096 | 0.379 | 0.411 | 0.412 | 53.8 | 9726372971 |
| ndpi:rgb-3-sample-reads (after) | CMU-1.ndpi | 20000,14000,4096,4096 | 0.238 | 0.269 | 0.271 | 53.7 | 9726372971 |
| ndpi:rgb-samples (after) | CMU-1.ndpi | 20000,14000,4096,4096 | 0.103 | 0.134 | 0.136 | 53.8 | 9726372971 |
| ndpi:rgb-channel-chw (before) | CMU-1.ndpi | 20000,14000,4096,4096 | 0.213 | 0.243 | 0.244 | 53.8 | 9726372971 |
| ndpi:rgb-channel-chw (after) | CMU-1.ndpi | 20000,14000,4096,4096 | 0.098 | 0.130 | 0.132 | 53.7 | 9726372971 |

# Output options: CPU and RAM

Written by `tests/scripts/bench_output.py`.

## before

2026-10-25, x86_64, 22 threads, median of 5 runs, one process per run.

| Scenario | File | Region | CPU s | Wall s | Peak RAM MiB | Checksum |
|---|---|---|---:|---:|---:|---|
| native | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.018 | 0.019 | 23.9 | 2484246997 |
| native-convert | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.033 | 0.034 | 58.5 | 606653.7393077933 |
| f32-normalized | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.035 | 0.037 | 41.1 | 37907.17931029387 |
| rgb-3-sample-reads | CMU-1.svs | 20000,14000,4096,4096 | 0.267 | 0.270 | 53.0 | 12360315113 |
| rgb-channel-chw | CMU-1.svs | 20000,14000,4096,4096 | 0.179 | 0.181 | 53.1 | 12360315113 |

## after: scaled f32 output

2026-10-25, x86_64, 22 threads, median of 5 runs, one process per run.

| Scenario | File | Region | CPU s | Wall s | Peak RAM MiB | Checksum |
|---|---|---|---:|---:|---:|---|
| native | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.021 | 0.023 | 23.6 | 2484246997 |
| native-convert | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.034 | 0.036 | 58.1 | 606653.7393077933 |
| f32-normalized | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.031 | 0.033 | 40.9 | 37907.17931029387 |
| f32-significant-bits | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.032 | 0.034 | 41.0 | 37907.17931029387 |
| f32-scaled | Stitch_A01_G001.oir | 0,0,3554,2540 | 0.032 | 0.033 | 40.8 | 606653.7571305305 |

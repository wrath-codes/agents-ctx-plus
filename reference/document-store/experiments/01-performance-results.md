# Performance Results

## Overview

Performance experiments were conducted to measure insertion times, retrieval times, the impact of key-value sizes, comparison with JavaScript, initial bucket count effects, and WARC parsing throughput. All values are averages of 10 runs. All times are in **seconds (s)**.

## Table 1: Insertion Times by Key-Value Size

Measures the time to insert records with varying key-value sizes (4, 8, 16, and 32 bytes). Starting with 2 initial buckets.

| Insertions | 4 bytes (s) | 8 bytes (s) | 16 bytes (s) | 32 bytes (s) |
|-----------|------------|------------|-------------|-------------|
| 10,000 | 19.5812 | 13.7196 | 10.7140 | 11.0392 |
| 20,000 | 40.5785 | 29.5620 | 29.2961 | 29.1158 |
| 50,000 | 135.2164 | 115.9305 | 110.8547 | 111.2843 |
| 100,000 | 294.1901 | 286.4809 | 290.2886 | 295.3691 |

**Analysis**: The 16-byte key-value size is the sweet spot for insertion performance. At 10K insertions, 16 bytes is nearly 2x faster than 4 bytes. This is due to how page size, bucket size, and item counts are configured. At higher volumes (100K), the differences diminish as bucket splitting overhead dominates.

## Table 2: Retrieval Times by Key-Value Size

Measures the time to retrieve records with varying key-value sizes.

| Retrievals | 4 bytes (s) | 8 bytes (s) | 16 bytes (s) | 32 bytes (s) |
|-----------|------------|------------|-------------|-------------|
| 10,000 | 8.4741 | 6.0003 | 4.0419 | 4.2534 |
| 20,000 | 22.6415 | 18.2303 | 11.5651 | 11.5278 |
| 50,000 | 77.5042 | 53.4573 | 50.5842 | 55.6155 |
| 100,000 | 103.5246 | 91.2837 | 92.4166 | 91.2974 |

**Analysis**: Retrieval follows the same pattern as insertion — 16 bytes is optimal. Retrievals are consistently faster than insertions because they do not trigger bucket splits or write operations.

## Table 3: Rust vs JavaScript Comparison

Compares insertion performance between the Rust and JavaScript implementations using 4-byte key-value size with 2 initial buckets.

| Insertions | Rust (s) | JavaScript (s) |
|-----------|---------|----------------|
| 10,000 | 19.5812 | 45.0361 |
| 20,000 | 40.5785 | 101.1892 |
| 50,000 | 135.2164 | 296.7893 |
| 100,000 | 294.1901 | 687.6312 |

**Analysis**: Rust is approximately **2.3x faster** than JavaScript across all insertion volumes. The performance gap is consistent, indicating Rust's advantage comes from lower-level memory management and lack of garbage collection overhead rather than algorithmic differences.

## Table 4: Impact of Initial Bucket Count

Measures how the starting number of buckets affects insertion performance (4-byte key-value size).

| Insertions | 2 buckets (s) | 256 buckets (s) | 1024 buckets (s) |
|-----------|--------------|----------------|------------------|
| 10,000 | 19.5812 | 6.1263 | 6.0912 |
| 20,000 | 40.5785 | 14.3429 | 15.2482 |
| 50,000 | 135.2164 | 43.1485 | 41.6578 |
| 100,000 | 294.1901 | 195.3374 | 156.7210 |

**Analysis**: Starting with more buckets dramatically reduces insertion time:
- At 10K insertions, 256 buckets is **3.2x faster** than 2 buckets
- At 100K insertions, 1024 buckets is **1.9x faster** than 2 buckets
- The difference between 256 and 1024 initial buckets is minimal at lower volumes, but becomes significant at 100K insertions
- More initial buckets means fewer split operations, which are the primary source of overhead

## Table 5: WARC Parsing Performance

Measures the time to parse records from a compressed (gzipped) WARC file. This is approximately the same performance as the `warcio` library in Python.

| Records Parsed | Time - Rust (s) |
|---------------|----------------|
| 10,000 | 71.5156 |
| 20,000 | 148.0942 |
| 50,000 | 432.8688 |

**Analysis**: WARC parsing time grows approximately linearly with the number of records. Including conversion of WARC records into linear hash-table format (via PackedTableTools), 10,000 records takes an average of 80 seconds.

The WARC utility can also write WARC files: 10,000 records with a body of ~60 bytes takes an average of 2 seconds.

## Key Findings

### 16-Byte Sweet Spot

The 16-byte key-value size consistently delivers the best performance for both insertions and retrievals. This is due to the way page size, bucket size, and size and number of items in the bucket are configured. Smaller sizes (4 bytes) are slower, while larger sizes (32 bytes) show no significant improvement over 16 bytes.

### 80% Split Threshold

The 80% load factor threshold for bucket splitting balances between:
- **Too low** (e.g., 50%) — excessive splitting, wasted space
- **Too high** (e.g., 95%) — long bucket chains, degraded lookup performance
- **80%** — good balance of space utilization and lookup speed

Different results are seen when the bucket splitting threshold is changed.

### Logarithmic Insertion Time Growth

Insertion time increases logarithmically with the number of records. This is because:
- Each split operation redistributes only one bucket
- The number of splits grows logarithmically relative to total insertions
- Page I/O remains constant (4KB per operation)

### Retrieval vs Insertion

Retrievals are significantly faster than insertions because:
- No write I/O required
- No load factor check
- No bucket splitting
- Read-only page access

## Summary

| Finding | Detail |
|---------|--------|
| Optimal key-value size | 16 bytes |
| Rust vs JS speedup | ~2.3x |
| Split threshold | 80% load factor |
| Best initial buckets | 1024 (for 100K+ records) |
| WARC parse rate (linear) | ~8.7s per 1,000 records |
| Insertion time growth | Logarithmic |

## Next Steps

- **[Linear Hashing](../architecture/02-linear-hashing.md)** — Algorithm details behind these results
- **[WARC Files](../data-formats/01-warc-files.md)** — WARC parsing implementation
- **[Planned Improvements](../future-work/02-improvements.md)** — How performance could be further improved

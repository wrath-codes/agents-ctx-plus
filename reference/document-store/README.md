# High Performance Document Store Implementation in Rust

> **A linear hashing-based document store built in Rust for search engine backends**

A high-performance document store designed to support applications requiring fast data storage and retrieval. Built in Rust for speed, robustness, and memory efficiency, it uses linear hashing for dynamic bucket management and supports WebArchive (WARC) file ingestion as a data feed. The system serves as a backend service for search engines like Yioop via a GraphQL API.

## Key Features

- **Linear Hashing** - Dynamic hash table with automatic bucket splitting at 80% load factor
- **WARC Support** - Native reader/writer for WebArchive files with gzip decompression
- **CDX Indexing** - Efficient offset-based jumping into large WARC archives
- **GraphQL API** - HTTP-based query interface for search engine integration
- **Rust Performance** - 2-3x faster than equivalent JavaScript implementation
- **4KB Page Size** - Disk-backed storage with page-level I/O

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│            GraphQL Server                   │
│       (HTTP requests from Yioop)            │
├─────────────────────────────────────────────┤
│            Query Engine                     │
│     (Schema + Query Execution)              │
├─────────────────────────────────────────────┤
│         Linear Hash Table                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Bucket 0 │  │ Bucket 1 │  │ Bucket N │  │
│  │ (Pages)  │  │ (Pages)  │  │ (Pages)  │  │
│  └──────────┘  └──────────┘  └──────────┘  │
├─────────────────────────────────────────────┤
│         Buffer Pool / Disk I/O              │
│           (4KB Page Storage)                │
├─────────────────────────────────────────────┤
│         WARC Reader / Writer                │
│     (gzip via libflate, CDX index)          │
└─────────────────────────────────────────────┘
```

## Performance Highlights

| Metric | Value |
|--------|-------|
| 10K insertions (16-byte keys) | ~10.7 seconds |
| 10K retrievals (16-byte keys) | ~4.0 seconds |
| Rust vs JS speedup | ~2.3x faster |
| WARC parse (10K records) | ~71.5 seconds |
| Optimal key-value size | 16 bytes |

## Documentation Map

```
reference/document-store/
├── index.md                          # Comprehensive reference
├── architecture/
│   ├── 01-system-overview.md         # System design and components
│   ├── 02-linear-hashing.md          # Linear hashing implementation
│   └── 03-indexing.md                # PackedTableTools packing format
├── data-formats/
│   ├── 01-warc-files.md              # WebArchive file handling
│   └── 02-cdx-files.md               # CDX index file format
├── query-engine/
│   └── 01-graphql-server.md          # GraphQL API server
├── experiments/
│   └── 01-performance-results.md     # Benchmark tables and analysis
├── future-work/
│   ├── 01-consistent-hashing.md      # Distributed hashing design
│   └── 02-improvements.md           # Planned improvements
└── challenges/
    └── 01-implementation-challenges.md  # Rust porting challenges
```

## Quick Links

- **[Complete Reference](index.md)** - Full documentation and navigation
- **[Architecture](architecture/)** - System design and linear hashing
- **[Data Formats](data-formats/)** - WARC and CDX file handling
- **[Performance Results](experiments/01-performance-results.md)** - Benchmark data
- **[Future Work](future-work/)** - Distributed hashing and improvements

## Citation

> Aggarwal, Ishaan, "High Performance Document Store Implementation in Rust" (2021). Master's Projects. 1044. DOI: [https://doi.org/10.31979/etd.96kc-fcmu](https://doi.org/10.31979/etd.96kc-fcmu)

---

*San Jose State University, Fall 2021*

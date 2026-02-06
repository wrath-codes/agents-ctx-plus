# DuckDB

> Analytical in-process SQL database management system. DuckDB is designed for fast analytics on embedded and edge devices, with no external dependencies.

## Overview

DuckDB is an embeddable SQL OLAP (Online Analytical Processing) database management system. Key characteristics:

| Property | Description |
|----------|-------------|
| **Type** | In-process (embedded), no server required |
| **Model** | Columnar storage, vectorized execution |
| **SQL** | PostgreSQL-compatible dialect |
| **Size** | Single dependency, ~20MB compiled |
| **Speed** | Optimized for analytical (OLAP) queries |
| **Portability** | Single-file database format (`.duckdb`) |

## Architecture

```
┌─────────────────────────────────────────┐
│           SQL Interface                 │
│    (Parser, Binder, Logical Planner)    │
├─────────────────────────────────────────┤
│         Optimizer Engine                │
│   (Statistics, Join Ordering, etc.)   │
├─────────────────────────────────────────┤
│       Execution Engine                  │
│   (Vectorized, Parallel, Streaming)     │
├─────────────────────────────────────────┤
│      Storage Layer                      │
│   (Columnar, Compression, WAL)          │
└─────────────────────────────────────────┘
```

## Key Concepts

### In-Process Architecture
- No separate server process — database runs inside your application
- Direct memory access to data (no IPC overhead)
- Perfect for edge computing, data science, and embedded analytics

### Columnar Storage
- Data stored by column, not by row
- Efficient compression (RLE, Dictionary, Bitpacking)
- Vectorized query execution (processing chunks of data)

### Zero External Dependencies
- Single static library or executable
- No need for PostgreSQL, MySQL, or other services
- Portable across platforms (Linux, macOS, Windows, WASM)

## Installation Methods

### CLI
```bash
# macOS
brew install duckdb

# Linux (various distros)
wget https://github.com/duckdb/duckdb/releases/download/v1.4.1/duckdb_cli-linux-amd64.zip

# All platforms
pip install duckdb
```

### Rust
```toml
[dependencies]
duckdb = { version = "1.4.4", features = ["bundled"] }
```

### Python
```bash
pip install duckdb
```

## Quick Start

```sql
-- Create in-memory database (default)
SELECT * FROM range(10);

-- Persist to file
ATTACH 'mydb.duckdb' AS mydb;
USE mydb;

-- Query Parquet directly
SELECT * FROM 'data.parquet';

-- Read from HTTP
SELECT * FROM 'https://example.com/data.csv';
```

## Extensions System

DuckDB uses a modular extension architecture:

| Category | Source | Installation |
|----------|--------|--------------|
| **Built-in** | Core team | Statically linked, always available |
| **Core Extensions** | Core team | `INSTALL httpfs; LOAD httpfs;` |
| **Community Extensions** | Third-party | `INSTALL xyz FROM community;` |

## Storage Format

- **File extension**: `.duckdb`
- **Format**: Single-file, portable
- **Compression**: Built-in (auto-selected)
- **WAL**: Write-ahead logging for durability
- **Versioning**: Storage format evolves with DuckDB versions

## Use Cases

1. **Data Science** — Pandas/Arrow/Polars integration
2. **Edge Analytics** — Embedded devices, IoT
3. **Testing** — Fast, isolated database for tests
4. **ETL Pipelines** — Transform CSV, JSON, Parquet
5. **Local Analytics** — Query large datasets on laptop
6. **Serverless** — WASM deployment in browsers

## Comparison

| Feature | DuckDB | SQLite | PostgreSQL |
|---------|--------|--------|------------|
| Type | Embedded | Embedded | Server |
| Model | Columnar | Row | Row |
| OLAP | Excellent | Poor | Good |
| OLTP | Good | Excellent | Excellent |
| Size | ~20MB | ~1MB | Large |
| Extensions | Rich | Limited | Moderate |

## Cross-References

- [Core Extensions](core-extensions/01-overview.md) — Built-in and installable extensions
- [Community Extensions](community-extensions/01-overview.md) — Third-party contributions
- [Community Extension Reference](community-extensions/04-extension-reference.md) — Complete catalog of 130+ extensions
- [Rust SDK](rust-sdk/01-overview.md) — Rust crate documentation

## Resources

- [Official Documentation](https://duckdb.org/docs/)
- [GitHub Repository](https://github.com/duckdb/duckdb)
- [Rust Crate (duckdb-rs)](https://github.com/duckdb/duckdb-rs)
- [Community Extensions](https://duckdb.org/community_extensions/)

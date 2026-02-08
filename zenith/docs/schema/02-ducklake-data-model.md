# Zenith: DuckLake Data Model

**Version**: 2026-02-07
**Status**: Design Document
**Purpose**: Package documentation lake schema -- DuckDB + DuckLake + MotherDuck + Cloudflare R2

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Storage Layout](#3-storage-layout)
4. [Tables](#4-tables)
5. [Vector Search](#5-vector-search)
6. [Indexing Pipeline](#6-indexing-pipeline)
7. [Query Patterns](#7-query-patterns)
8. [Extensions](#8-extensions)
9. [Setup](#9-setup)

---

## 1. Overview

The DuckLake stores **indexed package documentation** -- tree-sitter-extracted API symbols and chunked documentation text with fastembed vector embeddings. This is what powers `zen search`.

### Design Principles

- **DuckLake** provides ACID transactions, time travel (snapshots), and schema evolution over Parquet files
- **MotherDuck** hosts the catalog metadata and provides cloud compute for heavy queries
- **Cloudflare R2** stores the actual Parquet data files ($0.015/GB/mo, zero egress)
- **Local DuckDB** connects to the DuckLake for development and pipeline use
- **fastembed** generates embeddings locally (ONNX runtime, no API keys)
- **VSS extension** provides HNSW-indexed vector similarity search

### What Gets Indexed

When a user runs `zen install <package>`:

1. Clone the package repository to a temp directory
2. Parse source files with tree-sitter (16 supported languages)
3. Extract API symbols: functions, structs, enums, traits, classes, interfaces, type aliases, constants, macros, modules
4. Chunk documentation files (README, docs/, guides)
5. Generate fastembed vectors for symbols and doc chunks
6. Write Parquet files to R2 via DuckLake
7. Register the package in `indexed_packages`

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Zenith CLI (zen)                            │
│                                                                     │
│  zen install <pkg>    zen search <query>    zen onboard             │
└──────────┬────────────────────┬────────────────────┬────────────────┘
           │                    │                    │
           ▼                    ▼                    ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│  Clone + Parse   │  │  DuckDB Query    │  │  Detect Deps +   │
│  (tree-sitter +  │  │  (VSS + FTS)     │  │  Batch Index     │
│   fastembed)     │  │                  │  │                  │
└──────────┬───────┘  └──────────┬───────┘  └──────────┬───────┘
           │                     │                     │
           ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           DuckLake                                   │
│                                                                      │
│  ┌─────────────────────────┐       ┌──────────────────────────────┐ │
│  │  MotherDuck             │       │  Cloudflare R2               │ │
│  │  (Catalog + Compute)    │       │  (Parquet Storage)           │ │
│  │                         │       │                              │ │
│  │  • Table metadata       │◄─────►│  s3://zenith-lake/           │ │
│  │  • Snapshots            │       │    └── {ecosystem}/          │ │
│  │  • Statistics           │       │        └── {package}/        │ │
│  │  • HNSW index state     │       │            └── {version}/    │ │
│  │                         │       │                ├── api.pq    │ │
│  │  Database: zenith_lake  │       │                └── docs.pq   │ │
│  └─────────────────────────┘       └──────────────────────────────┘ │
│           ▲                                                          │
│           │                                                          │
│  ┌────────┴────────┐                                                 │
│  │  Local DuckDB   │                                                 │
│  │  (dev/pipeline)  │                                                │
│  └─────────────────┘                                                 │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 3. Storage Layout

### R2 Bucket

```
s3://zenith-lake/
└── {ecosystem}/
    └── {package}/
        └── {version}/
            ├── api_symbols.parquet
            └── doc_chunks.parquet
```

**Partition scheme:** `ecosystem / package / version`

Examples:
```
s3://zenith-lake/rust/tokio/1.40.0/api_symbols.parquet
s3://zenith-lake/rust/tokio/1.40.0/doc_chunks.parquet
s3://zenith-lake/npm/zod/3.23.0/api_symbols.parquet
s3://zenith-lake/npm/zod/3.23.0/doc_chunks.parquet
s3://zenith-lake/hex/phoenix/1.7.14/api_symbols.parquet
s3://zenith-lake/hex/phoenix/1.7.14/doc_chunks.parquet
```

This partition scheme enables:
- Scoped queries: `FROM 's3://zenith-lake/rust/tokio/**/*.parquet'` for single-package search
- Cross-package queries: `FROM 's3://zenith-lake/rust/**/*.parquet'` for ecosystem-wide search
- DuckDB glob pattern support for efficient partition pruning

### Local Cache

```
.zenith/
  lake/
    cache.duckdb       # Local DuckDB with cached query results
```

---

## 4. Tables

### indexed_packages

Registry of all packages that have been cloned, parsed, and indexed.

```sql
CREATE TABLE indexed_packages (
    ecosystem TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    repo_url TEXT,
    description TEXT,
    license TEXT,
    downloads BIGINT,
    indexed_at TIMESTAMP DEFAULT current_timestamp,
    file_count INTEGER DEFAULT 0,
    symbol_count INTEGER DEFAULT 0,
    PRIMARY KEY (ecosystem, name, version)
);
```

| Column | Description |
|--------|-------------|
| `ecosystem` | `rust`, `npm`, `hex`, `pypi`, `go` |
| `name` | Package name as published in registry |
| `version` | Exact version that was indexed |
| `repo_url` | Source repository URL (GitHub, GitLab, etc.) |
| `description` | Package description from registry |
| `license` | License identifier (MIT, Apache-2.0, etc.) |
| `downloads` | Download count at time of indexing (for relevance ranking) |
| `file_count` | Number of source files parsed |
| `symbol_count` | Total API symbols extracted |

### api_symbols

Tree-sitter-extracted public API symbols from package source code.

```sql
CREATE TABLE api_symbols (
    id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    signature TEXT,
    doc_comment TEXT,
    line_start INTEGER,
    line_end INTEGER,
    visibility TEXT,
    is_async BOOLEAN DEFAULT FALSE,
    is_unsafe BOOLEAN DEFAULT FALSE,
    return_type TEXT,
    generics TEXT,
    attributes TEXT,
    metadata JSON,
    embedding FLOAT[384],
    PRIMARY KEY (id)
);
```

| Column | Description |
|--------|-------------|
| `id` | Deterministic hash: `sha256(ecosystem:package:version:file_path:kind:name)`, truncated |
| `kind` | Symbol type (see table below) |
| `name` | Symbol name as it appears in source |
| `signature` | Full signature line (no body). E.g., `pub async fn spawn<F: Future>(f: F) -> JoinHandle<F::Output>` |
| `doc_comment` | Extracted doc comment (/// in Rust, docstring in Python, JSDoc in TS) |
| `visibility` | `pub`, `pub(crate)`, `private`, `export`, `protected` |
| `is_async` | Whether the symbol is async |
| `is_unsafe` | Whether the symbol is unsafe (Rust) |
| `return_type` | Extracted return type string |
| `generics` | Generic parameters string (e.g., `<T: Clone + Send>`) |
| `attributes` | JSON array of attributes/decorators (e.g., `["derive(Debug, Clone)", "cfg(feature = \"full\")"]`) |
| `metadata` | Language-specific extras as JSON (see below) |
| `embedding` | 384-dimensional fastembed vector |

**Symbol kinds:**

| Kind | Languages |
|------|-----------|
| `function` | All |
| `method` | All (within impl/class) |
| `struct` | Rust, Go, Zig, Mojo |
| `enum` | Rust, Python, TypeScript |
| `trait` | Rust, Mojo |
| `interface` | TypeScript, Go |
| `class` | Python, TypeScript, JavaScript, Mojo |
| `type_alias` | Rust, TypeScript, Go |
| `const` | All |
| `static` | Rust |
| `macro` | Rust, Elixir |
| `module` | Rust, Python, Elixir |
| `union` | Rust |

**Metadata JSON (language-specific):**

Rust:
```json
{
    "lifetimes": ["'a", "'static"],
    "where_clause": "where T: Send + 'static",
    "is_pyo3": false,
    "trait_name": "Iterator",
    "for_type": "Vec<T>",
    "variants": ["Some(T)", "None"],
    "fields": ["name: String", "age: u32"],
    "methods": ["new", "build", "execute"],
    "associated_types": ["Item", "Error"],
    "abi": "C",
    "doc_sections": {
        "errors": "Returns Err if...",
        "panics": "Panics if...",
        "safety": "Caller must ensure...",
        "examples": "```rust\nlet x = ...\n```"
    }
}
```

Python:
```json
{
    "is_generator": false,
    "is_property": true,
    "is_pydantic": false,
    "is_protocol": false,
    "is_dataclass": true,
    "base_classes": ["BaseModel", "Generic[T]"],
    "decorators": ["@staticmethod", "@override"],
    "parameters": ["self", "name: str", "age: int = 0"],
    "doc_sections": {
        "args": {"name": "The user's name", "age": "Optional age"},
        "returns": "A new User instance",
        "raises": {"ValueError": "If name is empty"}
    }
}
```

TypeScript/JavaScript:
```json
{
    "is_exported": true,
    "is_default_export": false,
    "type_parameters": "<T extends Record<string, unknown>>",
    "implements": ["Serializable", "Comparable<T>"]
}
```

### doc_chunks

Chunked documentation text from READMEs, guides, and doc files. Each chunk is a paragraph-level or section-level piece of text.

```sql
CREATE TABLE doc_chunks (
    id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    title TEXT,
    content TEXT NOT NULL,
    source_file TEXT,
    format TEXT,
    embedding FLOAT[384],
    PRIMARY KEY (id)
);
```

| Column | Description |
|--------|-------------|
| `id` | Deterministic hash: `sha256(ecosystem:package:version:source_file:chunk_index)`, truncated |
| `chunk_index` | Sequential index within the source file |
| `title` | Section heading or first line summary |
| `content` | Chunk text content |
| `source_file` | Relative path within repo: `README.md`, `docs/guide.md`, `CHANGELOG.md` |
| `format` | `md`, `rst`, `txt`, `html` |
| `embedding` | 384-dimensional fastembed vector |

---

## 5. Vector Search

### HNSW Indexes

```sql
CREATE INDEX idx_symbols_embedding ON api_symbols USING HNSW(embedding);
CREATE INDEX idx_chunks_embedding ON doc_chunks USING HNSW(embedding);
```

### Query Indexes

```sql
CREATE INDEX idx_symbols_pkg ON api_symbols(ecosystem, package, version);
CREATE INDEX idx_symbols_kind ON api_symbols(kind);
CREATE INDEX idx_symbols_name ON api_symbols(name);
CREATE INDEX idx_symbols_visibility ON api_symbols(visibility);
CREATE INDEX idx_chunks_pkg ON doc_chunks(ecosystem, package, version);
CREATE INDEX idx_chunks_source ON doc_chunks(source_file);
```

### Similarity Search

```sql
-- Find API symbols similar to a query embedding
SELECT ecosystem, package, version, kind, name, signature, doc_comment,
    array_cosine_similarity(embedding, $query_embedding::FLOAT[384]) as score
FROM api_symbols
WHERE ecosystem = 'rust'
ORDER BY score DESC
LIMIT 20;
```

### Hybrid Search (Vector + Filter)

```sql
-- Find async functions in tokio that match a semantic query
SELECT name, signature, doc_comment,
    array_cosine_similarity(embedding, $query_embedding::FLOAT[384]) as score
FROM api_symbols
WHERE ecosystem = 'rust'
  AND package = 'tokio'
  AND kind = 'function'
  AND is_async = TRUE
ORDER BY score DESC
LIMIT 10;
```

---

## 6. Indexing Pipeline

### Step-by-Step

```
1. Clone Repository
   git clone --depth 1 <repo_url> /tmp/zenith-index/<pkg>

2. Detect Language
   Walk files, match extensions to supported languages (16 grammars)
   Skip: test files, test directories, vendor, node_modules, etc.

3. Parse with Tree-Sitter
   For each source file:
     a. Detect language from file extension
     b. Parse with tree-sitter grammar
     c. Walk AST, extract public symbols
     d. Build ParsedItem with rich metadata (klaw-style)

4. Extract Documentation
   Find README.md, docs/*, CHANGELOG.md, etc.
   Chunk by section headings (## in markdown, etc.)
   Each chunk becomes a doc_chunks row

5. Generate Embeddings
   Batch all symbol signatures + doc chunks through fastembed
   Model: default fastembed model (384 dimensions)

6. Write to DuckLake
   INSERT INTO api_symbols ...
   INSERT INTO doc_chunks ...
   INSERT INTO indexed_packages ...

7. Update Turso
   UPDATE project_dependencies SET indexed = TRUE WHERE name = <pkg>

8. Cleanup
   rm -rf /tmp/zenith-index/<pkg>
```

### Incremental Re-indexing

When a new version is available:

1. Check if `(ecosystem, name, new_version)` exists in `indexed_packages`
2. If not, run the full pipeline for the new version
3. Old versions remain indexed (queries can specify version or default to latest)

### Batch Indexing (Onboard)

`zen onboard` reads the project manifest and indexes all dependencies:

```
1. Detect project type (Cargo.toml, package.json, etc.)
2. Parse manifest → list of (ecosystem, name, version) tuples
3. Insert into project_dependencies
4. For each dependency not already indexed:
   a. Check indexed_packages in DuckLake
   b. If missing, run indexing pipeline
   c. If present (from another project), just mark as indexed
```

---

## 7. Query Patterns

### Search by Package

```sql
-- All public functions in tokio
SELECT name, signature, doc_comment
FROM api_symbols
WHERE ecosystem = 'rust' AND package = 'tokio' AND kind = 'function'
ORDER BY name;
```

### Search by Symbol Name

```sql
-- Find any symbol named "spawn" across all indexed packages
SELECT ecosystem, package, version, kind, signature
FROM api_symbols
WHERE name = 'spawn'
ORDER BY downloads DESC;  -- from indexed_packages join
```

### Search by Kind

```sql
-- All traits in the axum ecosystem
SELECT name, signature, doc_comment
FROM api_symbols
WHERE ecosystem = 'rust' AND package = 'axum' AND kind = 'trait';
```

### Semantic Search

```sql
-- "How do I set up middleware?" across all rust packages
SELECT package, name, signature, doc_comment,
    array_cosine_similarity(embedding, $query_embedding::FLOAT[384]) as score
FROM api_symbols
WHERE ecosystem = 'rust'
ORDER BY score DESC
LIMIT 10;
```

### Documentation Search

```sql
-- Search doc chunks for "error handling" in a specific package
SELECT title, content, source_file,
    array_cosine_similarity(embedding, $query_embedding::FLOAT[384]) as score
FROM doc_chunks
WHERE ecosystem = 'rust' AND package = 'anyhow'
ORDER BY score DESC
LIMIT 5;
```

### Cross-Package Comparison

```sql
-- Compare API surface of two HTTP client libraries
SELECT package, kind, COUNT(*) as count
FROM api_symbols
WHERE ecosystem = 'rust'
  AND package IN ('reqwest', 'hyper')
  AND visibility = 'pub'
GROUP BY package, kind
ORDER BY package, count DESC;
```

---

## 8. Extensions

### Required DuckDB Extensions

| Extension | Purpose | Install |
|-----------|---------|---------|
| `vss` | Vector similarity search (HNSW) | `INSTALL vss; LOAD vss;` |
| `parquet` | Columnar storage | Built-in |
| `json` | JSON operations for metadata column | Built-in |
| `httpfs` | S3/R2 remote file access | `INSTALL httpfs; LOAD httpfs;` |
| `fts` | Full-text search (optional, if needed alongside vector) | `INSTALL fts; LOAD fts;` |

### DuckLake Extension

```sql
INSTALL ducklake;
LOAD ducklake;
```

---

## 9. Setup

### Environment Variables

```bash
# .env (or .zenith/config.toml)
R2_ACCOUNT_ID=<cloudflare-account-id>
R2_ACCESS_KEY_ID=<r2-access-key>
R2_SECRET_ACCESS_KEY=<r2-secret>
R2_BUCKET_NAME=zenith-lake

MOTHERDUCK_ACCESS_TOKEN=<motherduck-token>
```

### DuckLake Initialization

```sql
-- 1. Create R2 secret in MotherDuck
CREATE SECRET r2_zenith IN MOTHERDUCK (
    TYPE s3,
    KEY_ID '<R2_ACCESS_KEY_ID>',
    SECRET '<R2_SECRET_ACCESS_KEY>',
    ENDPOINT '<R2_ACCOUNT_ID>.r2.cloudflarestorage.com',
    URL_STYLE 'path'
);

-- 2. Create DuckLake database with R2 storage
CREATE DATABASE zenith_lake (
    TYPE DUCKLAKE,
    DATA_PATH 's3://zenith-lake/'
);

-- 3. Create tables
USE zenith_lake;

CREATE TABLE indexed_packages ( ... );
CREATE TABLE api_symbols ( ... );
CREATE TABLE doc_chunks ( ... );

-- 4. Create HNSW indexes
CREATE INDEX idx_symbols_embedding ON api_symbols USING HNSW(embedding);
CREATE INDEX idx_chunks_embedding ON doc_chunks USING HNSW(embedding);
```

### Local Development (No Cloud)

For offline development, use a local DuckDB file:

```sql
-- Local-only mode (no MotherDuck, no R2)
ATTACH 'zenith_lake.duckdb' AS zenith_lake;

-- Same schema, data stored in local Parquet files
-- .zenith/lake/ directory
```

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Aether storage validation: `~/projects/aether/crates/aether-storage/src/bin/test_r2_ducklake.rs`

# Zenith: Data Architecture

**Version**: 2026-02-09
**Status**: Design Document (validated in spikes 0.17-0.20)
**Purpose**: Package index storage architecture — Turso catalog + Lance on R2 + DuckDB query engine
**Supersedes**: `02-ducklake-data-model.md` (MotherDuck/DuckLake architecture, retired)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Three-Tier Index Model](#3-three-tier-index-model)
4. [Turso Catalog (DuckLake-Inspired)](#4-turso-catalog-ducklake-inspired)
5. [Lance Storage on R2](#5-lance-storage-on-r2)
6. [DuckDB Query Engine](#6-duckdb-query-engine)
7. [Data Schemas](#7-data-schemas)
8. [Indexing Pipeline](#8-indexing-pipeline)
9. [Search Flows](#9-search-flows)
10. [Authentication & Visibility](#10-authentication--visibility)
11. [Source Files (znt grep)](#11-source-files-znt-grep)
12. [Environment Variables](#12-environment-variables)
13. [Validated In](#13-validated-in)

---

## 1. Overview

The package index stores **indexed package documentation** — ast-grep-extracted API symbols and chunked documentation text with fastembed vector embeddings. This is what powers `znt search`, `znt grep`, and `znt onboard`.

### Design Principles

- **Turso** (libsql) as the global catalog — DuckLake-inspired schema with snapshots, visibility scoping, embedded replicas on every client
- **Lance** on Cloudflare R2 as the search data store — native vector/FTS/hybrid search, written by `lancedb` Rust crate via `serde_arrow`
- **DuckDB** as the local read-only query engine — lance extension for search, no storage management
- **Clerk** JWT via JWKS for authentication — `sub` (user_id), `org_id`, `org_role` drive visibility scoping without custom RBAC
- **Crowdsourced** — any authenticated user can index a public package; first indexer wins (PRIMARY KEY dedup)
- **fastembed** generates embeddings locally (ONNX runtime, 384-dim, no API keys)

### What Changed from the DuckLake/MotherDuck Architecture

| Component | Old (doc 02 v1) | New (this doc) | Why |
|-----------|----------------|----------------|-----|
| Catalog | MotherDuck (DuckLake extension) | **Turso** (DuckLake-inspired tables) | Multi-user replicas, Clerk JWT auth, no $250/mo, no per-user accounts |
| Data format | Parquet on R2 | **Lance** on R2 | Native vector/FTS/hybrid search, persistent indexes |
| Write path | DuckDB `COPY TO` → Parquet | **lancedb** Rust crate via `serde_arrow` | Type-safe, no SQL string building, explicit index creation |
| Read path | DuckDB `read_parquet()` + `array_cosine_similarity()` | **DuckDB lance extension** (`lance_vector_search`, `lance_fts`, `lance_hybrid_search`) | Persistent vector indexes, BM25 FTS, no brute-force scan |
| Auth | None | **Clerk** JWT (JWKS) | Zero-cost auth, org claims for team visibility |
| Visibility | None (single user) | **public/team/private** scoping | Crowdsourced global index + team/private code |

### What Gets Indexed

When a user runs `znt install <package>`:

1. Check Turso catalog — already indexed globally? If yes, skip (crowdsource dedup)
2. Clone the package repository to a temp directory
3. Parse source files with ast-grep (26 built-in languages)
4. Extract API symbols: functions, structs, enums, traits, classes, interfaces, type aliases, constants, macros, modules
5. Chunk documentation files (README, docs/, guides)
6. Generate fastembed vectors (384-dim) for symbols and doc chunks
7. Write Lance datasets to R2 via `lancedb` Rust crate (`serde_arrow` → RecordBatch → lance)
8. Register in Turso catalog (`dl_data_file` + `dl_snapshot`)
9. Store source files in local DuckDB for `znt grep`

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Zenith CLI (znt)                            │
│                                                                     │
│  znt install <pkg>    znt search <query>    znt onboard             │
└──────────┬────────────────────┬────────────────────┬────────────────┘
           │                    │                    │
           ▼                    ▼                    ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│  Clone + Parse   │  │  Turso Catalog   │  │  Detect Deps +   │
│  + Embed         │  │  → Lance Paths   │  │  Check Catalog   │
│  (ast-grep +     │  │  → DuckDB Query  │  │  → Batch Index   │
│   fastembed +    │  │  (lance ext)     │  │                  │
│   serde_arrow)   │  │                  │  │                  │
└──────────┬───────┘  └──────────────────┘  └──────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  ┌─────────────────────────┐       ┌──────────────────────────────┐ │
│  │  Turso Cloud            │       │  Cloudflare R2               │ │
│  │  (Catalog, replicated)  │       │  (Lance Storage)             │ │
│  │                         │       │                              │ │
│  │  • dl_data_file         │──────►│  s3://zenith/lance/          │ │
│  │  • dl_snapshot          │ paths │    └── {ecosystem}/          │ │
│  │  • dl_metadata          │       │        └── {package}/        │ │
│  │                         │       │            └── {version}/    │ │
│  │  Visibility scoping:    │       │                └── *.lance   │ │
│  │  public/team/private    │       │                              │ │
│  │                         │       │  Written by: lancedb crate   │ │
│  │  Clerk JWT (JWKS)       │       │  Read by: DuckDB lance ext   │ │
│  └─────────────────────────┘       └──────────────────────────────┘ │
│           ▲                                       ▲                  │
│           │ embedded replica                      │ lance ext        │
│  ┌────────┴──────────────────────────────────────┴───────────────┐  │
│  │  Local Client                                                  │  │
│  │                                                                │  │
│  │  Turso replica (.zenith/catalog.db)                           │  │
│  │    → offline catalog access                                    │  │
│  │    → visibility-scoped path discovery                          │  │
│  │                                                                │  │
│  │  DuckDB (in-memory or cache)                                   │  │
│  │    → INSTALL lance FROM community; LOAD lance;                 │  │
│  │    → lance_vector_search(path_from_catalog, ...)               │  │
│  │    → lance_fts(path_from_catalog, ...)                         │  │
│  │    → lance_hybrid_search(path_from_catalog, ...)               │  │
│  │                                                                │  │
│  │  Local DuckDB file (.zenith/source_files.duckdb)               │  │
│  │    → source code cache for znt grep                            │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 3. Three-Tier Index Model

All Lance datasets live on the **same R2 bucket**. The Turso catalog controls who can discover what:

| Tier | Visibility | Who Writes | Who Reads | Use Case |
|------|-----------|------------|-----------|----------|
| **Global (public)** | `visibility = 'public'` | Any authenticated user (crowdsource) | Everyone | Open-source packages (tokio, axum, serde) |
| **Team** | `visibility = 'team'` | Team members (`team_id = org_id`) | Team members only | Internal shared libraries, company frameworks |
| **Private** | `visibility = 'private'` | Package owner (`owner_id = sub`) | Owner only | User's own project code indexed as a first-class package |

### Discovery Query (the WHERE clause every `znt search` runs)

```sql
-- Turso catalog query: get Lance paths for packages the user can access
SELECT path, ecosystem, package, version, record_count
FROM dl_data_file
WHERE visibility = 'public'
   OR (visibility = 'team' AND team_id = :org_id)
   OR (visibility = 'private' AND owner_id = :user_id)
ORDER BY package;
```

The `:org_id` and `:user_id` come from the Clerk JWT claims (`org_id` and `sub`).

---

## 4. Turso Catalog (DuckLake-Inspired)

The catalog design is inspired by DuckLake's SQLite catalog schema (22 tables for snapshots, data files, schema evolution). We implement the subset we need in Turso, with additions for visibility scoping and Lance-specific metadata.

### Why Turso Instead of DuckLake's SQLite Extension

| Factor | DuckLake SQLite Extension | Turso Catalog |
|--------|--------------------------|---------------|
| Multi-user | Single writer (SQLite file lock) | Multi-user via Turso Cloud (Clerk JWT) |
| Replicas | None (local file only) | Embedded replicas on every client |
| Auth | None | Clerk JWT via JWKS |
| Visibility | None | public/team/private scoping |
| Data format | Parquet only | Lance (we manage writes ourselves) |
| Writes | DuckDB sqlite extension (bypasses Turso replication) | libsql client (proper replication) |

### Catalog Tables

#### dl_metadata

Global configuration for the catalog instance.

```sql
CREATE TABLE dl_metadata (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);

-- Example rows:
-- ('version', '0.1')
-- ('data_path', 's3://zenith/lance/')
-- ('file_format', 'lance')
-- ('created_by', 'zenith-cli 0.1.0')
```

#### dl_snapshot

Versioned history of catalog changes. Every write operation creates a new snapshot.

```sql
CREATE TABLE dl_snapshot (
    snapshot_id INTEGER PRIMARY KEY,
    snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
    schema_version INTEGER DEFAULT 1,
    description TEXT
);

-- Example rows:
-- (0, '2026-02-09T00:00:00Z', 1, 'initial')
-- (1, '2026-02-09T01:00:00Z', 1, 'registered rust/tokio/1.49.0 (public)')
-- (2, '2026-02-09T02:00:00Z', 1, 'registered rust/internal-sdk/2.0.0 (team)')
```

#### dl_data_file

Tracks every Lance dataset on R2. This is the core table — it maps packages to Lance paths with visibility.

```sql
CREATE TABLE dl_data_file (
    file_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,           -- 'api_symbols' or 'doc_chunks'
    snapshot_id INTEGER NOT NULL,
    path TEXT NOT NULL,                 -- Lance dataset path on R2
    file_format TEXT DEFAULT 'lance',
    record_count INTEGER DEFAULT 0,
    -- Package identity
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    -- Visibility scoping (Clerk JWT claims)
    visibility TEXT NOT NULL DEFAULT 'public',  -- 'public', 'team', 'private'
    owner_id TEXT,                              -- Clerk user_id (for 'private')
    team_id TEXT,                               -- Clerk org_id (for 'team')
    -- Provenance
    indexed_by TEXT NOT NULL,                   -- Clerk user_id of who indexed it
    created_at TEXT DEFAULT (datetime('now')),
    -- Package metadata
    repo_url TEXT,
    description TEXT,
    license TEXT
);
```

### Validated Catalog Operations

All operations below were validated in spike 0.20 (9/9 tests pass) using live Turso Cloud.

#### Register a new package (after lancedb write)

```sql
-- 1. Check if already indexed (crowdsource dedup)
SELECT 1 FROM dl_data_file
WHERE ecosystem = 'rust' AND package = 'tokio' AND version = '1.49.0'
  AND table_name = 'api_symbols';

-- 2. If not exists: register (PRIMARY KEY dedup if concurrent)
INSERT INTO dl_data_file
    (table_name, snapshot_id, path, record_count,
     ecosystem, package, version, visibility, indexed_by)
VALUES
    ('api_symbols', :next_snapshot, 's3://zenith/lance/rust/tokio/1.49.0/symbols.lance',
     1234, 'rust', 'tokio', '1.49.0', 'public', :user_id);

-- 3. Record snapshot
INSERT INTO dl_snapshot (snapshot_id, description)
VALUES (:next_snapshot, 'registered rust/tokio/1.49.0 (public)');
```

#### Concurrent write dedup (validated in spike 0.20 L1)

```
User A: INSERT INTO dl_data_file (..., 'tokio', '1.49.0', ...)  → SUCCESS
User B: INSERT INTO dl_data_file (..., 'tokio', '1.49.0', ...)  → SQLITE_CONSTRAINT (first writer wins)
User B: checks if exists → already indexed → skips upload, uses existing Lance data
```

#### Visibility-scoped discovery (validated in spike 0.20 J3, K1, K2)

```sql
-- Team member of org_acme sees public + their team's packages
SELECT path, package, visibility FROM dl_data_file
WHERE visibility = 'public'
   OR (visibility = 'team' AND team_id = 'org_39PSbEI9mVoLgBQWuASKeltV7S9');

-- Result:
-- /tmp/.../rust_tokio_1.49.0.lance     | tokio        | public
-- /tmp/.../rust_internal_sdk_2.0.0.lance | internal-sdk | team
```

#### Embedded replica for offline catalog (validated in spike 0.20 J2)

```rust
// Rust (libsql): create embedded replica, sync, query offline
let db = Builder::new_remote_replica(
    ".zenith/catalog.db",
    turso_url,
    clerk_jwt,  // Clerk JWT as auth token (JWKS validated by Turso)
)
.read_your_writes(true)
.build().await?;

db.sync().await?;

// Query locally — no network needed
let paths = conn.query(
    "SELECT path FROM dl_data_file WHERE visibility = 'public' AND ecosystem = ?",
    ["rust"],
).await?;
```

---

## 5. Lance Storage on R2

### R2 Layout

```
s3://zenith/lance/
└── {ecosystem}/
    └── {package}/
        └── {version}/
            ├── symbols.lance         # api_symbols with embeddings
            └── doc_chunks.lance      # documentation chunks with embeddings
```

Examples:
```
s3://zenith/lance/rust/tokio/1.49.0/symbols.lance
s3://zenith/lance/rust/axum/0.8.0/symbols.lance
s3://zenith/lance/acme-corp/internal-sdk/2.0.0/symbols.lance   (team)
s3://zenith/lance/jdoe/my-app/0.1.0/symbols.lance              (private)
```

### Writing Lance Datasets (Production Path)

Validated in spike 0.19 test M1. This is the **only** write path in production:

```rust
use serde::{Serialize, Deserialize};
use serde_arrow::schema::{SchemaLike, TracingOptions};
use arrow_schema::{DataType, Field, FieldRef};
use arrow_array::RecordBatchIterator;
use zen_core::arrow_serde;
use chrono::{DateTime, Utc};

// 1. Define the Rust struct (production data type)
#[derive(Serialize, Deserialize)]
struct ApiSymbol {
    id: String,
    ecosystem: String,
    package: String,
    version: String,
    file_path: String,
    kind: String,
    name: String,
    signature: Option<String>,
    doc_comment: Option<String>,
    line_start: Option<i32>,
    line_end: Option<i32>,
    visibility: Option<String>,
    is_async: Option<bool>,
    is_unsafe: Option<bool>,
    attributes: Option<String>,
    embedding: Vec<f32>,           // 384-dim from fastembed
    #[serde(with = "arrow_serde::timestamp_micros_utc_option")]
    created_at: Option<DateTime<Utc>>,
}

// 2. serde_arrow: trace schema + override embedding to FixedSizeList(384)
let mut fields = Vec::<FieldRef>::from_type::<ApiSymbol>(
    TracingOptions::default()
)?;
fields = fields.into_iter().map(|f| {
    if f.name() == "embedding" {
        Arc::new(Field::new("embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)), 384),
            false))
    } else { f }
}).collect();

// 3. Serialize to RecordBatch
let batch = serde_arrow::to_record_batch(&fields, &symbols)?;

// 4. Write to R2 via lancedb
let db = lancedb::connect("s3://zenith/lance/rust/tokio/1.49.0")
    .storage_option("aws_access_key_id", &key_id)
    .storage_option("aws_secret_access_key", &secret)
    .storage_option("aws_endpoint", &endpoint)
    .storage_option("aws_region", "auto")
    .storage_option("aws_virtual_hosted_style_request", "false")
    .execute().await?;

let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
let tbl = db.create_table("symbols", Box::new(batches)).execute().await?;

// 5. Create search indexes
tbl.create_index(&["embedding"], lancedb::index::Index::Auto).execute().await?;
tbl.create_index(&["doc_comment"],
    lancedb::index::Index::FTS(FtsIndexBuilder::default())
).execute().await?;

// 6. Register in Turso catalog
conn.execute(
    "INSERT INTO dl_data_file (table_name, snapshot_id, path, record_count,
     ecosystem, package, version, visibility, indexed_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    params!["api_symbols", next_snapshot,
        "s3://zenith/lance/rust/tokio/1.49.0/symbols.lance",
        symbols.len(), "rust", "tokio", "1.49.0", "public", user_id],
).await?;
```

### Incremental Updates (validated in spike 0.19 H5)

```rust
// Add new symbols to an existing Lance dataset
let tbl = db.open_table("symbols").execute().await?;
let new_batches = RecordBatchIterator::new(vec![Ok(new_batch)], schema);
tbl.add(Box::new(new_batches)).execute().await?;
// count: 100 initial + 50 added = 150 total
```

### Arrow Version Note

`duckdb 1.4` uses `arrow 56`, `lancedb 0.26` uses `arrow 57`. Both coexist in the same binary. In production, data flows as Rust structs → `serde_arrow` → arrow-57 → `lancedb`, so no version bridging is needed. A value-based bridge exists in spike 0.19 (tests I1/I2) for the DuckDB extraction path, which is exploratory only.

---

## 6. DuckDB Query Engine

DuckDB is **read-only** in this architecture. It queries Lance datasets whose paths come from the Turso catalog.

### Required Extensions

```sql
INSTALL lance FROM community;
LOAD lance;
INSTALL httpfs;
LOAD httpfs;
```

### Search Functions (validated in spikes 0.18 + 0.19)

```sql
-- Vector search (returns _distance, smaller = closer)
SELECT name, signature, _distance
FROM lance_vector_search(
    's3://zenith/lance/rust/tokio/1.49.0/symbols.lance',
    'embedding',
    :query_embedding::FLOAT[384],
    k=20
)
ORDER BY _distance ASC;

-- Full-text search (BM25, returns _score, larger = better)
SELECT name, doc_comment, _score
FROM lance_fts(
    's3://zenith/lance/rust/tokio/1.49.0/symbols.lance',
    'doc_comment',
    'spawn async task',
    k=10
)
ORDER BY _score DESC;

-- Hybrid search (vector + FTS combined)
SELECT name, signature, _hybrid_score
FROM lance_hybrid_search(
    's3://zenith/lance/rust/tokio/1.49.0/symbols.lance',
    'embedding', :query_embedding::FLOAT[384],
    'doc_comment', 'spawn async task',
    k=10, alpha=0.5, oversample_factor=4
)
ORDER BY _hybrid_score DESC;
```

### R2 Credentials for DuckDB

DuckDB's lance extension uses `CREATE SECRET` for S3-compatible access:

```sql
CREATE SECRET r2 (
    TYPE s3,
    KEY_ID :access_key_id,
    SECRET :secret_access_key,
    ENDPOINT :account_id || '.r2.cloudflarestorage.com',
    URL_STYLE 'path'
);
```

Note: `lancedb` (Rust crate, for writes) uses `storage_option()` / AWS env vars. DuckDB lance extension (for reads) uses `CREATE SECRET`. Both work with the same R2 credentials.

---

## 7. Data Schemas

### api_symbols (Lance dataset)

Tree-sitter-extracted public API symbols from package source code.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `Utf8` | Deterministic hash: `sha256(ecosystem:package:version:file_path:kind:name)` |
| `ecosystem` | `Utf8` | `rust`, `npm`, `hex`, `pypi`, `go` |
| `package` | `Utf8` | Package name as published in registry |
| `version` | `Utf8` | Exact version indexed |
| `file_path` | `Utf8` | Relative path within repo |
| `kind` | `Utf8` | Symbol type: `function`, `struct`, `enum`, `trait`, `class`, etc. |
| `name` | `Utf8` | Symbol name as it appears in source |
| `signature` | `Utf8` (nullable) | Full signature line (no body) |
| `source` | `Utf8` (nullable) | Source code body |
| `doc_comment` | `Utf8` (nullable) | Extracted doc comment |
| `line_start` | `Int32` (nullable) | Start line in source file |
| `line_end` | `Int32` (nullable) | End line in source file |
| `visibility` | `Utf8` (nullable) | `pub`, `pub(crate)`, `private`, `export` |
| `is_async` | `Boolean` (nullable) | Whether the symbol is async |
| `is_unsafe` | `Boolean` (nullable) | Whether the symbol is unsafe |
| `is_error_type` | `Boolean` (nullable) | Whether it's an error type |
| `returns_result` | `Boolean` (nullable) | Whether it returns Result |
| `attributes` | `Utf8` (nullable) | JSON array of attributes/decorators |
| `embedding` | `FixedSizeList(Float32, 384)` | fastembed vector |
| `created_at` | `Int64` (nullable) | Timestamp as microseconds since epoch |

### doc_chunks (Lance dataset)

Chunked documentation text from READMEs, guides, and doc files.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `Utf8` | Deterministic hash |
| `ecosystem` | `Utf8` | Package ecosystem |
| `package` | `Utf8` | Package name |
| `version` | `Utf8` | Package version |
| `chunk_index` | `Int32` | Sequential index within source file |
| `title` | `Utf8` (nullable) | Section heading |
| `content` | `Utf8` | Chunk text content |
| `source_file` | `Utf8` (nullable) | Relative path: `README.md`, `docs/guide.md` |
| `format` | `Utf8` (nullable) | `md`, `rst`, `txt` |
| `embedding` | `FixedSizeList(Float32, 384)` | fastembed vector |

---

## 8. Indexing Pipeline

```
1. Check Catalog (crowdsource dedup)
   Turso: SELECT 1 FROM dl_data_file WHERE ecosystem=? AND package=? AND version=?
   → If exists: skip (another user already indexed it)

2. Clone Repository
   git clone --depth 1 <repo_url> /tmp/zenith-index/<pkg>

3. Parse with Tree-Sitter (ast-grep)
   For each source file: detect language → parse → extract symbols → build ApiSymbol structs

4. Extract Documentation
   Find README.md, docs/*, CHANGELOG.md → chunk by section → build DocChunk structs

5. Generate Embeddings
   Batch ApiSymbol.signature + DocChunk.content through fastembed (384-dim)

6. Write Lance to R2 (serde_arrow production path)
   serde_arrow::to_record_batch(&fields, &symbols) → lancedb::create_table() → R2
   Create vector index (IVF-PQ, needs >= 256 rows) + FTS index (BM25)

7. Register in Turso Catalog
   INSERT INTO dl_data_file (...) — if SQLITE_CONSTRAINT, another user won the race → skip

8. Store Source Files (for znt grep)
   INSERT INTO local source_files DuckDB table (for grep, not uploaded to R2)

9. Update Project Dependencies
   Turso: UPDATE project_dependencies SET indexed = TRUE WHERE name = <pkg>

10. Cleanup
    rm -rf /tmp/zenith-index/<pkg>
```

---

## 9. Search Flows

### `znt search "spawn async task"` (the complete flow)

```
1. Turso replica: SELECT path FROM dl_data_file
   WHERE (visibility = 'public'
       OR (visibility = 'team' AND team_id = :jwt.org_id)
       OR (visibility = 'private' AND owner_id = :jwt.sub))
   AND ecosystem = 'rust'
   AND package IN (:user_deps)

2. For each path from Turso:
   DuckDB: lance_hybrid_search(path, 'embedding', :query_vec,
       'doc_comment', 'spawn async task', k=10, alpha=0.5)

3. Merge results across all packages, sort by _hybrid_score

4. Return top N to the LLM
```

### `znt search --mode recursive` (symbolic recursion flow)

```
1. Build metadata-only root plan
   - package/file counts
   - byte budgets
   - candidate scopes

2. Select slices via symbolic handles
   - AST kinds (functions/structs/traits/enums)
   - doc-comment keyword filtering
   - source snippets by path + line ranges

3. Execute budgeted recursive sub-queries
   - max_depth
   - max_chunks
   - max_bytes_per_chunk
   - max_total_bytes

4. Build categorized reference graph
   - same_module
   - other_module_same_crate
   - other_crate_workspace
   - external (cross-workspace evidence, e.g. DataFusion)

5. Return search hits + signatures + optional summary payload
```

### `znt onboard` (check catalog before indexing)

```
1. Parse Cargo.toml → list of (ecosystem, package, version)

2. Turso: SELECT ecosystem, package, version FROM dl_data_file
   WHERE (ecosystem, package, version) IN (...)

3. For each dependency NOT in catalog:
   Run indexing pipeline (step 8 above)

4. For each dependency already in catalog:
   Skip — search will hit existing Lance data on R2
```

---

## 10. Authentication & Visibility

### Clerk JWT Template (`zenith_cli`)

Generated via `POST /v1/sessions/{session_id}/tokens/zenith_cli`:

```json
{
  "sub": "user_39PB2iMuMcpYGrHobrukpqZ8UjE",
  "org_id": "org_39PSbEI9mVoLgBQWuASKeltV7S9",
  "org_slug": "zenith-dev",
  "org_role": "org:admin",
  "org_permissions": [],
  "p": {
    "rw": {
      "ns": ["wrath-codes.zenith-dev"],
      "tables": { "all": { "data_read": true, "data_add": true, "..." } }
    }
  }
}
```

**Critical**: `org_permissions` must be `[]` (static empty array), not `{{org.permissions}}` (shortcode doesn't resolve). Without it, `clerk-rs`'s `ActiveOrganization` deserialization fails silently.

### Claim Usage

| Claim | Used For |
|-------|----------|
| `sub` | `owner_id` in dl_data_file (private visibility) |
| `org_id` | `team_id` in dl_data_file (team visibility) |
| `org_role` | Informational (admin/member), not used for filtering |
| `p` | Turso JWKS auth (Clerk JWT = libsql auth token) |

### Programmatic JWT Generation (validated in spike 0.20 J0)

```rust
// 1. Create a session via Clerk Backend API
let session = reqwest::Client::new()
    .post("https://api.clerk.com/v1/sessions")
    .header("Authorization", format!("Bearer {secret_key}"))
    .json(&json!({"user_id": user_id}))
    .send().await?
    .json::<Value>().await?;

// 2. Get JWT from template
let token = reqwest::Client::new()
    .post(format!("https://api.clerk.com/v1/sessions/{}/tokens/zenith_cli",
        session["id"].as_str().unwrap()))
    .header("Authorization", format!("Bearer {secret_key}"))
    .send().await?
    .json::<Value>().await?;

let jwt = token["jwt"].as_str().unwrap();

// 3. Validate with clerk-rs (JWKS)
let clerk_jwt = clerk_rs::validators::authorizer::validate_jwt(jwt, jwks_provider).await?;
let org = clerk_jwt.org.unwrap(); // ActiveOrganization { id, slug, role, permissions }
```

### Free vs Pro Boundary

| Feature | Free (local) | Pro (team) |
|---------|-------------|------------|
| Local indexing + search | Yes | Yes |
| Global public index (read) | Yes | Yes |
| Contribute to global index | Yes (with auth) | Yes |
| Team visibility | -- | Yes |
| Private code indexing | -- | Yes |
| Turso Cloud sync | -- | Yes |
| `znt team` commands | -- | Yes |

No license checks. No credentials = local mode. Valid Clerk JWT = authenticated mode. `org_id` present = team mode.

---

## 11. Source Files (znt grep)

Source files for `znt grep` stay in **local DuckDB** (not R2, not Turso). They're large, not shared, and don't need vector search.

```sql
-- Local DuckDB: .zenith/source_files.duckdb
CREATE TABLE source_files (
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT,
    size_bytes INTEGER,
    line_count INTEGER,
    PRIMARY KEY (ecosystem, package, version, file_path)
);
```

See [13-zen-grep-design.md](./13-zen-grep-design.md) for the full grep design.

---

## 12. Environment Variables

```bash
# Turso / libSQL
ZENITH_TURSO__URL=libsql://zenith-dev-wrath-codes.aws-us-east-1.turso.io
ZENITH_TURSO__AUTH_TOKEN=...          # Platform API token (dev/CI)
ZENITH_TURSO__PLATFORM_API_KEY=...    # For creating DBs programmatically
ZENITH_TURSO__ORG_SLUG=wrath-codes

# Cloudflare R2
ZENITH_R2__ACCOUNT_ID=...
ZENITH_R2__ACCESS_KEY_ID=...
ZENITH_R2__SECRET_ACCESS_KEY=...
ZENITH_R2__BUCKET_NAME=zenith

# AWS-style vars for Lance S3/R2 access (lancedb reads these directly)
AWS_ACCESS_KEY_ID=...                 # Same as R2 key
AWS_SECRET_ACCESS_KEY=...             # Same as R2 secret
AWS_ENDPOINT_URL=https://{account_id}.r2.cloudflarestorage.com
AWS_DEFAULT_REGION=auto

# Clerk
ZENITH_CLERK__SECRET_KEY=sk_test_...
ZENITH_CLERK__JWKS_URL=https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json
ZENITH_AUTH__TEST_TOKEN=...           # Org-scoped JWT for testing
```

---

## 13. Validated In

| Component | Spike | Tests | Key Findings |
|-----------|-------|-------|-------------|
| Clerk JWT + Turso JWKS | 0.17 | 14/14 | Clerk JWT = Turso auth token. Auth at builder time. Expired → `Sync("Unauthorized")`. |
| Lance on R2 (DuckDB ext) | 0.18 | 18/18 | `lance_vector_search()`, `lance_fts()`, `lance_hybrid_search()` all work on R2. PQ needs >= 256 rows. |
| Native lancedb writes | 0.19 | 10/10 | `serde_arrow` production path validated. `arrow_serde` adapters for DateTime. FixedSizeList(384) override needed. |
| Turso catalog + visibility | 0.20 | 9/9 | DuckLake-inspired tables in Turso. Clerk JWT drives visibility. Concurrent dedup via PRIMARY KEY. Two replicas coexist. Programmatic org-scoped JWT generation. |
| DuckLake SQLite catalog | CLI test | N/A | DuckLake + SQLite catalog + R2 Parquet works. Snapshots, time travel validated. But: can't use Turso replica (DuckDB writes bypass replication), Parquet only (no Lance), no PRIMARY KEY support. |

---

## Cross-References

- Turso entity data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
- Zen grep design: [13-zen-grep-design.md](./13-zen-grep-design.md)
- Clerk auth spike: [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md)
- R2 Lance spike: [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md)
- Native lancedb spike: [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md)
- Catalog visibility spike: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md)
- Old architecture (retired): [02-ducklake-data-model.md](./02-ducklake-data-model.md)

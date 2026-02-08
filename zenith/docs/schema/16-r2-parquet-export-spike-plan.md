# Zenith: R2 Parquet Export for Team Index -- Spike Plan

**Version**: 2026-02-08
**Status**: DONE -- 18/18 tests pass (13 Parquet + 5 Lance)
**Purpose**: Validate the R2 export pipeline for sharing indexed package data across team members without requiring per-user MotherDuck accounts. Validates both Parquet and Lance formats on R2. **Decision**: Use Lance format for both shared team index and local index — persistent vector indexes, native BM25 FTS, and hybrid search.
**Spike ID**: 0.18
**Crate**: zen-lake (spike file, reuses existing DuckDB + R2 infrastructure)
**Blocks**: Phase 9 (tasks 9.13-9.16: R2 export, shared reader, manifest, `znt export` command)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Background & Prior Art](#2-background--prior-art)
3. [Architecture](#3-architecture)
4. [What We're Validating](#4-what-were-validating)
5. [Dependencies](#5-dependencies)
6. [Spike Tests](#6-spike-tests)
7. [Evaluation Criteria](#7-evaluation-criteria)
8. [What This Spike Does NOT Test](#8-what-this-spike-does-not-test)
9. [Success Criteria](#9-success-criteria)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Motivation

Zenith Phase 9 (Team & Pro) needs a shared package index so team members don't re-index the same packages (tokio, axum, serde, etc.). The MotherDuck sharing model has a critical constraint: **consumers need MotherDuck accounts**, and **DuckLake databases are single-writer**. Requiring every team member to create a MotherDuck account is friction we want to avoid.

**Solution**: The admin (writer) indexes packages into DuckLake and periodically exports the three index tables (`api_symbols`, `doc_chunks`, `indexed_packages`) as Parquet files to Cloudflare R2. Team members (readers) query the Parquet files directly from R2 using local DuckDB + `httpfs` -- no MotherDuck account needed.

This spike validates that the export/read pipeline works, maintains vector search capability, has acceptable performance, and handles the manifest/versioning lifecycle.

### Why R2 Over MotherDuck Shares

| Factor | MotherDuck Share | R2 Parquet |
|--------|-----------------|------------|
| Reader needs account | Yes (MotherDuck) | No (R2 is S3-compatible, creds in config) |
| Reader cost | $250/mo Business plan | R2 egress is free |
| Write access | Single writer only | Single writer exports, readers use read-only Parquet |
| Update latency | ~1 min auto-update | Export on-demand (`znt export`) |
| Vector search | DuckDB VSS/Lance | `array_cosine_similarity()` on Parquet `FLOAT[]` columns |
| Offline support | Requires MotherDuck connection | Can cache Parquet locally |

### Prior Art: aether R2 + DuckDB

Aether's `test_r2_ducklake.rs` spike validated DuckDB → R2 Parquet writes and reads. Key findings from spike 0.5:
- `httpfs` extension works with R2 (S3-compatible)
- `CREATE SECRET ... (TYPE s3, ...)` for R2 credentials
- `COPY ... TO 's3://bucket/path.parquet'` for writes
- `SELECT * FROM read_parquet('s3://bucket/path.parquet')` for reads
- `FLOAT[]` columns survive Parquet roundtrip (no fixed-size `FLOAT[384]` in Parquet)

---

## 2. Background & Prior Art

### DuckLake Table Schema (from 02-ducklake-data-model.md)

```sql
-- api_symbols: indexed API symbols with embeddings
CREATE TABLE api_symbols (
    id VARCHAR PRIMARY KEY,
    ecosystem VARCHAR NOT NULL,
    package VARCHAR NOT NULL,
    version VARCHAR NOT NULL,
    file_path VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    signature VARCHAR,
    source TEXT,
    doc_comment TEXT,
    line_start INTEGER,
    line_end INTEGER,
    visibility VARCHAR DEFAULT 'public',
    is_async BOOLEAN DEFAULT FALSE,
    is_unsafe BOOLEAN DEFAULT FALSE,
    is_error_type BOOLEAN DEFAULT FALSE,
    returns_result BOOLEAN DEFAULT FALSE,
    metadata JSON,
    attributes TEXT,
    embedding FLOAT[],
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- doc_chunks: documentation sections with embeddings
CREATE TABLE doc_chunks (
    id VARCHAR PRIMARY KEY,
    ecosystem VARCHAR NOT NULL,
    package VARCHAR NOT NULL,
    version VARCHAR NOT NULL,
    file_path VARCHAR NOT NULL,
    section_title VARCHAR,
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    embedding FLOAT[],
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- indexed_packages: registry of what's been indexed
CREATE TABLE indexed_packages (
    ecosystem VARCHAR NOT NULL,
    package VARCHAR NOT NULL,
    version VARCHAR NOT NULL,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    symbol_count INTEGER DEFAULT 0,
    chunk_count INTEGER DEFAULT 0,
    source_cached BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (ecosystem, package, version)
);
```

### R2 Bucket

Already created: `s3://zenith/` (us-east-1, same region as MotherDuck).
R2 credentials in `.env`: `ZENITH_R2__ACCESS_KEY_ID`, `ZENITH_R2__SECRET_ACCESS_KEY`, `ZENITH_R2__ACCOUNT_ID`, `ZENITH_R2__BUCKET`.

### Export Path Convention

```
s3://zenith/packages/{org_id}/
├── api_symbols.parquet
├── doc_chunks.parquet
├── indexed_packages.parquet
└── manifest.json
```

`manifest.json`:
```json
{
  "schema_version": 1,
  "exported_at": "2026-02-08T12:00:00Z",
  "org_id": "org_abc123",
  "packages": [
    {"ecosystem": "rust", "package": "tokio", "version": "1.49.0", "symbol_count": 1234},
    {"ecosystem": "rust", "package": "axum", "version": "0.8.0", "symbol_count": 567}
  ],
  "total_symbols": 1801,
  "total_chunks": 342
}
```

---

## 3. Architecture

```
Admin CLI (writer)                         Team Member CLI (reader)
     │                                          │
     ├── zen-lake (DuckLake)                    ├── zen-lake (local DuckDB)
     │   indexes packages                       │   no DuckLake, no MotherDuck
     │                                          │
     ├── znt export                             ├── znt search / znt install --shared
     │   COPY tables TO R2 Parquet              │   read_parquet('s3://zenith/packages/{org}/...')
     │   write manifest.json                    │   array_cosine_similarity() for vector search
     │                                          │
     └── R2: s3://zenith/packages/{org_id}/     └── R2: s3://zenith/packages/{org_id}/
         ├── api_symbols.parquet                    (reads via httpfs, R2 creds in config)
         ├── doc_chunks.parquet
         ├── indexed_packages.parquet
         └── manifest.json
```

---

## 4. What We're Validating

7 hypotheses that must hold for the R2 Parquet export architecture:

| # | Hypothesis | Risk if wrong |
|---|---|---|
| H1 | `COPY table TO 's3://...' (FORMAT PARQUET)` exports DuckLake tables to R2 correctly | Can't export index to R2 |
| H2 | `read_parquet('s3://...')` reads R2 Parquet files into local DuckDB | Readers can't access shared index |
| H3 | `FLOAT[]` embedding columns survive Parquet roundtrip and `array_cosine_similarity()` works on the result | Vector search broken over shared index |
| H4 | `JSON` metadata columns survive Parquet roundtrip | Rich metadata lost on export |
| H5 | Export + read performance is acceptable (export 10K symbols < 30s, query < 2s) | Shared index too slow to be useful |
| H6 | Incremental export works (export only packages indexed since last export) | Full re-export every time is too slow |
| H7 | Manifest JSON tracks exported state and readers can check freshness | No way to know if cached Parquet is stale |

---

## 5. Dependencies

No new workspace dependencies. All already in workspace:

| Crate | Version | Role in spike |
|-------|---------|--------------|
| `duckdb` | workspace (1.4, bundled) | DuckLake tables, Parquet export/read |
| `serde` | workspace | Manifest serialization |
| `serde_json` | workspace | Manifest JSON read/write |
| `chrono` | workspace | Timestamps in manifest |
| `reqwest` | workspace | S3/R2 manifest upload (if not through DuckDB) |
| `tempfile` | workspace | Temp directories for local DuckDB |

---

## 6. Spike Tests

**File**: `zenith/crates/zen-lake/src/spike_r2_parquet.rs`

### Part A: Parquet Export to R2 (3 tests)

| # | Test | Validates |
|---|------|-----------|
| A1 | `spike_export_symbols_to_r2_parquet` | Create local DuckDB with `api_symbols` table. Insert 100 test symbols with embeddings (`FLOAT[]` 384-dim), metadata (JSON), and all fields. Export via `COPY api_symbols TO 's3://zenith/packages/test_spike/api_symbols.parquet' (FORMAT PARQUET)`. Verify file created (query R2 file listing or read back). Skip if R2 creds missing. (H1) |
| A2 | `spike_export_doc_chunks_to_r2_parquet` | Same as A1 but for `doc_chunks` table with embeddings and content. (H1) |
| A3 | `spike_export_indexed_packages_to_r2_parquet` | Same for `indexed_packages` table (no embeddings, just metadata). (H1) |

### Part B: Parquet Read from R2 (3 tests)

| # | Test | Validates |
|---|------|-----------|
| B1 | `spike_read_symbols_from_r2_parquet` | Open a fresh local DuckDB (no DuckLake). Install/load `httpfs`. Set R2 credentials via `CREATE SECRET`. Query: `SELECT * FROM read_parquet('s3://zenith/packages/test_spike/api_symbols.parquet') LIMIT 10`. Verify all columns present, types correct. (H2) |
| B2 | `spike_embedding_roundtrip_cosine_similarity` | Read symbols from R2 Parquet. Generate a test query embedding (384-dim random). Execute `SELECT name, array_cosine_similarity(embedding, ?::FLOAT[384]) AS score FROM read_parquet('s3://...') ORDER BY score DESC LIMIT 5`. Verify scores are in [0, 1] range and results are ranked. (H3) |
| B3 | `spike_metadata_json_roundtrip` | Read symbols from R2 Parquet. Verify `metadata` column is accessible via JSON operators (`metadata->>'is_async'`). Verify round-trip: original JSON values match what was exported. (H4) |

### Part C: Performance (2 tests)

| # | Test | Validates |
|---|------|-----------|
| C1 | `spike_export_10k_symbols_performance` | Insert 10,000 test symbols with embeddings. Time the `COPY ... TO 's3://...'` export. Assert < 30 seconds. Measure file size. (H5) |
| C2 | `spike_query_10k_symbols_performance` | Query the 10K-symbol Parquet file from R2. Time a vector search (cosine similarity, top 10). Time a text filter (`WHERE name LIKE '%spawn%'`). Assert each < 2 seconds. (H5) |

### Part D: Incremental Export (2 tests)

| # | Test | Validates |
|---|------|-----------|
| D1 | `spike_incremental_export_by_timestamp` | Insert 100 symbols at T1. Export. Insert 50 more at T2. Export only rows where `created_at > T1`: `COPY (SELECT * FROM api_symbols WHERE created_at > ?) TO 's3://.../api_symbols_delta.parquet'`. Verify delta file has exactly 50 rows. (H6) |
| D2 | `spike_merge_base_and_delta_parquet` | Read both base and delta Parquet files via `SELECT * FROM read_parquet(['s3://.../api_symbols.parquet', 's3://.../api_symbols_delta.parquet'])`. Verify total count = 150. Verify dedup works if we use `UNION` or `QUALIFY`. (H6) |

### Part E: Manifest (2 tests)

| # | Test | Validates |
|---|------|-----------|
| E1 | `spike_write_manifest_to_r2` | Construct manifest JSON (schema_version, exported_at, packages list, totals). Upload to `s3://zenith/packages/test_spike/manifest.json` via DuckDB `COPY` or direct S3 PUT. Verify readable. (H7) |
| E2 | `spike_read_manifest_check_freshness` | Read manifest from R2. Parse JSON. Check `exported_at` against current time. Determine if cache is stale (> 1 hour old). List packages in manifest. (H7) |

**Total: 12 tests**

---

## 7. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| Parquet export to R2 | **Critical** | Tests A1-A3: all three tables export successfully |
| Parquet read from R2 | **Critical** | Tests B1-B3: all columns readable, types correct |
| Embedding roundtrip + cosine search | **Critical** | Test B2: `array_cosine_similarity()` works on R2 Parquet |
| JSON metadata roundtrip | **High** | Test B3: JSON operators work on Parquet-read data |
| Export performance | **High** | Test C1: 10K symbols < 30s |
| Query performance | **High** | Test C2: vector + text queries < 2s each |
| Incremental export | **Medium** | Tests D1-D2: delta export and merge work |
| Manifest lifecycle | **Medium** | Tests E1-E2: write, read, freshness check |

---

## 8. What This Spike Does NOT Test

- **Real package data** -- spike uses synthetic test data, not actual parsed packages. Real indexing pipeline is Phase 3.
- **DuckLake → Parquet export** -- spike creates tables in plain DuckDB, not DuckLake. If DuckLake adds constraints on `COPY TO`, we'd discover that in Phase 9 integration.
- **Multi-org isolation** -- spike uses a single test org path. Multi-org path isolation is Phase 9 task 9.13.
- **R2 signed URLs for readers** -- spike uses direct R2 credentials. If we want credential-less reader access, we'd need signed URLs or a proxy. Deferred.
- **Parquet partitioning** -- partitioning by ecosystem (e.g., `s3://zenith/packages/{org}/rust/api_symbols.parquet`) could improve performance but adds complexity. Deferred.
- **Lance format** -- spike 0.5 validated Lance as an alternative. R2 Parquet is simpler and sufficient for the team index use case.
- **Cache invalidation** -- readers checking manifest freshness is tested, but automatic re-download is Phase 9.

---

## 9. Success Criteria

- **All three tables export to R2 Parquet and read back correctly** (tests A1-A3, B1 pass)
- **Vector search works over R2 Parquet** (`array_cosine_similarity()`, test B2 passes)
- **JSON metadata survives roundtrip** (test B3 passes)
- **Export performance < 30s for 10K symbols** (test C1 passes)
- **Query performance < 2s for 10K symbols** (test C2 passes)
- **Incremental export produces correct delta files** (tests D1-D2 pass)
- **Manifest writes to and reads from R2** (tests E1-E2 pass)
- **All 12 tests pass** (some may be skipped if R2 creds missing)

---

## 10. Post-Spike Actions

### If Spike Passes (Expected Path)

| Doc | Update |
|-----|--------|
| `07-implementation-plan.md` | Add spike 0.18 to Phase 0 table with results. Confirm R2 Parquet architecture for Phase 9. |
| `05-crate-designs.md` | Add `ZenLake::export_to_r2()`, `ZenLake::open_shared_reader()` to zen-lake design |
| `02-ducklake-data-model.md` | Add export path convention and manifest schema |
| `INDEX.md` | Add doc 16 to document map |

### If Embedding Roundtrip Fails (Fallback A)

- Store embeddings separately in Lance format on R2 (spike 0.5 validated Lance)
- Export other columns as Parquet, embeddings as Lance
- Readers use `lance_vector_search()` for vectors, `read_parquet()` for metadata
- More complex but preserves vector search quality

### If R2 Performance Too Slow (Fallback B)

- Parquet partitioning by ecosystem: `s3://zenith/packages/{org}/rust/api_symbols.parquet`
- Row-group filtering by package name (requires sorted Parquet)
- Local Parquet cache: download once, query locally, re-download on manifest change

### If JSON Metadata Doesn't Roundtrip (Fallback C)

- Flatten metadata into top-level columns (one column per metadata field)
- Or store metadata as `VARCHAR` (JSON string) instead of DuckDB `JSON` type
- Minor schema change, unlikely to be needed

---

## Cross-References

- DuckDB + R2 spike: [spike_duckdb_vss.rs](../../crates/zen-lake/src/spike_duckdb_vss.rs)
- DuckLake data model: [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- R2 config: [zen-config/src/r2.rs](../../crates/zen-config/src/r2.rs)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)

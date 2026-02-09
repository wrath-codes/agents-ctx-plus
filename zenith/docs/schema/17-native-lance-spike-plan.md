# Zenith: Native lancedb Writes + Arrow Bridge -- Spike Plan

**Version**: 2026-02-08
**Status**: DONE -- 9/9 tests pass
**Purpose**: Validate native `lancedb` Rust crate for writing Lance datasets to R2 (replacing DuckDB `COPY TO (FORMAT lance)`). Validate the Arrow C FFI bridge between `duckdb` (arrow 56) and `lancedb` (arrow 57) for zero-copy RecordBatch transfer. This spike establishes the **write path** for the unified index architecture (global + team + private).
**Spike ID**: 0.19
**Crate**: zen-lake (spike file, reuses existing DuckDB + R2 infrastructure)
**Blocks**: Phase 9 (crowdsourced index writes, `znt install` upload, `znt index` private code)

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

Spike 0.18 validated that Lance datasets on R2 work for vector search, FTS, and hybrid search — but it used DuckDB's Lance extension (`COPY TO (FORMAT lance)`) for writes. This is SQL string building through DuckDB to write a format that has a **native Rust library** (`lancedb`).

For the production architecture, we need:

1. **Type-safe Rust writes** — `lancedb::create_table()` with `RecordBatch`, not SQL strings
2. **Explicit index creation** — `tbl.create_index(...)` for IVF-PQ vectors and BM25 FTS
3. **DuckDB → lancedb pipeline** — extract RecordBatches from DuckDB via `query_arrow()`, write to Lance via `lancedb`
4. **Arrow version bridge** — `duckdb 1.4` uses `arrow 56`, `lancedb 0.26` uses `arrow 57`. Need zero-copy FFI bridge.

### Why Native lancedb Over DuckDB Lance Extension for Writes

| Factor | DuckDB Lance Extension | Native `lancedb` Crate |
|--------|----------------------|----------------------|
| Write API | SQL strings (`COPY TO`) | Typed Rust API (`create_table()`, `add()`) |
| Index creation | Implicit (extension decides) | Explicit (`create_index()` with params) |
| Error handling | SQL error strings | Rust `Result<T, E>` |
| Streaming writes | Not supported (full COPY) | `RecordBatchIterator` (memory-efficient) |
| Incremental adds | Not supported | `tbl.add()` for delta updates |
| Format version | Extension's bundled lance | lance 2.0.0 (latest) |

DuckDB Lance extension remains the **read** path (`lance_vector_search()`, `lance_fts()`, `lance_hybrid_search()`). We're only replacing the **write** path.

### Arrow Version Mismatch

`duckdb 1.4.4` depends on `arrow ^56`. `lancedb 0.26.1` depends on `arrow ^57.2`. Cargo resolves both versions (they coexist as separate crates), but `RecordBatch` from arrow 56 is a different Rust type than `RecordBatch` from arrow 57.

**Solution explored**: Arrow C FFI was attempted first, but Rust treats `FFI_ArrowArray` from arrow 56 and arrow 57 as different types even with identical `#[repr(C)]` layout — `transmute` would be required which is fragile and `unsafe`.

**Actual solution**: Value-based reconstruction bridge (safe code, no `unsafe`). Reads values from arrow-56 arrays and constructs arrow-57 arrays. O(n) copy per batch but negligible vs network I/O.

**Production path** (discovered during spike): Data originates as Rust structs, not DuckDB tables. Use `serde_arrow` with `arrow-57` feature to go directly from Rust structs → arrow-57 RecordBatch → lancedb. No bridge needed. The DuckDB bridge is only for migrating existing data.

`unsafe_code = "forbid"` is preserved — no unsafe code required.

---

## 2. Background & Prior Art

### Spike 0.18 Results (Lance on R2)

- `COPY ... TO 's3://...' (FORMAT lance)` works for writes
- `lance_vector_search()` returns correct NN results (distance=0.0 self-match)
- `lance_fts()` BM25 works
- `lance_hybrid_search()` alpha=0.5 works
- Lance uses AWS credential chain (not DuckDB secrets)
- 10K symbols export in 5.1s, vector search in 3.0s

### lancedb Rust API (0.26.1, lance 2.0.0)

```rust
use lancedb::connect;
use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{Schema, Field, DataType};

// Connect to R2
let db = connect("s3://bucket/path")
    .storage_options([
        ("aws_access_key_id", key_id),
        ("aws_secret_access_key", secret),
        ("aws_endpoint", endpoint),
        ("aws_region", "auto"),
    ])
    .execute().await?;

// Write
let batches = RecordBatchIterator::new(vec![batch].into_iter().map(Ok), schema);
let tbl = db.create_table("symbols", Box::new(batches)).execute().await?;

// Create indexes
tbl.create_index(&["embedding"], lancedb::index::Index::Auto).execute().await?;

// Incremental add
tbl.add(new_batches).execute().await?;
```

### Arrow C FFI Bridge

```rust
/// Convert duckdb arrow-56 RecordBatch to lancedb arrow-57 RecordBatch.
/// Zero-copy via C Data Interface.
#[allow(unsafe_code)]
fn duckdb_batch_to_lance(
    batch: &duckdb::arrow::record_batch::RecordBatch,
) -> arrow_array::RecordBatch {
    use duckdb::arrow::ffi as ffi56;
    use arrow_array::ffi as ffi57;

    let data = batch.clone().into_data();  // no copy, just Arc bump
    let (ffi_array, ffi_schema) = ffi56::to_ffi(&data).expect("export to FFI");

    let data57 = unsafe { ffi57::from_ffi(ffi_array, &ffi_schema) }
        .expect("import from FFI");
    arrow_array::RecordBatch::from(data57)
}
```

---

## 3. Architecture

```
Production Write Path (what this spike validates):

  tree-sitter parse → ApiSymbol structs
       │
       ▼
  DuckDB (local staging table)
       │
       ├── stmt.query_arrow([]) → Vec<RecordBatch>  (arrow 56)
       │
       ├── Arrow C FFI bridge → Vec<RecordBatch>  (arrow 57)
       │
       ▼
  lancedb::create_table() / tbl.add()
       │
       ▼
  R2: s3://zenith/lance/{ecosystem}/{package}/{version}/symbols.lance

Production Read Path (unchanged from spike 0.18):

  DuckDB + lance extension
       │
       ├── lance_vector_search('s3://...', ...)
       ├── lance_fts('s3://...', ...)
       └── lance_hybrid_search('s3://...', ...)
```

---

## 4. What We're Validating

9 hypotheses:

| # | Hypothesis | Risk if wrong |
|---|---|---|
| H1 | Arrow C FFI bridge transfers RecordBatch from arrow 56 to arrow 57 zero-copy | Can't pipe DuckDB query results to lancedb |
| H2 | `lancedb` writes api_symbols schema (20 cols, `FixedSizeList(Float32, 384)`) to local Lance | Schema incompatibility blocks local indexing |
| H3 | `lancedb` writes to R2 via S3-compatible credentials | Can't upload to shared index |
| H4 | Explicit IVF-PQ vector + BM25 FTS indexes creatable via `create_index()` | Search performance degrades without indexes |
| H5 | `tbl.add()` works for incremental updates (delta adds) | Must re-export entire dataset on every update |
| H6 | DuckDB `query_arrow()` → FFI bridge → `lancedb::create_table()` pipeline works end-to-end (local) | Production indexing pipeline broken |
| H7 | Same pipeline works to R2 | Can't upload crowdsourced indexes |
| H8 | Indexes created by `lancedb` are readable by DuckDB lance extension (cross-process) | Writer and reader incompatible |
| H9 | `create_table()` with `exist_ok` handles pre-existing datasets | Concurrent indexers conflict |

---

## 5. Dependencies

### New workspace dependencies (added to `zenith/Cargo.toml`)

| Crate | Version | Role in spike |
|-------|---------|--------------|
| `lancedb` | 0.26, features = ["aws"] | Native Lance dataset writes to local + R2 |
| `arrow-array` | 57 | RecordBatch construction (lancedb's arrow version) |
| `arrow-schema` | 57 | Schema/Field/DataType for lance tables |
| `arrow-ipc` | 57 | Fallback bridge if FFI has issues |

### Changed workspace dependencies

| Crate | Old | New | Reason |
|-------|-----|-----|--------|
| `object_store` | 0.13 | 0.12 | Align with lancedb 0.26 / lance 2.0 (transitive dep) |

### New workspace dependencies (added for production path)

| Crate | Version | Role |
|-------|---------|------|
| `serde_arrow` | 0.13, features = ["arrow-57"] | Rust structs ↔ arrow-57 RecordBatch (production bridge, no DuckDB needed) |

### Existing (no changes)

| Crate | Version | Role |
|-------|---------|------|
| `duckdb` | 1.4, bundled | Local staging tables, `query_arrow()`, lance extension reads |
| `tokio` | 1.49, full | Async runtime for lancedb operations |

### Arrow version coexistence

Cargo resolves two arrow versions in the same binary:
- `arrow 56.2.0` — used by `duckdb 1.4.4`
- `arrow 57.3.0` — used by `lancedb 0.26.1`

A value-based reconstruction bridge converts between them for the DuckDB extraction path (spike tests I1/I2). In production, data originates as Rust structs → `serde_arrow` → arrow-57 directly, so no bridge is needed.

---

## 6. Spike Tests

**File**: `zenith/crates/zen-lake/src/spike_native_lance.rs`

### Part H: Arrow Bridge + Native lancedb Writes (5 tests)

| # | Test | Validates |
|---|------|-----------|
| H1 | `spike_arrow_ffi_bridge` | Construct a `duckdb::arrow::record_batch::RecordBatch` (arrow 56) with api_symbols schema. Convert via C FFI to `arrow_array::RecordBatch` (arrow 57). Verify schema, column count, row count, and data values match. (H1) |
| H2 | `spike_lancedb_write_local` | Create `arrow_array::RecordBatch` with api_symbols schema (20 cols, `FixedSizeList(Float32, 384)` for embedding). Write to local temp dir via `lancedb::connect(tmpdir).execute()` + `db.create_table()`. Read back via DuckDB lance extension. Verify roundtrip. (H2) |
| H3 | `spike_lancedb_write_r2` | Same as H2 but write to `s3://zenith/spike19/`. R2 creds from env. Read back via DuckDB lance extension with R2 credentials. Skip if creds missing. (H3) |
| H4 | `spike_lancedb_create_indexes` | Write 100 symbols with embeddings to R2 via lancedb. Create IVF-PQ vector index on `embedding`. Create FTS index on `doc_comment`. Query via DuckDB `lance_vector_search()` and `lance_fts()`. Verify results. (H4) |
| H5 | `spike_lancedb_incremental_add` | Write 100 symbols via `create_table()`. Then `tbl.add()` 50 more. Count total (expect 150). Run vector search, verify results include symbols from both batches. (H5) |

### Part I: DuckDB → lancedb Pipeline (2 tests)

| # | Test | Validates |
|---|------|-----------|
| I1 | `spike_duckdb_to_lance_local` | Create + populate DuckDB in-memory table (api_symbols schema, 100 rows). `stmt.query_arrow([])` → FFI bridge → `lancedb::create_table()` → local temp dir. Read back via DuckDB lance extension. Full production pipeline. (H6) |
| I2 | `spike_duckdb_to_lance_r2` | Same as I1 but write to R2. Validates the complete crowdsource upload path: DuckDB staging → arrow → FFI bridge → lancedb → R2. (H7) |

### Part L: Operational Concerns (2 tests)

| # | Test | Validates |
|---|------|-----------|
| L2 | `spike_lance_cross_process_index_read` | Write Lance dataset + create vector/FTS indexes via `lancedb`. Close all lancedb handles. Open a **new** DuckDB connection (simulating different process). Query via `lance_vector_search()` + `lance_fts()`. Verify indexes are usable. (H8) |
| L4 | `spike_lancedb_create_table_exists` | Write a dataset to a path. Call `create_table()` again with same name. Verify: default behavior (error or overwrite). Then test `mode(CreateTableMode::exist_ok(...))` — should return existing table without data loss. (H9) |

**Total: 9 tests**

---

## 7. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| Arrow FFI bridge | **Critical** | Test H1: schema, data, types survive round-trip |
| Local Lance write | **Critical** | Test H2: all 20 columns including FixedSizeList embedding |
| R2 Lance write | **Critical** | Test H3: write to R2, read back via DuckDB |
| Index creation | **High** | Test H4: vector search + FTS return correct results |
| Incremental add | **High** | Test H5: count correct, search finds both batches |
| DuckDB → lancedb pipeline (local) | **Critical** | Test I1: full production path works |
| DuckDB → lancedb pipeline (R2) | **Critical** | Test I2: crowdsource upload path works |
| Cross-process index read | **High** | Test L2: indexes survive process boundary |
| Exist-ok behavior | **Medium** | Test L4: no data loss on re-create |

---

## 8. What This Spike Does NOT Test

- **Turso catalog** — Turso as the `indexed_packages` catalog is spike 0.20
- **Visibility scoping** — Clerk JWT → visibility WHERE clause is spike 0.20
- **Three-tier search** — Federated search across public + team + private is spike 0.20
- **Concurrent writers** — Two users indexing the same package simultaneously is spike 0.20 (L1 test)
- **R2 temporary credentials** — CF Worker credential minting is Phase 9 implementation
- **Real package data** — Spike uses synthetic test data
- **Performance benchmarks** — Spike 0.18 already validated performance; this spike validates correctness
- **Schema evolution** — Adding columns to existing Lance datasets is Phase 9

---

## 9. Success Criteria

- **Arrow FFI bridge works zero-copy** (H1 passes)
- **lancedb writes to local and R2** with full api_symbols schema (H2, H3 pass)
- **Explicit indexes are creatable and queryable** via DuckDB lance extension (H4 passes)
- **Incremental add works** without data loss (H5 passes)
- **DuckDB → FFI → lancedb pipeline works** end-to-end, local and R2 (I1, I2 pass)
- **Cross-process index reads work** (L2 passes)
- **All 9 tests pass** (R2 tests skipped if creds missing)

---

## 10. Post-Spike Actions

### If Spike Passes (Expected Path)

| Doc | Update |
|-----|--------|
| `07-implementation-plan.md` | Add spike 0.19 to Phase 0 table. Update Phase 8/9 write path to use native lancedb. |
| `05-crate-designs.md` | Update zen-lake: `lancedb::create_table()` for writes, drop `COPY TO (FORMAT lance)` |
| `02-ducklake-data-model.md` | Begin rewrite to `02-data-architecture.md` (Turso catalog + Lance storage) |
| `INDEX.md` | Add doc 17 |

### If Arrow FFI Bridge Fails (Fallback A)

- Use Arrow IPC bridge instead (safe code, one memcpy per batch)
- Both arrow 56 and 57 support IPC — serialize with 56's writer, deserialize with 57's reader
- Slightly slower but no `unsafe` needed
- Revert `unsafe_code` lint to `forbid`

### If lancedb R2 Credentials Fail (Fallback B)

- Use DuckDB `COPY TO (FORMAT lance)` for R2 writes (spike 0.18 validated this)
- Use native lancedb only for local writes
- Investigate lancedb `object_store` credential configuration

### If object_store 0.12 Causes Issues Elsewhere (Fallback C)

- Downgrade to lancedb 0.23.1 (uses arrow 56, no FFI bridge needed, object_store 0.12)
- Lose lance 2.0.0 features but gain version alignment
- Upgrade when duckdb bumps to arrow 57

---

## 11. Results

**9/9 PASS** — all tests pass (R2 tests use live credentials).

| Test | Result | Time | Notes |
|------|--------|------|-------|
| H1 `spike_arrow_ffi_bridge` | PASS | 0.04s | Value bridge converts arrow 56 → 57. Schema, types, data all correct. |
| H2 `spike_lancedb_write_local` | PASS | 0.25s | 50 rows, 19 columns, 384-dim FixedSizeList embeddings. DuckDB reads back. |
| H3 `spike_lancedb_write_r2` | PASS | ~2s | R2 write via lancedb storage_options. DuckDB lance extension reads back. |
| H4 `spike_lancedb_create_indexes` | PASS | ~20s | 300 rows. IVF-PQ vector index + BM25 FTS index. `lance_vector_search()` returns func_5 at distance=0.003. `lance_fts()` returns results. |
| H5 `spike_lancedb_incremental_add` | PASS | ~3s | 100 + 50 via `tbl.add()` = 150. Search works across both batches. |
| I1 `spike_duckdb_to_lance_local` | PASS | 1.6s | DuckDB → query_arrow() → value bridge → lancedb → local Lance. Vector search finds func_10 at distance ~0. |
| I2 `spike_duckdb_to_lance_r2` | PASS | ~3s | Full upload pipeline: DuckDB → bridge → lancedb → R2. |
| L2 `spike_lance_cross_process_index_read` | PASS | 1.5s | 300 rows. Vector index survives handle drop. Fresh DuckDB connection reads it. |
| L4 `spike_lancedb_create_table_exists` | PASS | 0.01s | Duplicate errors correctly. `exist_ok` preserves 30 rows. |

### Key Gotchas Discovered

1. **Arrow FFI doesn't work across crate versions**: Rust treats `FFI_ArrowArray` from arrow 56 and 57 as different types despite identical `#[repr(C)]` layout. `transmute` would work but requires `unsafe`. Value-based bridge is the safe alternative.
2. **PQ index minimum 256 rows**: Lance 2.0 IVF-PQ requires >= 256 rows for training. Use `Index::Auto` (which falls back to brute-force for small datasets) or skip vector indexing for small tables. Alternative: `IVF_HNSW_SQ` has no hard minimum.
3. **DuckDB `FLOAT[]` is `List(Float32)`, not `FixedSizeList`**: The bridge must detect uniform-length lists and convert to `FixedSizeList(384)` for Lance compatibility. Schema must be derived from converted columns, not original DuckDB schema.
4. **Production path doesn't need a bridge at all**: Data originates as Rust structs. Use `serde_arrow` with `arrow-57` feature to go directly to arrow-57 RecordBatch. The DuckDB extraction bridge is only for migrating existing data.
5. **`unsafe_code = "forbid"` preserved**: Value bridge is safe code. No lint changes needed.
6. **`object_store` downgraded to 0.12**: Required by lance 2.0 transitive dep. Nothing in the workspace used 0.13 directly.
7. **`protoc` required**: `lance-encoding` crate needs the Protocol Buffers compiler (`brew install protobuf`).

### Architecture Decision: serde_arrow for Production

The spike validates two paths:
- **DuckDB extraction** (tests I1/I2): `query_arrow()` → value bridge → lancedb. Works but copies data.
- **Direct construction** (tests H2-H5): Build `RecordBatch` in Rust → lancedb. No bridge.

For production, the **serde_arrow** path is preferred:
```rust
use serde_arrow::schema::SchemaLike;

let fields = Vec::<FieldRef>::from_type::<ApiSymbol>(TracingOptions::default())?;
let batch = serde_arrow::to_record_batch(&fields, &symbols)?;
// → arrow-57 RecordBatch, directly usable by lancedb::create_table()
```

This avoids both the value bridge AND manual RecordBatch construction. `serde_arrow` with `arrow-57` feature produces native arrow-57 types. Added to workspace as `serde_arrow = { version = "0.13", features = ["arrow-57"] }`.

---

## Cross-References

- R2 Parquet/Lance spike: [spike_r2_parquet.rs](../../crates/zen-lake/src/spike_r2_parquet.rs) (spike 0.18)
- DuckDB VSS spike: [spike_duckdb_vss.rs](../../crates/zen-lake/src/spike_duckdb_vss.rs)
- Clerk auth spike: [spike_clerk_auth.rs](../../crates/zen-db/src/spike_clerk_auth.rs) (spike 0.17)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
- Catalog/visibility spike plan: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md) (spike 0.20)

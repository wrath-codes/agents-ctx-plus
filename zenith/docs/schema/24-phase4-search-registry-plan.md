# Phase 4: Search & Registry — Implementation Plan

**Version**: 2026-02-16 (rev 3)
**Status**: In Progress — Streams A/B/C/D/E implemented in crates; Phase 5 CLI wiring pending
**Depends on**: Phase 2 (zen-db FTS5 repos — **IMPLEMENTED**, 15 repo modules), Phase 3 (zen-lake + zen-embeddings + zen-parser + walker — **COMPLETE**, 1497+ tests), Phase 0 (spikes 0.5, 0.14, 0.21, 0.22)
**Produces**: Milestone 4 — `cargo test -p zen-search -p zen-registry` passes, vector/FTS/hybrid/grep/recursive search works end-to-end, registry clients return real results

> **⚠️ Storage Scope**: Phase 4 builds search engines on top of Phase 3's **local-only DuckDB cache** (`ZenLake` for api_symbols/doc_chunks, `SourceFileStore` for source_files). Production storage (Lance on R2 + Turso catalog) arrives in Phase 8/9. Search queries in Phase 4 use brute-force `array_cosine_similarity()` — Lance indexes replace this later.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state-as-of-2026-02-13)
3. [Key Decisions](#3-key-decisions)
4. [Architecture](#4-architecture)
5. [PR 1 — Stream A: Vector + FTS + Hybrid Search](#5-pr-1--stream-a-vector--fts--hybrid-search)
6. [PR 2 — Stream B: Grep Engine](#6-pr-2--stream-b-grep-engine)
7. [PR 3 — Stream C: Recursive Query + Reference Graph](#7-pr-3--stream-c-recursive-query--reference-graph)
8. [PR 4 — Stream D: Registry Clients](#8-pr-4--stream-d-registry-clients)
9. [PR 5 — Stream E: SearchEngine Orchestrator](#9-pr-5--stream-e-searchengine-orchestrator)
10. [Execution Order](#10-execution-order)
11. [Gotchas & Warnings](#11-gotchas--warnings)
12. [Milestone 4 Validation](#12-milestone-4-validation)
13. [Validation Traceability Matrix](#13-validation-traceability-matrix)
14. [Mismatch Log — Plan vs. Implementation](#14-mismatch-log--plan-vs-implementation)

---

## 1. Overview

**Goal**: Vector search over the local DuckDB lake, FTS over knowledge entities (via zen-db), hybrid search combining both, two-engine grep (package mode via DuckDB + local mode via ripgrep library), RLM-style recursive context query with categorized reference graph, registry HTTP clients (crates.io, npm, PyPI, hex.pm, proxy.golang.org, rubygems.org, packagist.org, Maven Central, NuGet, Hackage, LuaRocks), and a `SearchEngine` orchestrator that ties it all together.

**Crate status summary**:
- `zen-search` — **IMPLEMENTED**: `error.rs`, `vector.rs`, `fts.rs`, `hybrid.rs`, `grep.rs`, `recursive.rs`, `ref_graph.rs`, `graph.rs`, `lib.rs` orchestrator, and `walk.rs`. Current validation: `cargo test -p zen-search` passes (109 tests + 1 doc-test).
- `zen-registry` — **COMPLETE** (14 production files, 2244 LOC, 39 unit tests + 3 ignored network tests). 11 ecosystem clients, shared `http.rs` helper, `RegistryClient` orchestrator with `search_all()` and `search()` dispatch.

**Dependency changes needed**:
- `zen-search`: Promote `rustworkx-core` from `[dev-dependencies]` to `[dependencies]` (for `graph.rs` production module) — **DONE**
- `zen-search`: Add `duckdb.workspace = true` to `[dependencies]` (currently dev-only; needed for `grep.rs` package mode and `vector.rs`) — **DONE**
- `zen-search`: No separate `regex` dep needed — `grep::regex::RegexMatcher` handles pattern compilation for both local and package grep modes
- `zen-registry`: Add `urlencoding.workspace = true` to `[dependencies]` (URL-safe query encoding for registry search URLs) — **DONE**
- `zen-registry`: Add `http = "1"` to `[dev-dependencies]` (mock response construction in `http.rs` tests) — **DONE**

**Estimated deliverables**: ~19 new production files, ~3800 LOC production code, ~1100 LOC tests

**PR strategy**: 5 PRs by stream. Streams A–D are independent and can proceed in parallel. Stream E integrates them.

| PR | Stream | Contents | Depends On | Status |
|----|--------|----------|------------|--------|
| PR 1 | A: Vector + FTS + Hybrid | `vector.rs`, `fts.rs`, `hybrid.rs`, `error.rs` | Phase 2 (zen-db FTS repos), Phase 3 (zen-lake) | **COMPLETE** |
| PR 2 | B: Grep Engine | `grep.rs` (package + local modes) | Phase 3 (SourceFileStore, walk.rs) | **COMPLETE** |
| PR 3 | C: Recursive + Graph | `recursive.rs`, `ref_graph.rs`, `graph.rs` | Phase 3 (zen-lake, zen-parser) | **COMPLETE** |
| PR 4 | D: Registry Clients | `crates_io.rs`, `npm.rs`, `pypi.rs`, `hex.rs`, `go.rs`, `ruby.rs`, `php.rs`, `java.rs`, `csharp.rs`, `haskell.rs`, `lua.rs`, `http.rs`, `error.rs`, `lib.rs` orchestrator | None (standalone HTTP clients) | **COMPLETE** — 14 files, 2244 LOC, 39 unit + 3 ignored tests |
| PR 5 | E: SearchEngine | `lib.rs` orchestrator, `SearchEngine`, `SearchMode` dispatch | Streams A–D | **COMPLETE** |

---

## 2. Current State (as of 2026-02-16)

### zen-search — Streams A/B/C/E Implemented

| Aspect | Status | Detail |
|--------|--------|--------|
| **`walk.rs`** | **DONE** (Phase 3) | `build_walker()`, `WalkMode::LocalProject`/`Raw`, `.zenithignore`, `skip_tests`, include/exclude globs. 6 tests + 1 doc-test. |
| **`lib.rs`** | **DONE** | Full orchestrator: `SearchEngine`, `SearchMode`, `SearchResult`, dispatch helpers for recursive/graph, re-exports |
| **`error.rs`** | **DONE** | `SearchError` hierarchy wired across all engines |
| **`vector.rs`** | **DONE** | Vector search over DuckDB `api_symbols`/`doc_chunks` + filters/tests |
| **`fts.rs`** | **DONE** | FTS5 search via `ZenService`/zen-db repos |
| **`hybrid.rs`** | **DONE** | Combined vector + FTS ranking with alpha blending |
| **`grep.rs`** | **DONE** | Two-engine grep (package + local) |
| **`recursive.rs`** | **DONE (MVP+)** | Recursive engine with budgets, `from_directory`, `from_source_store`, parser-backed extraction, summary JSON |
| **`ref_graph.rs`** | **DONE** | In-memory DuckDB reference graph (`symbol_refs`, `ref_edges`) |
| **`graph.rs`** | **DONE** | Decision graph over `entity_links`: toposort/centrality/shortest path/components/cycles |
| **Cargo.toml** | **DONE (Phase 4)** | Production deps include `duckdb.workspace = true`, `rustworkx-core.workspace = true`; `ast-grep-*` and `tree-sitter` remain dev-deps in zen-search (recursive now uses `zen-parser` extraction API). |

**Spike code inventory** (patterns to promote):

| Spike File | Tests | Key Patterns for Production |
|------------|-------|----------------------------|
| `spike_grep.rs` | 26 | `RegexMatcher` + `Searcher` + custom `Sink`, `WalkBuilder` integration, DuckDB `source_files` fetch + Rust regex line matching, symbol correlation via `idx_symbols_file_lines` binary search |
| `spike_recursive_query.rs` | 17 | `ContextStore` (file→source+spans), `ChunkSelector`, `RecursiveQueryEngine` (metadata-only root + budgeted sub-calls), `RefCategory` enum, `SymbolRefHit`/`RefEdge` types, `symbol_refs`/`ref_edges` DuckDB tables, JSON summary output |
| `spike_graph_algorithms.rs` | 54 | `rustworkx-core` `petgraph::DiGraph`, toposort, centrality (betweenness/closeness), shortest path (Dijkstra), connected components, cycle detection, budget caps, deterministic hash, visibility filtering |

### zen-registry — **COMPLETE** (PR4 / Stream D)

| Aspect | Status | Detail |
|--------|--------|--------|
| **Production code** | **DONE** | 14 files, 2244 LOC total. 11 ecosystem clients + shared `http.rs` helper + `error.rs` + `lib.rs` orchestrator. |
| **Cargo.toml** | **DONE** | Production deps: zen-core, reqwest, serde, serde_json, tokio, thiserror, tracing, urlencoding. Dev-deps: http (v1), pretty_assertions. |
| **Tests** | **DONE** | 39 passing unit tests (fixture-based parsing + error handling + dispatch). 3 ignored network integration tests (manually verified passing). |
| **Review** | **DONE** | Oracle code review + CodeRabbit CLI review (zero findings). JoinSet drain bug fixed, `http.rs` centralization applied. |
| **Design** | **DONE** | Matches `05-crate-designs.md` §9 with documented deviations (see §14 Mismatch Log). |

### Upstream Dependencies — Ready

| Dependency | Crate | Status | Evidence |
|------------|-------|--------|----------|
| `ZenLake` (api_symbols, doc_chunks) | zen-lake | **DONE** | `store_symbols()`, `store_doc_chunks()`, `conn()` for raw SQL, `array_cosine_similarity()` validated |
| `SourceFileStore` (source_files) | zen-lake | **DONE** | Separate DuckDB, `store_source_files()`, `conn()` for content fetch |
| `EmbeddingEngine` (query embedding) | zen-embeddings | **DONE** | `embed_single()`, `embed_batch()`, 384-dim `AllMiniLML6V2`, `&mut self` API |
| `ZenDb` (FTS5 search) | zen-db | **DONE** | 15 repo modules with `search_*()` methods using FTS5 MATCH queries (findings, hypotheses, insights, research, tasks, issues, studies, audit) |
| `build_walker()` | zen-search | **DONE** | `WalkMode::LocalProject`/`Raw`, `.zenithignore`, skip_tests |
| `idx_symbols_file_lines` index | zen-lake | **DONE** | Already in `schemas.rs` CREATE_INDEXES — `ON api_symbols(ecosystem, package, version, file_path, line_start, line_end)` |
| `source_cached` flag | zen-lake | **DONE** | `indexed_packages.source_cached BOOLEAN DEFAULT FALSE`, `set_source_cached()` method |

---

## 3. Key Decisions

All decisions are backed by validated spike results.

### 3.1 Brute-Force Vector Search (Phase 4) — Replaced by Lance Indexes (Phase 8/9)

**Decision**: Use DuckDB `array_cosine_similarity()` with `FLOAT[]` → `FLOAT[384]` cast for vector search in Phase 4. No persistent vector index.

**Rationale**: Spike 0.5 validated `array_cosine_similarity()` works correctly. At local cache scale (thousands of symbols, not millions), brute-force scan is fast enough (<50ms). Lance's `IVF-PQ` vector index replaces this in Phase 8/9 for production scale.

**Validated in**: spike 0.5 (`spike_duckdb_float_array_cosine`), zen-lake production tests (`cosine_similarity_query`).

### 3.2 FTS via zen-db Repos, Not zen-lake

**Decision**: Full-text search over knowledge entities (findings, hypotheses, tasks, etc.) goes through zen-db's existing FTS5-backed `search_*()` repo methods. zen-lake does NOT have FTS — it has vector embeddings.

**Rationale**: Knowledge entities live in Turso/libSQL (zen-db). API symbols and doc chunks live in DuckDB (zen-lake). These are different databases with different search strategies. The `SearchEngine` orchestrator combines results from both.

**Validated in**: Phase 2 implementation — all repo modules have `search_*()` methods using `_fts MATCH ?1` queries with porter stemming.

### 3.3 Hybrid Search: Vector Primary, FTS Boost

**Decision**: Hybrid search uses vector similarity as the primary signal and FTS relevance as a boost factor. Configurable `alpha` parameter controls the blend (0.0 = FTS only, 1.0 = vector only, default 0.7).

**Rationale**: Per task 4.3 design note — Lance FTS is term-exact (no stemming) while libSQL FTS5 uses porter stemming. Vector embeddings capture semantic similarity that keyword search misses. FTS catches exact term matches that vector may rank lower.

### 3.4 Grep: Two Separate Engines, Not One

**Decision**: `GrepEngine` has two distinct code paths — `grep_package()` (DuckDB fetch + Rust regex) and `grep_local()` (`grep` + `ignore` crates). They share types (`GrepMatch`, `GrepResult`, `GrepOptions`) but not implementation.

**Rationale**: Validated in spike 0.14. Package source is stored in DuckDB (compressed, no file sprawl). Local files are on the live filesystem. Each engine is optimal for its domain. See [13-zen-grep-design.md](./13-zen-grep-design.md) for full design.

**Validated in**: spike 0.14 (26 tests — regex matching, ignore walking, DuckDB grep, symbol correlation, combined pipeline).

### 3.5 Source Files in Separate DuckDB — grep_package Takes SourceFileStore

**Decision**: `GrepEngine::grep_package()` accepts a `&SourceFileStore` reference (not `&ZenLake`). Source files live in `.zenith/source_files.duckdb`, separate from the lake cache.

**Rationale**: Per `02-data-architecture.md` §11 and Phase 3 implementation — `SourceFileStore` is a permanent local store for large file content. `ZenLake` holds api_symbols/doc_chunks (temporary cache replaced in Phase 8/9). Grep needs source content, not embeddings.

### 3.6 rustworkx-core Promoted to Production Dependency

**Decision**: Promote `rustworkx-core` from `[dev-dependencies]` to `[dependencies]` in zen-search. The `graph.rs` module is production code.

**Rationale**: Decision graph analytics (toposort, centrality, shortest path, connected components) are production features for `znt search --mode graph` and `znt whats-next` enrichment. Spike 0.22 validated the full API (54 tests). `rustworkx-core` is pure Rust with no heavy transitive deps.

### 3.7 DuckDB Promoted to Production Dependency in zen-search

**Decision**: Promote `duckdb` from `[dev-dependencies]` to `[dependencies]` in zen-search. Required for `grep_package()` (source_files queries) and `vector.rs` (direct `array_cosine_similarity()` queries on ZenLake).

**Rationale**: Vector search and grep package mode both execute SQL directly on DuckDB connections obtained via `ZenLake::conn()` and `SourceFileStore::conn()`. This is direct DuckDB interaction, not abstracted through zen-lake methods.

### 3.8 Recursive Query: Metadata-Only Root, Budgeted Sub-Calls

**Decision**: `RecursiveQueryEngine` follows the RLM pattern — root loop sees metadata only (file list, counts, spans), sub-calls operate on bounded slices. Budget controls: `max_depth`, `max_chunks`, `max_bytes_per_chunk`, `max_total_bytes`.

**Rationale**: Spike 0.21 validated this at Arrow monorepo scale (606 files, 14.9MB). Deterministic budgeted recursion works. Full-context ingestion degrades quality.

**Validated in**: spike 0.21 (17 tests — metadata-only root, budget enforcement, deterministic output, reference categorization, graph persistence).

### 3.9 Reference Graph: DuckDB In-Memory Tables

**Decision**: `symbol_refs` and `ref_edges` tables are created in an in-memory DuckDB connection during recursive query execution. They are transient — not persisted to the lake cache file.

**Rationale**: Spike 0.21 validated DuckDB in-memory tables for ref graph storage. The ref graph is query-session-scoped, not permanent. Production persistence (if needed) can use the lake file in Phase 8/9.

### 3.10 Registry Clients: Recorded JSON Fixtures for Tests

**Decision**: Registry client tests use recorded JSON response fixtures (saved from real API calls), not live HTTP requests. Live integration tests are separate and gated behind `#[ignore]`.

**Rationale**: Per `05-crate-designs.md` §9 test spec — tests must parse real API response format. Live HTTP in CI is flaky (rate limits, network issues). Recorded fixtures give deterministic, fast tests.

**Implementation note** (rev 2): Fixtures are inline `const &str` literals in each module's `#[cfg(test)]` block, not separate files. See mismatch 14.7.

### 3.12 Shared HTTP Response Helper — `check_response()`

**Decision**: Centralize HTTP response status checking (429 rate limiting with `Retry-After` parsing, non-success → `RegistryError::Api`) in a shared `http.rs` module rather than duplicating in each registry client.

**Rationale**: Oracle code review identified ~135 lines of duplicated 429/error boilerplate across 11 registry modules. `check_response()` provides a single point of maintenance. `Retry-After` header parsing (with 60s fallback) was inconsistent across modules before centralization.

**Validated in**: Post-implementation review. 7 unit tests cover `parse_retry_after` (present, missing, non-numeric) and `check_response` (rate-limited with/without header, API error, success). CodeRabbit CLI review passed clean.

### 3.13 JoinSet Drain: Always Join All Tasks

**Decision**: JoinSet drain loops must use `while let Some(res) = set.join_next().await` with explicit `match` on `Ok`/`Err`, logging `JoinError` via `tracing::warn!`. Never use `while let Some(Ok(...))` which silently aborts on the first error.

**Rationale**: Oracle review identified a medium-severity concurrency bug — `while let Some(Ok(...))` breaks the loop early if any task panics or is cancelled, abandoning remaining results. This affected `npm.rs` (download batch), `lua.rs` (config ref counts), and `php.rs` (version/license fetch).

**Validated in**: Fixed in all three modules. Pattern now consistent across the crate.

### 3.14 Lua/Neovim: GitHub API over LuaRocks

**Decision**: `lua.rs` searches GitHub API instead of LuaRocks for Neovim plugin discovery. Uses dual strategy: convention-based (`{query}.nvim in:name`) and broad (`{query} neovim plugin language:lua`). Stargazers count serves as download proxy, boosted by config reference counts from GitHub code search.

**Rationale**: Most Neovim plugins live on GitHub and follow the `.nvim` naming convention. LuaRocks has limited adoption for Neovim plugins. GitHub API provides richer metadata (stars, license, description) and better search relevance than LuaRocks HTML scraping.

### 3.11 SearchEngine Holds References, Not Owned Values

**Decision**: `SearchEngine` borrows `&ZenService`, `&ZenLake`, `&SourceFileStore`, and `&mut EmbeddingEngine` — it does not own them. Lifetime-parameterized struct.

**Rationale**: The CLI creates and owns these resources. `SearchEngine` is a coordinator that dispatches to the right engine. Owning the resources would prevent the CLI from using them for non-search operations.

**Implementation note (rev 3)**: FTS is implemented on `ZenService` repo methods, so orchestrator keeps `&ZenService` rather than `&ZenDb`.

### 3.15 Recursive Mode Dispatches Through SearchEngine With Source-Store Shortcut

**Decision**: `SearchMode::Recursive` now dispatches through `SearchEngine` and chooses context source by filters:
- if `ecosystem + package + version` are present → `RecursiveQueryEngine::from_source_store(...)`
- otherwise → `RecursiveQueryEngine::from_directory(".", ...)`

**Rationale**: This keeps mode dispatch uniform for CLI integration while preserving efficient package-mode execution over indexed source files.

---

## 4. Architecture

### Dependency Flow (Phase 4)

```
zen-core (types, error hierarchy)
    │
    ├──► zen-db (FTS5 search via repos — Phase 2, DONE)
    │       │
    │       └──► zen-core, libsql
    │
    ├──► zen-lake (DuckDB local cache — Phase 3, DONE)
    │       │
    │       └──► zen-core, duckdb
    │
    ├──► zen-embeddings (query embedding — Phase 3, DONE)
    │       │
    │       └──► zen-core, fastembed
    │
    ├──► zen-parser (is_test_file/is_test_dir — Phase 3, DONE)
    │       │
    │       └──► zen-core, ast-grep-*
    │
    ├──► zen-search (ALL search engines — Phase 4, THIS PHASE)
    │       │
    │       ├──► zen-core, zen-db, zen-lake, zen-embeddings, zen-parser
    │       ├──► grep, ignore (local grep)
    │       ├──► duckdb (vector search + grep package mode)
    │       └──► rustworkx-core (graph analytics)
    │
    ├──► zen-registry (HTTP clients — Phase 4, THIS PHASE)
    │       │
    │       └──► zen-core, reqwest, serde, tokio
    │
    └──► zen-cli (orchestration — Phase 5, wires search + registry to CLI)
            │
            └──► zen-search, zen-registry, all other crates
```

### Module Structure After Phase 4

```
zen-search/src/
├── lib.rs              # SearchEngine orchestrator, SearchResult, SearchMode, re-exports
├── error.rs            # NEW: SearchError hierarchy
├── vector.rs           # NEW: Vector search via DuckDB array_cosine_similarity
├── fts.rs              # NEW: FTS5 search via zen-db repos
├── hybrid.rs           # NEW: Hybrid vector + FTS combined ranking
├── grep.rs             # NEW: GrepEngine — package mode (DuckDB) + local mode (grep crate)
├── recursive.rs        # NEW: RecursiveQueryEngine — RLM-style budgeted symbolic recursion
├── ref_graph.rs        # NEW: Reference graph model (symbol_refs, ref_edges, categories)
├── graph.rs            # NEW: Decision context graph (rustworkx-core analytics)
└── walk.rs             # EXISTING: File walker factory (Phase 3, unchanged)

zen-registry/src/
├── lib.rs              # DONE: RegistryClient, PackageInfo, search_all(), search() dispatch (294 LOC)
├── error.rs            # DONE: RegistryError — Http, Api, Parse, UnsupportedEcosystem, RateLimited (35 LOC)
├── http.rs             # DONE: Shared check_response() — 429 Retry-After parsing, error mapping (114 LOC)
├── crates_io.rs        # DONE: crates.io API client (124 LOC)
├── npm.rs              # DONE: npm registry + api.npmjs.org download batch fetch via JoinSet (187 LOC)
├── pypi.rs             # DONE: PyPI JSON API single-package lookup (125 LOC)
├── hex.rs              # DONE: hex.pm API client (142 LOC)
├── go.rs               # DONE: proxy.golang.org module proxy + pkg.go.dev HTML search (118 LOC)
├── ruby.rs             # DONE: rubygems.org API client (108 LOC)
├── php.rs              # DONE: packagist.org search + p2 version/license fetch via JoinSet (194 LOC)
├── java.rs             # DONE: search.maven.org (Maven Central) API client (150 LOC)
├── csharp.rs           # DONE: nuget.org (NuGet v3) + SPDX license extraction + GitHub URL splitting (214 LOC)
├── haskell.rs          # DONE: hackage.haskell.org two-step lookup (preferred.json + metadata) (192 LOC)
└── lua.rs              # DONE: GitHub dual-search (*.nvim convention + broad) + config ref boost (247 LOC)
```

### Cargo.toml Changes

**zen-search/Cargo.toml** — promote dev-deps to production:

```toml
[dependencies]
# ... existing deps unchanged ...

# Vector search + grep package mode (Phase 4 — promoted from dev-deps)
duckdb.workspace = true

# Graph analytics — decision context graph (Phase 4 — promoted from dev-deps)
rustworkx-core.workspace = true

[dev-dependencies]
pretty_assertions.workspace = true
rstest.workspace = true
tempfile.workspace = true
# `ast-grep-*` and `tree-sitter` remain dev-dependencies in zen-search
# (recursive extraction uses `zen-parser::extract_api` in production path)
```

**zen-registry/Cargo.toml** — no changes needed. Existing deps sufficient.

---

## 5. PR 1 — Stream A: Vector + FTS + Hybrid Search

**Tasks**: 4.1 (vector), 4.2 (FTS), 4.3 (hybrid)
**Depends on**: Phase 2 (zen-db FTS repos — DONE), Phase 3 (zen-lake — DONE)

### A1. `src/error.rs` — SearchError Hierarchy

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("lake error: {0}")]
    Lake(#[from] zen_lake::LakeError),

    #[error("database error: {0}")]
    Database(#[from] zen_db::error::DatabaseError),

    #[error("embedding error: {0}")]
    Embedding(#[from] zen_embeddings::EmbeddingError),

    #[error("grep error: {0}")]
    Grep(String),

    #[error("registry error: {0}")]
    Registry(String),

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("no results found")]
    NoResults,

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}
```

### A2. `src/vector.rs` — Vector Search (task 4.1)

Embeds the query text via `EmbeddingEngine`, then queries DuckDB with `array_cosine_similarity()`.

```rust
use duckdb::params;
use zen_embeddings::EmbeddingEngine;
use zen_lake::ZenLake;

use crate::error::SearchError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub file_path: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub score: f64,
    pub source_type: VectorSource,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum VectorSource {
    ApiSymbol,
    DocChunk,
}

#[derive(Debug, Clone)]
pub struct VectorSearchFilters {
    pub package: Option<String>,
    pub ecosystem: Option<String>,
    pub kind: Option<String>,
    pub limit: u32,
    pub min_score: f64,
}

impl Default for VectorSearchFilters {
    fn default() -> Self {
        Self {
            package: None,
            ecosystem: None,
            kind: None,
            limit: 20,
            min_score: 0.0,
        }
    }
}

/// Search api_symbols by vector similarity.
///
/// Embeds the query, then uses brute-force `array_cosine_similarity()`
/// on DuckDB FLOAT[] columns (cast to FLOAT[384] at query time).
///
/// **Phase 4 only** — replaced by `lance_vector_search()` in Phase 8/9.
pub fn vector_search_symbols(
    lake: &ZenLake,
    query_embedding: &[f32],
    filters: &VectorSearchFilters,
) -> Result<Vec<VectorSearchResult>, SearchError> {
    // Build WHERE clause from filters
    // SELECT *, array_cosine_similarity(embedding::FLOAT[384], ?::FLOAT[384]) AS score
    // FROM api_symbols WHERE embedding IS NOT NULL AND <filters>
    // ORDER BY score DESC LIMIT ?
    // ...
}

/// Search doc_chunks by vector similarity.
pub fn vector_search_doc_chunks(
    lake: &ZenLake,
    query_embedding: &[f32],
    filters: &VectorSearchFilters,
) -> Result<Vec<VectorSearchResult>, SearchError> {
    // Same pattern as symbols but against doc_chunks table
    // ...
}
```

**Key implementation notes**:
- Query embedding comes from `EmbeddingEngine::embed_single()` (called by `SearchEngine`, not by `vector.rs` directly)
- `embedding::FLOAT[384]` cast is required — embeddings stored as `FLOAT[]` (per §3.5 of Phase 3 plan)
- Query vector also needs `?::FLOAT[384]` cast — pass as `vec_to_sql()` string literal (reuse from `zen-lake/src/store.rs`)
- DuckDB connection obtained via `lake.conn()` — synchronous queries
- Filter building: optional WHERE clauses for package, ecosystem, kind

### A3. `src/fts.rs` — FTS5 Search via zen-db (task 4.2)

Thin adapter over zen-db's existing `search_*()` repo methods.

```rust
use zen_db::ZenDb;

use crate::error::SearchError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtsSearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: Option<String>,
    pub content: String,
    pub relevance: f64,
}

#[derive(Debug, Clone)]
pub struct FtsSearchFilters {
    pub entity_types: Vec<String>,  // filter to specific entity types
    pub limit: u32,
}

/// Search across all FTS5-indexed knowledge entities in zen-db.
///
/// Queries: findings, hypotheses, insights, research, tasks, issues, studies, audit.
/// Returns results ranked by FTS5 relevance score (porter stemming).
///
/// **Note**: Takes `&ZenService` (not `&ZenDb`) because `search_*()` methods
/// are implemented on `ZenService`.
pub async fn fts_search(
    service: &ZenService,
    query: &str,
    filters: &FtsSearchFilters,
) -> Result<Vec<FtsSearchResult>, SearchError> {
    let mut results = Vec::new();

    // Call each repo's search method, collect results
    // Filter by entity_types if specified
    // findings_fts, hypotheses_fts, insights_fts, research_fts,
    // tasks_fts, issues_fts, studies_fts, audit_fts
    // ...

    Ok(results)
}
```

**Key implementation notes**:
- zen-db is async (libsql) — `fts_search()` is `async fn`
- Each repo returns its own entity type (Finding, Hypothesis, etc.) — normalize to `FtsSearchResult`
- FTS5 uses porter stemming: "spawning" matches "spawn", "runtime" matches "runtimes"
- `rank` column from FTS5 provides relevance ordering

### A4. `src/hybrid.rs` — Hybrid Search (task 4.3)

Combines vector similarity and FTS relevance with configurable alpha blending.

```rust
use crate::error::SearchError;
use crate::fts::FtsSearchResult;
use crate::vector::VectorSearchResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridSearchResult {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub content: String,
    pub vector_score: Option<f64>,
    pub fts_score: Option<f64>,
    pub combined_score: f64,
    pub source: HybridSource,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HybridSource {
    VectorSymbol,
    VectorDocChunk,
    Fts,
}

/// Combine vector and FTS results with alpha blending.
///
/// `alpha` controls the blend: 0.0 = FTS only, 1.0 = vector only, default 0.7.
/// Results are deduplicated by name and ranked by combined score.
pub fn combine_results(
    vector_results: &[VectorSearchResult],
    fts_results: &[FtsSearchResult],
    alpha: f64,
    limit: u32,
) -> Vec<HybridSearchResult> {
    // Normalize scores to [0, 1] range
    // Combine: combined = alpha * vector_score + (1 - alpha) * fts_score
    // Deduplicate by (name, kind) or entity_id
    // Sort by combined_score DESC, take limit
    // ...
}
```

**Key implementation notes**:
- Vector scores (cosine similarity) are already in [-1, 1] range — normalize to [0, 1]
- FTS5 `rank` values are negative (more negative = more relevant) — normalize to [0, 1] by inverting
- Deduplication: a finding about "tokio spawn" may appear in both vector and FTS results
- Default alpha=0.7 favors vector (semantic) over keyword (exact match)

### A5. Tests for Stream A

**Vector search tests** (unit, in `vector.rs`):
- Insert known symbols with synthetic embeddings → query with same embedding → self-match has highest score
- Insert symbols with different embeddings → verify ranking by cosine similarity
- Package filter: only returns symbols from specified package
- Kind filter: only returns specified symbol kinds
- Min score filter: excludes low-similarity results
- Empty lake returns empty results (not error)

**FTS search tests** (unit, in `fts.rs`):
- Porter stemming: "spawning" matches finding containing "spawn"
- Entity type filter: only returns specified entity types
- Multi-entity search: returns results from multiple entity types, merged
- Empty query returns error

**Hybrid search tests** (unit, in `hybrid.rs`):
- Alpha=1.0: only vector results contribute to score
- Alpha=0.0: only FTS results contribute to score
- Alpha=0.5: equal blend produces expected ranking
- Deduplication: same entity from both sources merged correctly
- Combined ranking better than either alone (test with crafted scores)

---

## 6. PR 2 — Stream B: Grep Engine

**Tasks**: 4.10 (grep_package), 4.11 (grep_local), 4.12 (idx_symbols_file_lines — ALREADY DONE)
**Depends on**: Phase 3 (SourceFileStore, walk.rs, idx_symbols_file_lines)

> **Task 4.12 status**: `idx_symbols_file_lines` already exists in `zen-lake/src/schemas.rs` CREATE_INDEXES. Verified by `index_existence` test. No work needed.

### B1. `src/grep.rs` — Two-Engine Grep

Full design in [13-zen-grep-design.md](./13-zen-grep-design.md). Key types and both modes:

```rust
use std::path::PathBuf;

use zen_lake::{SourceFileStore, ZenLake};

use crate::error::SearchError;

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: u64,
    pub text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub symbol: Option<SymbolRef>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRef {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
    pub stats: GrepStats,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepStats {
    pub files_searched: u64,
    pub files_matched: u64,
    pub matches_found: u64,
    pub matches_with_symbol: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub case_insensitive: bool,
    pub smart_case: bool,
    pub fixed_strings: bool,
    pub word_regexp: bool,
    pub multiline: bool,
    pub context_before: u32,
    pub context_after: u32,
    pub include_glob: Option<String>,
    pub exclude_glob: Option<String>,
    pub max_count: Option<u32>,
    pub skip_tests: bool,
    pub no_symbols: bool,
}

// ── GrepEngine ─────────────────────────────────────────────────────

pub struct GrepEngine;

impl GrepEngine {
    /// Grep indexed package source: DuckDB fetch + Rust regex + symbol correlation.
    ///
    /// 1. Query `source_files` from SourceFileStore for specified packages
    /// 2. For each file: split content by lines, apply regex, collect matches + context
    /// 3. Correlate matches with `api_symbols` via idx_symbols_file_lines index
    ///
    /// **Spike 0.14 validated**: DuckDB fetch + Rust regex is faster than SQL-level
    /// line splitting. DuckDB is compressed storage; Rust does line matching.
    pub fn grep_package(
        source_store: &SourceFileStore,
        lake: &ZenLake,
        pattern: &str,
        packages: &[(String, String, String)], // (ecosystem, package, version)
        opts: &GrepOptions,
    ) -> Result<GrepResult, SearchError> {
        // 1. Build regex from pattern + flags
        // 2. For each package: SELECT file_path, content FROM source_files WHERE ...
        // 3. For each file: line-by-line regex match + context lines
        // 4. If !no_symbols: batch symbol lookup per matched file
        //    SELECT id, kind, name, signature, line_start, line_end
        //    FROM api_symbols WHERE ecosystem=? AND package=? AND version=? AND file_path=?
        //    ORDER BY line_start
        // 5. Binary search: find symbol where line_start <= match_line <= line_end
        // ...
    }

    /// Grep local project files using grep + ignore crates (ripgrep library).
    ///
    /// Uses `build_walker()` from walk.rs for file discovery and
    /// `grep::searcher::Searcher` + custom `Sink` for matching.
    /// Symbol correlation is NOT available in local mode.
    pub fn grep_local(
        pattern: &str,
        paths: &[PathBuf],
        opts: &GrepOptions,
    ) -> Result<GrepResult, SearchError> {
        // 1. Build RegexMatcher from grep-regex crate
        // 2. For each path: build_walker(path, WalkMode::LocalProject, skip_tests, ...)
        // 3. For each file: Searcher::new().search_path(matcher, path, sink)
        // 4. Custom Sink collects GrepMatch structs with context lines
        // 5. symbol is always None in local mode
        // ...
    }
}
```

**Key implementation notes**:
- `GrepEngine` is stateless (no stored references) — takes dependencies as parameters
- Package mode needs both `SourceFileStore` (content) and `ZenLake` (symbol correlation)
- Local mode needs neither — uses filesystem directly via `grep` + `ignore` crates
- Symbol correlation algorithm: per-file batch query + binary search (O(log n) per match)
- `grep::searcher::Searcher` with `grep::searcher::sinks::UTF8` for line-by-line matching
- Custom `Sink` implementation to capture context_before/context_after lines
- `skip_tests` in package mode: filter by filename patterns (reuse `zen_parser::is_test_file`)
- `skip_tests` in local mode: via `build_walker()` filter_entry (already implemented)

### B2. Tests for Stream B

**Package mode tests** (`grep.rs`):
- Pattern matches lines in stored source files
- Context lines (before/after) correct
- Symbol correlation: match inside a function gets `SymbolRef` attached
- Match outside any symbol range gets `symbol: None`
- `no_symbols` flag: all matches have `symbol: None`
- `skip_tests`: test files excluded from results
- Case-insensitive matching works
- Fixed-string (literal) matching works
- `max_count` limits matches per file
- `include_glob`: only matching files searched
- Multi-package: `packages` with 2+ entries searches all
- Package not cached: returns error with guidance

**Local mode tests** (`grep.rs`):
- Pattern matches files in temp directory
- `.gitignore` respected (ignored files not searched)
- `.zenithignore` respected
- `skip_tests`: test directories excluded
- Include/exclude globs work
- Symbol is always `None`
- Binary files skipped automatically (grep crate behavior)

**Stats tests**:
- `files_searched`, `files_matched`, `matches_found` counts correct
- `matches_with_symbol` count correct for package mode
- `elapsed_ms` is non-zero

---

## 7. PR 3 — Stream C: Recursive Query + Reference Graph

**Tasks**: 4.13 (RecursiveQueryEngine), 4.14 (ref graph), 4.15 (external references + JSON summary)
**Depends on**: Phase 3 (zen-lake, zen-parser), spike 0.21 (17 tests)

### C1. `src/recursive.rs` — RecursiveQueryEngine (task 4.13)

Promotes patterns from `spike_recursive_query.rs` to production code.

```rust
use std::collections::HashMap;
use std::path::Path;

use crate::error::SearchError;
use crate::ref_graph::{RefCategory, RefEdge, SymbolRefHit};

// ── Budget Controls ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RecursiveBudget {
    pub max_depth: usize,
    pub max_chunks: usize,
    pub max_bytes_per_chunk: usize,
    pub max_total_bytes: usize,
}

impl Default for RecursiveBudget {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_chunks: 200,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 750_000,
        }
    }
}

// ── Context Store ──────────────────────────────────────────────────

/// In-memory map of file_path → source + AST spans + doc spans.
/// Root loop sees metadata only (counts, file list). Sub-calls access content.
pub struct ContextStore {
    files: HashMap<String, FileContext>,
}

pub struct FileContext {
    pub source: String,
    pub symbols: Vec<SymbolSpan>,
    pub doc_spans: Vec<DocSpan>,
}

pub struct SymbolSpan {
    pub kind: String,
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    pub doc_comment: Option<String>,
}

pub struct DocSpan {
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
}

// ── Engine ─────────────────────────────────────────────────────────

pub struct RecursiveQueryEngine {
    store: ContextStore,
    budget: RecursiveBudget,
}

/// Result of a recursive query — categorized symbol references with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecursiveQueryResult {
    pub hits: Vec<SymbolRefHit>,
    pub edges: Vec<RefEdge>,
    pub category_counts: HashMap<String, usize>,
    pub budget_used: BudgetUsed,
    pub summary_json: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetUsed {
    pub depth_reached: usize,
    pub chunks_processed: usize,
    pub total_bytes_processed: usize,
}

impl RecursiveQueryEngine {
    /// Build a ContextStore by scanning source files under `root`.
    ///
    /// Uses ast-grep KindMatcher for symbol extraction and prev() sibling
    /// walking for doc comment extraction. Extended impl queries for Rust.
    pub fn from_directory(
        root: &Path,
        budget: RecursiveBudget,
    ) -> Result<Self, SearchError> {
        // Walk files, parse each, extract symbols + docs
        // Store in ContextStore (file_path → FileContext)
        // ...
    }

    /// Build from pre-indexed DuckDB source_files (for indexed packages).
    pub fn from_source_store(
        source_store: &zen_lake::SourceFileStore,
        ecosystem: &str,
        package: &str,
        version: &str,
        budget: RecursiveBudget,
    ) -> Result<Self, SearchError> {
        // Fetch source from DuckDB, parse, extract symbols + docs
        // ...
    }

    /// Run metadata-only root planning.
    ///
    /// Returns file count, total symbols, total doc spans, total bytes
    /// WITHOUT loading source content into the root context.
    pub fn plan(&self) -> RecursiveQueryPlan {
        // ...
    }

    /// Execute the recursive query with budgeted sub-calls.
    ///
    /// 1. Root loop: filter files/symbols by query (AST kind + doc keyword scan)
    /// 2. Select slices within budget (max_chunks, max_bytes_per_chunk)
    /// 3. Sub-call each slice: extract references, categorize
    /// 4. Assemble output with stable ordering
    pub fn execute(
        &self,
        query: &RecursiveQuery,
    ) -> Result<RecursiveQueryResult, SearchError> {
        // ...
    }
}

#[derive(Debug, Clone)]
pub struct RecursiveQuery {
    /// AST node kinds to target (e.g., "function_item", "struct_item")
    pub target_kinds: Vec<String>,
    /// Doc comment keywords to filter by (e.g., "safety", "panic", "invariant")
    pub doc_keywords: Vec<String>,
    /// Include external references (e.g., DataFusion Arrow usage)
    pub include_external: bool,
    /// Generate JSON summary output
    pub generate_summary: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecursiveQueryPlan {
    pub file_count: usize,
    pub total_symbols: usize,
    pub total_doc_spans: usize,
    pub total_bytes: usize,
}
```

### C2. `src/ref_graph.rs` — Reference Graph Model (task 4.14)

Promotes types from `spike_recursive_query.rs`. Reference graph persistence uses in-memory DuckDB.

```rust
use std::collections::HashMap;

use crate::error::SearchError;

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRefHit {
    /// Stable ID: file::kind::name::line
    pub ref_id: String,
    pub file_path: String,
    pub kind: String,
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub signature: String,
    pub doc: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefEdge {
    pub source_ref_id: String,
    pub target_ref_id: String,
    pub category: RefCategory,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RefCategory {
    SameModule,
    OtherModuleSameCrate,
    OtherCrateWorkspace,
    External,
}

// ── Reference Graph ────────────────────────────────────────────────

/// In-memory reference graph with DuckDB backing for queries.
///
/// Stores symbol_refs and ref_edges in a transient in-memory DuckDB connection.
/// Used during recursive query execution for category stats and signature lookup.
pub struct ReferenceGraph {
    conn: duckdb::Connection,
}

/// DuckDB DDL for transient ref graph tables.
const CREATE_REF_GRAPH: &str = "
CREATE TABLE symbol_refs (
    ref_id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    signature TEXT NOT NULL,
    doc TEXT
);

CREATE TABLE ref_edges (
    source_ref_id TEXT NOT NULL,
    target_ref_id TEXT NOT NULL,
    category TEXT NOT NULL,
    evidence TEXT,
    PRIMARY KEY(source_ref_id, target_ref_id)
);

CREATE INDEX ref_edges_category_idx ON ref_edges(category);
";

impl ReferenceGraph {
    /// Create a new in-memory reference graph.
    pub fn new() -> Result<Self, SearchError> {
        let conn = duckdb::Connection::open_in_memory()
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        conn.execute_batch(CREATE_REF_GRAPH)
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        Ok(Self { conn })
    }

    /// Insert symbol refs and edges in bulk.
    pub fn insert(
        &self,
        refs: &[SymbolRefHit],
        edges: &[RefEdge],
    ) -> Result<(), SearchError> {
        // Use parameterized INSERT for refs and edges
        // ...
    }

    /// Get category counts (same_module, other_module_same_crate, etc.).
    pub fn category_counts(&self) -> Result<HashMap<String, usize>, SearchError> {
        // SELECT category, COUNT(*) FROM ref_edges GROUP BY category
        // ...
    }

    /// Lookup signature by ref_id.
    pub fn lookup_signature(&self, ref_id: &str) -> Result<Option<String>, SearchError> {
        // SELECT signature FROM symbol_refs WHERE ref_id = ?
        // ...
    }
}
```

### C3. `src/graph.rs` — Decision Context Graph (task 4.4, extended)

Promotes patterns from `spike_graph_algorithms.rs`. Uses `rustworkx-core` with `petgraph::DiGraph`.

```rust
use std::collections::HashMap;

use rustworkx_core::petgraph::graph::DiGraph;

use crate::error::SearchError;

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub entity_type: String,
    pub entity_id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub relation: String,
    pub weight: f64,
}

/// Decision context graph built from entity_links in zen-db.
///
/// Provides graph algorithms over the entity relationship network:
/// - Topological sort (task ordering)
/// - Centrality (influence ranking)
/// - Shortest path (explainability chains)
/// - Connected components (clusters)
/// - Cycle detection (circular dependencies)
pub struct DecisionGraph {
    graph: DiGraph<GraphNode, GraphEdge>,
    id_to_index: HashMap<String, petgraph::graph::NodeIndex>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphAnalysis {
    pub node_count: usize,
    pub edge_count: usize,
    pub components: usize,
    pub has_cycles: bool,
    pub topological_order: Option<Vec<String>>,
    pub centrality: Vec<(String, f64)>,
}

impl DecisionGraph {
    /// Build graph from entity_links in zen-db.
    pub async fn from_db(db: &zen_db::ZenDb) -> Result<Self, SearchError> {
        // SELECT source_type, source_id, target_type, target_id, relation
        // FROM entity_links
        // Build DiGraph with GraphNode/GraphEdge
        // ...
    }

    /// Topological sort (DAG only — returns None if cycles exist).
    pub fn toposort(&self) -> Option<Vec<String>> {
        // rustworkx_core::dag_algo::topological_sort()
        // ...
    }

    /// Betweenness centrality for all nodes.
    pub fn centrality(&self) -> Vec<(String, f64)> {
        // rustworkx_core::centrality::betweenness_centrality()
        // ...
    }

    /// Shortest path between two entities.
    pub fn shortest_path(
        &self,
        from: &str,
        to: &str,
    ) -> Option<Vec<String>> {
        // rustworkx_core::shortest_path::dijkstra()
        // ...
    }

    /// Connected components count.
    pub fn connected_components(&self) -> usize {
        // petgraph::algo::connected_components()
        // ...
    }

    /// Cycle detection.
    pub fn has_cycles(&self) -> bool {
        // petgraph::algo::is_cyclic_directed()
        // ...
    }

    /// Full analysis with budget cap on centrality computation.
    pub fn analyze(&self, max_nodes_for_centrality: usize) -> GraphAnalysis {
        // ...
    }
}
```

### C4. Tests for Stream C

**RecursiveQueryEngine tests** (`recursive.rs`):
- Build ContextStore from temp directory with Rust/Python files
- Metadata-only plan: returns counts without loading source
- Budget max_chunks: selection truncates deterministically
- Budget max_bytes_per_chunk: slices truncated
- Budget max_total_bytes: hard cap stops recursion
- Linear query: doc keyword filter returns expected symbols
- Output determinism: two runs produce identical results
- From source_store: DuckDB-backed context loading works
- **Arrow monorepo scale** (`#[ignore]`): Run recursive query over `/Users/wrath/reference/rust/arrow-rs` (606 files, 14.9MB) with budget controls — verify deterministic output matches spike 0.21 baseline

**Reference graph tests** (`ref_graph.rs`):
- Insert refs + edges, verify category_counts
- Lookup signature by ref_id
- Category enum serialization roundtrip
- Empty graph returns empty counts (not error)
- Duplicate ref_id ignored (PRIMARY KEY)

**External reference + JSON summary tests** (`ref_graph.rs` + `recursive.rs`):
- External reference detection: populate ContextStore with Arrow + DataFusion sources → verify refs tagged `RefCategory::External`
- JSON summary output: `summary_json` and `summary_json_pretty` produce valid JSON with pair samples, external samples, signatures
- External references at scale (`#[ignore]`): cached DataFusion Arrow references discoverable via `~/.cargo/registry/src/**/datafusion-*` path pattern

**Decision graph tests** (`graph.rs`):
- Build from entity_links: correct node/edge counts
- Toposort: DAG produces valid ordering
- Toposort: cyclic graph returns None
- Centrality: hub node has highest score
- Shortest path: returns correct chain
- Connected components: disjoint subgraphs counted
- Budget cap: centrality skipped for large graphs
- Empty graph: all operations return defaults

---

## 8. PR 4 — Stream D: Registry Clients

**Tasks**: 4.5 (crates.io), 4.6 (npm), 4.7 (PyPI), 4.8 (hex.pm), 4.16 (Go), 4.17 (Ruby), 4.18 (PHP), 4.19 (Java/Maven), 4.20 (C#/NuGet), 4.21 (Haskell/Hackage), 4.22 (Lua/LuaRocks), 4.9 (search_all)
**Depends on**: None (standalone HTTP clients)

### D1. `zen-registry/src/lib.rs` — RegistryClient + PackageInfo

```rust
pub mod crates_io;
pub mod csharp;
pub mod go;
pub mod haskell;
pub mod hex;
pub mod java;
pub mod lua;
pub mod npm;
pub mod php;
pub mod pypi;
pub mod ruby;

mod error;

pub use error::RegistryError;

use serde::{Deserialize, Serialize};

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub description: String,
    pub downloads: u64,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

// ── Client ─────────────────────────────────────────────────────────

pub struct RegistryClient {
    http: reqwest::Client,
}

impl RegistryClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("zenith/0.1")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client should build"),
        }
    }

    /// Search all registries concurrently. Returns merged results sorted by downloads.
    pub async fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<PackageInfo> {
        let (crates, npm, pypi, hex, go, ruby, php, java, csharp, haskell, lua) = tokio::join!(
            self.search_crates_io(query, limit),
            self.search_npm(query, limit),
            self.search_pypi(query, limit),
            self.search_hex(query, limit),
            self.search_go(query, limit),
            self.search_rubygems(query, limit),
            self.search_packagist(query, limit),
            self.search_maven(query, limit),
            self.search_nuget(query, limit),
            self.search_hackage(query, limit),
            self.search_luarocks(query, limit),
        );

        let mut results = Vec::new();
        results.extend(crates.unwrap_or_default());
        results.extend(npm.unwrap_or_default());
        results.extend(pypi.unwrap_or_default());
        results.extend(hex.unwrap_or_default());
        results.extend(go.unwrap_or_default());
        results.extend(ruby.unwrap_or_default());
        results.extend(php.unwrap_or_default());
        results.extend(java.unwrap_or_default());
        results.extend(csharp.unwrap_or_default());
        results.extend(haskell.unwrap_or_default());
        results.extend(lua.unwrap_or_default());
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results.truncate(limit);
        results
    }

    /// Search a specific ecosystem.
    pub async fn search(
        &self,
        query: &str,
        ecosystem: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        match ecosystem {
            "rust" => self.search_crates_io(query, limit).await,
            "npm" | "javascript" | "typescript" => self.search_npm(query, limit).await,
            "pypi" | "python" => self.search_pypi(query, limit).await,
            "hex" | "elixir" => self.search_hex(query, limit).await,
            "go" | "golang" => self.search_go(query, limit).await,
            "ruby" | "rubygems" => self.search_rubygems(query, limit).await,
            "php" | "packagist" => self.search_packagist(query, limit).await,
            "java" | "maven" => self.search_maven(query, limit).await,
            "csharp" | "nuget" | "dotnet" => self.search_nuget(query, limit).await,
            "haskell" | "hackage" => self.search_hackage(query, limit).await,
            "lua" | "luarocks" | "neovim" => self.search_luarocks(query, limit).await,
            _ => Err(RegistryError::UnsupportedEcosystem(ecosystem.to_string())),
        }
    }
}
```

### D2. `zen-registry/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("parse error: {0}")]
    Parse(String),

    #[error("unsupported ecosystem: {0}")]
    UnsupportedEcosystem(String),

    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
}
```

### D3. `zen-registry/src/crates_io.rs` — crates.io Client (task 4.5)

```rust
use crate::{PackageInfo, RegistryClient, error::RegistryError};

/// crates.io API response structures (serde models for JSON deserialization)
#[derive(serde::Deserialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
}

#[derive(serde::Deserialize)]
struct CrateInfo {
    name: String,
    max_version: String,
    description: Option<String>,
    downloads: u64,
    license: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
}

impl RegistryClient {
    pub async fn search_crates_io(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let url = format!(
            "https://crates.io/api/v1/crates?q={}&per_page={limit}",
            urlencoding::encode(query)
        );
        let resp = self.http.get(&url).send().await?;

        if resp.status() == 429 {
            return Err(RegistryError::RateLimited { retry_after_secs: 60 });
        }
        if !resp.status().is_success() {
            return Err(RegistryError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let data: CratesResponse = resp.json().await?;
        Ok(data.crates.into_iter().map(|c| PackageInfo {
            name: c.name,
            version: c.max_version,
            ecosystem: "rust".to_string(),
            description: c.description.unwrap_or_default(),
            downloads: c.downloads,
            license: c.license,
            repository: c.repository,
            homepage: c.homepage,
        }).collect())
    }
}
```

### D4. Per-Ecosystem Clients (tasks 4.6–4.8)

**`npm.rs`**: Searches `https://registry.npmjs.org/-/v1/search?text={query}&size={limit}`. Enriches with download counts from `https://api.npmjs.org/downloads/point/last-month/{name}`. Response shape: `{ objects: [{ package: { name, version, description, links: { repository, homepage } } }] }`.

**`pypi.rs`**: Searches `https://pypi.org/pypi/{name}/json` for single-package lookup. For search, uses `https://pypi.org/search/?q={query}` (HTML scraping) or the newer XML-RPC search endpoint. Response shape: `{ info: { name, version, summary, license, home_page, project_urls } }`.

**`hex.rs`**: Searches `https://hex.pm/api/packages?search={query}&sort=downloads&page=1`. Response shape: `[{ name, latest_stable_version, meta: { description, licenses, links } }]`. Downloads from `downloads.all` field.

**Note**: Each client module follows the same pattern as `crates_io.rs`: define serde response structs, implement `search_<ecosystem>()` on `RegistryClient`, map to `PackageInfo`.

### D4b. Additional Per-Ecosystem Clients (tasks 4.16–4.22)

**`go.rs`** (task 4.16): Go module proxy at `https://proxy.golang.org/{module}/@v/list` for version listing and `https://pkg.go.dev/search?q={query}` for search. Module paths are URL-encoded (e.g., `github.com/gin-gonic/gin` → `github.com/gin-gonic/gin`). Download counts from `https://proxy.golang.org` are not directly available — use `0` as default and document the limitation.

**`ruby.rs`** (task 4.17): RubyGems API at `https://rubygems.org/api/v1/search.json?query={query}&page=1`. Response shape: `[{ name, version, info, downloads, licenses, homepage_uri, source_code_uri }]`. Maps directly to `PackageInfo`.

**`php.rs`** (task 4.18): Packagist API at `https://packagist.org/search.json?q={query}&per_page={limit}`. Response shape: `{ results: [{ name, description, url, repository, downloads, favers }] }`. Version requires follow-up call to `https://repo.packagist.org/p2/{vendor}/{package}.json` — use latest version from `packages` key.

**`java.rs`** (task 4.19): Maven Central search API at `https://search.maven.org/solrsearch/select?q={query}&rows={limit}&wt=json`. Response shape: `{ response: { docs: [{ g, a, latestVersion, p, ec }] } }`. `g` = groupId, `a` = artifactId. Download counts not available via search API — use `0`. License from `https://search.maven.org/solrsearch/select?q=g:{g}+AND+a:{a}&core=gav`.

**`csharp.rs`** (task 4.20): NuGet v3 search API at `https://azuresearch-usnc.nuget.org/query?q={query}&take={limit}`. Response shape: `{ data: [{ id, version, description, totalDownloads, licenseUrl, projectUrl }] }`. Well-documented JSON API, maps cleanly to `PackageInfo`.

**`haskell.rs`** (task 4.21): Hackage search at `https://hackage.haskell.org/packages/search?terms={query}` (HTML) or package info at `https://hackage.haskell.org/package/{name}.json`. For search, use `https://hackage.haskell.org/packages/search?terms={query}` with HTML parsing or the deprecated JSON endpoint. For MVP, support direct package lookup via `/{name}.json` and document search limitations (similar to PyPI).

**`lua.rs`** (task 4.22): LuaRocks API at `https://luarocks.org/search?q={query}` (HTML) or manifest API at `https://luarocks.org/manifest`. For programmatic search, use `https://luarocks.org/search?q={query}&type=module` with HTML parsing. Scoped to Neovim ecosystem — tag results with `ecosystem: "lua"`. Download stats not available — use `0`.

**Note**: Go, Java, Haskell, and Lua registries have limited or no download count APIs. These clients set `downloads: 0` and document the limitation. This affects `search_all()` ranking — packages from these ecosystems will sort to the bottom when ordered by downloads. A future enhancement could add a secondary sort by relevance score.

### D5. Workspace Dependency Addition

`zen-registry/Cargo.toml` needs `urlencoding` for URL-safe query encoding:

```toml
[dependencies]
# ... existing ...
urlencoding = "2"
```

Add to workspace `Cargo.toml`:
```toml
urlencoding = "2"
```

### D6. Tests for Stream D

**Fixture-based tests** (per registry, using recorded JSON):
- `crates_io.rs`: Parse real crates.io search response → correct `PackageInfo` fields
- `npm.rs`: Parse real npm search response → correct fields, downloads enriched
- `pypi.rs`: Parse real PyPI JSON response → correct fields
- `hex.rs`: Parse real hex.pm response → correct fields
- `go.rs`: Parse real proxy.golang.org / pkg.go.dev response → correct `PackageInfo` fields
- `ruby.rs`: Parse real rubygems.org search response → correct fields, downloads present
- `php.rs`: Parse real packagist.org search response → correct fields
- `java.rs`: Parse real Maven Central search response → correct groupId:artifactId naming
- `csharp.rs`: Parse real NuGet v3 search response → correct fields, totalDownloads mapped
- `haskell.rs`: Parse real Hackage package JSON → correct fields
- `lua.rs`: Parse real LuaRocks response → correct fields, ecosystem tagged "lua"

**Error handling tests**:
- 404 response → `RegistryError::Api { status: 404 }`
- 429 response → `RegistryError::RateLimited`
- Invalid JSON → `RegistryError::Parse`
- Network timeout → `RegistryError::Http`

**`search_all()` tests**:
- Merges results from all registries
- Sorted by downloads (descending)
- One registry failure doesn't fail the whole search (returns empty for that ecosystem)
- Truncated to `limit`

**`search()` ecosystem dispatch tests**:
- `"rust"` → calls `search_crates_io`
- `"npm"` → calls `search_npm`
- `"python"` / `"pypi"` → calls `search_pypi`
- `"hex"` / `"elixir"` → calls `search_hex`
- `"go"` / `"golang"` → calls `search_go`
- `"ruby"` / `"rubygems"` → calls `search_rubygems`
- `"php"` / `"packagist"` → calls `search_packagist`
- `"java"` / `"maven"` → calls `search_maven`
- `"csharp"` / `"nuget"` / `"dotnet"` → calls `search_nuget`
- `"haskell"` / `"hackage"` → calls `search_hackage`
- `"lua"` / `"luarocks"` / `"neovim"` → calls `search_luarocks`
- Unknown ecosystem → `RegistryError::UnsupportedEcosystem`

**Live integration tests** (`#[ignore]`):
- `search_crates_io("tokio", 5)` returns results with name containing "tokio"
- `search_npm("express", 5)` returns results
- `search_all("http client", 10)` returns results from multiple ecosystems
- `search_rubygems("rails", 5)` returns results
- `search_nuget("newtonsoft", 5)` returns results
- `search_go("gin", 5)` returns results

---

## 9. PR 5 — Stream E: SearchEngine Orchestrator

**Tasks**: 4.4 (SearchEngine orchestrator)
**Depends on**: Streams A–D (all search engines + registry)

### E1. `src/lib.rs` — SearchEngine + Unified API

Implemented in this session:

- `src/lib.rs` is now full orchestrator code (no `todo!()` placeholders) with module declarations and re-exports.
- `SearchEngine<'a>` borrows `&ZenService`, `&ZenLake`, `&SourceFileStore`, and `&mut EmbeddingEngine`.
- `SearchMode` dispatch works for all modes:
  - `Vector`: embeds query once, searches symbols + doc chunks, sorts deterministically.
  - `Fts`: calls `fts::fts_search()` through `ZenService` repo methods.
  - `Hybrid { alpha }`: combines vector + FTS via `hybrid::combine_results()`.
  - `Recursive`: dispatches through orchestrator (package triplet → `from_source_store`, else local `from_directory`).
  - `Graph`: dispatches through `DecisionGraph::from_service()` and returns `GraphAnalysis`.
- `SearchResult` includes all active variants: `Vector`, `Fts`, `Hybrid`, `Recursive`, `Graph`.
- `SearchFilters` now includes `version` for package-scoped recursive dispatch.
- Helper coverage added in `lib.rs` tests (`recursive_package_triplet`, recursive dispatch helpers, graph helper).

### E2. Tests for Stream E

**Status**: Implemented and passing.

Added tests cover:
- deterministic orchestrator sorting and limit normalization helpers,
- recursive package triplet extraction from filters,
- recursive dispatch helper branch selection (source_store vs local fallback),
- graph dispatch helper returning correct analysis counts.

---

## 10. Execution Order

### Phase 4 Task Checklist

Streams A–D are independent and can proceed in parallel. Stream E integrates them.

```
Phase 4 Prerequisites (all DONE):
  [x] Phase 2: zen-db FTS5 repos (15 modules, search methods)
  [x] Phase 3: zen-lake (ZenLake, SourceFileStore, schemas)
  [x] Phase 3: zen-embeddings (EmbeddingEngine, 384-dim)
  [x] Phase 3: zen-search/walk.rs (WalkMode, build_walker)
  [x] Phase 0: idx_symbols_file_lines index in schemas.rs

Stream A: Vector + FTS + Hybrid (tasks 4.1–4.3)
  [x] A0. Promote duckdb to [dependencies] in zen-search/Cargo.toml
  [x] A1. Create src/error.rs — SearchError hierarchy
  [x] A2. Create src/vector.rs — vector_search_symbols, vector_search_doc_chunks
  [x] A3. Create src/fts.rs — fts_search over zen-db repos
  [x] A4. Create src/hybrid.rs — combine_results with alpha blending
  [x] A5. Tests: vector (6), fts (4), hybrid (5)

Stream B: Grep Engine (tasks 4.10–4.12)
  [x] B0. idx_symbols_file_lines already exists (task 4.12 DONE)
  [x] B1. Create src/grep.rs — GrepEngine with grep_package and grep_local
  [x] B2. Tests: package mode (12), local mode (7), stats (3)

Stream C: Recursive + Graph (tasks 4.13–4.15)
  [x] C0. Promote rustworkx-core to [dependencies] (ast-grep/tree-sitter promotion not required in final impl)
  [x] C1. Create src/recursive.rs — RecursiveQueryEngine + ContextStore
  [x] C2. Create src/ref_graph.rs — ReferenceGraph + SymbolRefHit + RefEdge
  [x] C3. Create src/graph.rs — DecisionGraph with rustworkx-core
  [x] C4. Tests: recursive/ref_graph/graph coverage implemented

Stream D: Registry Clients (tasks 4.5–4.9, 4.16–4.22) — **COMPLETE** ✅
  [x] D0. Add urlencoding to workspace + zen-registry Cargo.toml
  [x] D1. Create zen-registry/src/error.rs — RegistryError (Http, Api, Parse, UnsupportedEcosystem, RateLimited)
  [x] D1b. Create zen-registry/src/http.rs — shared check_response() helper (NOT IN ORIGINAL PLAN — added during review)
  [x] D2. Create zen-registry/src/crates_io.rs
  [x] D3. Create zen-registry/src/npm.rs (+ JoinSet download batch fetch with Semaphore(10))
  [x] D4. Create zen-registry/src/pypi.rs
  [x] D5. Create zen-registry/src/hex.rs
  [x] D6. Create zen-registry/src/go.rs (+ encode_go_module_path, lookup_go_module, pkg.go.dev HTML search)
  [x] D7. Create zen-registry/src/ruby.rs
  [x] D8. Create zen-registry/src/php.rs (+ JoinSet p2 version/license fetch with Semaphore(5))
  [x] D9. Create zen-registry/src/java.rs
  [x] D10. Create zen-registry/src/csharp.rs (+ extract_license SPDX, split_project_url GitHub detection)
  [x] D11. Create zen-registry/src/haskell.rs (two-step: preferred.json → metadata)
  [x] D12. Create zen-registry/src/lua.rs (GitHub dual-search + config ref boost via code search)
  [x] D13. Update zen-registry/src/lib.rs — RegistryClient + search_all (11 ecosystems) + search() dispatch
  [x] D14. Inline JSON fixtures in test modules (not separate files — deviation from plan)
  [x] D15. Tests: 39 unit (fixture parsing + error handling + dispatch + http helper) + 3 ignored network

Stream E: SearchEngine Orchestrator (task 4.4)
  [x] E1. Update src/lib.rs — SearchEngine, SearchMode, SearchResult
  [x] E2. Tests: orchestrator helper + dispatch coverage

Final:
  [x] cargo test -p zen-search -p zen-registry
  [x] cargo clippy -p zen-search -p zen-registry --no-deps -- -D warnings
  [ ] Milestone 4 acceptance criteria verified at CLI level (`znt search` wiring in Phase 5)
```

### Critical Path

```
Streams A, B, C, D (parallel) → Stream E (integrates all) → Milestone 4
```

No stream depends on another stream. Stream E depends on all four.

### Recommended Execution Order

1. **Stream D first** (zen-registry) — fully independent, no cross-crate complexity, good warm-up
2. **Stream B next** (grep) — depends only on existing Phase 3 code, well-defined by design doc
3. **Stream A** (vector/FTS/hybrid) — core search, needs both zen-lake and zen-db
4. **Stream C** (recursive/graph) — most complex, benefits from having error.rs and vector.rs established
5. **Stream E last** (orchestrator) — integrates everything

---

## 11. Gotchas & Warnings

### 11.1 DuckDB Is Synchronous — Use spawn_blocking from Async

DuckDB queries (`vector.rs`, `grep.rs` package mode, `ref_graph.rs`) are synchronous. When called from async CLI code via `SearchEngine`, wrap in `tokio::task::spawn_blocking()`. The `SearchEngine::search()` method is `async` because FTS and graph queries go through zen-db (async libsql), but vector and grep are sync internally.

**Spike evidence**: Spike 0.4 note — "DuckDB is synchronous; async strategy documented (prefer `spawn_blocking`)."

### 11.2 EmbeddingEngine Requires &mut self

`EmbeddingEngine::embed_single()` and `embed_batch()` take `&mut self`. This means `SearchEngine` must hold a mutable reference. Cannot share `EmbeddingEngine` across threads without `Mutex`.

**Spike evidence**: Spike 0.6 gotcha — "fastembed `embed()` takes `&mut self`, not `&self`."

### 11.3 FLOAT[] Cast Required for Cosine Similarity

Embeddings stored as `FLOAT[]` (variable-length). Must cast to `FLOAT[384]` at query time: `embedding::FLOAT[384]`. Without cast, `array_cosine_similarity()` may fail or produce wrong results.

**Spike evidence**: Spike 0.5 — "DuckDB `FLOAT[N]` enforces dimension at insert time." Spike 0.18 — "`FLOAT[]` embeddings need `::FLOAT[384]` cast for `array_cosine_similarity()`."

### 11.4 Query Vector Must Also Be Cast

Both sides of `array_cosine_similarity()` need `FLOAT[384]`. The query embedding (passed as SQL literal string `[0.1, 0.2, ...]`) must also have `::FLOAT[384]` cast.

### 11.5 FTS5 Rank Is Negative

libSQL FTS5 `rank` values are negative — more negative means more relevant. Normalization for hybrid search must invert: `normalized = -rank / max(-rank)` or similar.

### 11.6 crates.io Rate Limiting

crates.io enforces rate limits. The `User-Agent: zenith/0.1` header is required (per crates.io policy). 429 responses should be caught and returned as `RegistryError::RateLimited`.

### 11.7 grep Crate Sink API

The `grep` crate's `Sink` trait is how you capture match results. Custom `Sink` implementation needed for `context_before`/`context_after` capture. The `UTF8` sink provides basic line matching but doesn't capture context directly — use `grep::searcher::SinkContext` or manage context lines manually.

**Spike evidence**: Spike 0.14 — "Custom `Sink` for context lines" validated.

### 11.8 rustworkx-core petgraph Compatibility

`rustworkx-core` re-exports `petgraph` types. Use `rustworkx_core::petgraph::` imports, not a separate `petgraph` crate dependency, to avoid version conflicts.

**Spike evidence**: Spike 0.22 — "rustworkx-core 0.17" uses petgraph internally.

### 11.9 PyPI Has No Official Search API

PyPI deprecated its XML-RPC search endpoint. Options: (a) simple search via `https://pypi.org/simple/` (package listing, not search), (b) direct package lookup `https://pypi.org/pypi/{name}/json`, (c) use `https://pypi.org/search/?q=` with HTML parsing. For MVP, support direct package lookup only and document the limitation.

### 11.10 SourceFileStore vs ZenLake Connection

`GrepEngine::grep_package()` queries `source_files` from `SourceFileStore` (separate DuckDB file) but correlates with `api_symbols` from `ZenLake` (different DuckDB file). These are separate `duckdb::Connection` objects. Cannot do cross-database JOINs — must fetch from each independently and correlate in Rust.

### 11.11 Recursive Extraction Uses `zen-parser` API (No Direct ast-grep Promotion)

Final implementation routes recursive symbol extraction through `zen_parser::extract_api`, so zen-search does not need to promote `ast-grep-*` and `tree-sitter` to production dependencies.

### 11.12 Lance FTS Is Term-Exact (No Stemming) — Phase 8/9 Forward Reference

Lance BM25 FTS (`lance_fts()`) is **term-exact**: searching "spawning" will NOT match documents containing "spawn". This informed the Phase 4 hybrid search design (§3.3): vector similarity is the primary signal, FTS5 (with porter stemming) is the boost. When Lance replaces brute-force vector search in Phase 8/9, continue using libSQL FTS5 for stemmed keyword matching — do not rely on Lance FTS for inflected forms.

**Spike evidence**: Spike 0.5 — "Lance FTS is term-exact (no stemming)."

### 11.13 Lance Uses AWS Credential Chain, Not DuckDB Secrets — Phase 8/9 Forward Reference

Lance (via `object_store`) reads credentials from `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_ENDPOINT_URL` environment variables. It does NOT use DuckDB `CREATE SECRET`. When migrating to Lance-backed search in Phase 8/9, credentials must be set as env vars (or through the AWS credential chain), not through DuckDB secret management.

**Spike evidence**: Spike 0.5 — "Lance uses AWS env vars for S3 creds, not DuckDB `CREATE SECRET`." Spike 0.18 — same finding confirmed for R2 uploads.

### 11.14 Graph Centrality Is O(V·E) — Budget Cap Required

Betweenness centrality computation is O(V·E). For graphs with >1000 nodes, skip centrality and return only structural metrics (node/edge counts, components, cycles, toposort). The `DecisionGraph::analyze()` method takes a `max_nodes_for_centrality` parameter for this purpose.

### 11.15 Graph Visibility Filtering — Subgraph Before Analysis

When visibility scoping is active (Phase 9), construct a visibility-filtered subgraph BEFORE running any graph algorithms. Do not run algorithms on the full graph and filter results — that leaks information about private nodes through centrality scores and path lengths.

**Spike evidence**: Spike 0.22 — "Visibility-scoped subgraph construction" validated (test: `spike_visibility_filtering`).

### 11.16 Some Registries Lack Download Count APIs

Go (proxy.golang.org), Java (Maven Central), Haskell (Hackage), and Lua (LuaRocks) do not expose download counts in their search APIs. These clients set `downloads: 0`. This affects `search_all()` ranking — results from these ecosystems will always sort to the bottom when ordered by downloads descending. Consider adding a secondary relevance-based sort or ecosystem-weighted normalization in a future enhancement.

### 11.17 Go Module Proxy Protocol Is Not a Search API

`proxy.golang.org` is a **module proxy**, not a search API. It serves module versions (`/{module}/@v/list`) and metadata (`/{module}/@v/{version}.info`) but has no search endpoint. For search, use `https://pkg.go.dev/search?q={query}` with HTML parsing or the internal API. Document this dual-source approach in `go.rs`.

### 11.18 Hackage and LuaRocks Have Limited Programmatic Search

Hackage's search endpoint returns HTML. For MVP, support direct package lookup via `https://hackage.haskell.org/package/{name}.json` and document that search requires HTML parsing (similar to PyPI's deprecated search). LuaRocks similarly returns HTML for search — use manifest parsing or HTML extraction.

---

## 12. Milestone 4 Validation

### Command

```bash
cargo test -p zen-search -p zen-registry
cargo clippy -p zen-search -p zen-registry --no-deps -- -D warnings
```

### Acceptance Criteria

**Vector search** (tasks 4.1):
- [x] `vector_search_symbols()` returns results ranked by cosine similarity
- [x] `vector_search_doc_chunks()` searches doc chunks separately
- [x] `FLOAT[]` → `FLOAT[384]` cast works in production queries
- [x] Package, ecosystem, kind filters applied correctly
- [x] Empty lake returns empty results (not error)

**FTS search** (task 4.2):
- [x] `fts_search()` queries all 8 FTS5-indexed entity types
- [x] Porter stemming: "spawning" matches "spawn"
- [x] Entity type filter restricts search scope
- [x] Results ranked by FTS5 relevance

**Hybrid search** (task 4.3):
- [x] `combine_results()` blends vector + FTS with configurable alpha
- [x] Alpha=0.7 (default) produces sensible ranking
- [x] Deduplication handles same entity from both sources
- [x] Combined ranking better than either alone (validated in test)

**SearchEngine orchestrator** (task 4.4):
- [x] `SearchEngine::search()` dispatches to correct engine per mode
- [x] Vector mode embeds query → searches lake
- [x] FTS mode queries zen-db repos
- [x] Hybrid mode combines both
- [x] Graph mode builds decision graph, returns analysis

**Grep — package mode** (task 4.10):
- [x] `GrepEngine::grep_package()` searches stored source files
- [x] Regex matching with all flags (case, word, fixed, multiline)
- [x] Symbol correlation attaches `SymbolRef` to matches within symbol ranges
- [x] Context lines (before/after) correct
- [x] `skip_tests` excludes test files
- [x] Multi-package search across 2+ packages

**Grep — local mode** (task 4.11):
- [x] `GrepEngine::grep_local()` searches filesystem via grep + ignore crates
- [x] `.gitignore` and `.zenithignore` respected
- [x] `skip_tests` uses `build_walker()` filter_entry
- [x] Include/exclude globs work

**Grep — index** (task 4.12):
- [x] `idx_symbols_file_lines` exists in schemas.rs — **ALREADY DONE**

**Recursive query** (task 4.13):
- [x] `RecursiveQueryEngine::from_directory()` builds ContextStore
- [x] Metadata-only `plan()` returns counts without loading source
- [x] `execute()` with budget controls produces bounded results
- [x] Deterministic ordering/selection covered by tests

**Reference graph** (task 4.14):
- [x] `ReferenceGraph` stores symbol_refs + ref_edges in-memory DuckDB
- [x] `category_counts()` returns per-category edge counts
- [x] `lookup_signature()` finds signature by stable ref_id
- [x] `RefCategory` enum: SameModule, OtherModuleSameCrate, OtherCrateWorkspace, External

**External references + JSON summary** (task 4.15):
- [x] External references (heuristic path-based) discoverable and tagged as `External`
- [x] `summary_json` output available
- [x] JSON summary includes sample hits/edges + category counts
- [ ] External DataFusion Arrow references discoverable and tagged as `RefCategory::External`

**Registry — crates.io** (task 4.5): ✅
- [x] `search_crates_io()` parses real API response (fixture)
- [x] Returns correct `PackageInfo` fields
- [x] Handles 429 rate limit (via shared `check_response()`)

**Registry — npm** (task 4.6): ✅
- [x] `search_npm()` parses real API response (fixture)
- [x] Download count enrichment from api.npmjs.org (JoinSet batch with Semaphore(10))

**Registry — PyPI** (task 4.7): ✅
- [x] `search_pypi()` handles single-package JSON lookup
- [x] Returns correct fields (404 → empty Vec, not error)

**Registry — hex.pm** (task 4.8): ✅
- [x] `search_hex()` parses real API response (fixture)
- [x] Downloads from `downloads.all` field

**Registry — Go/proxy.golang.org** (task 4.16): ✅
- [x] `search_go()` parses pkg.go.dev HTML search results
- [x] Module path URL-encoding handled correctly (`encode_go_module_path()`)
- [x] `lookup_go_module()` resolves single module via proxy.golang.org

**Registry — Ruby/rubygems.org** (task 4.17): ✅
- [x] `search_rubygems()` parses real API response (fixture)
- [x] Downloads field mapped correctly

**Registry — PHP/packagist.org** (task 4.18): ✅
- [x] `search_packagist()` parses real API response (fixture)
- [x] Version resolved from follow-up p2 package call (JoinSet with Semaphore(5))

**Registry — Java/Maven Central** (task 4.19): ✅
- [x] `search_maven()` parses real API response (fixture)
- [x] groupId:artifactId mapped to name field

**Registry — C#/NuGet** (task 4.20): ✅
- [x] `search_nuget()` parses real NuGet v3 response (fixture)
- [x] totalDownloads mapped correctly
- [x] SPDX license extraction from licenseUrl (`extract_license()`)
- [x] GitHub/GitLab URL detection for repository field (`split_project_url()`)

**Registry — Haskell/Hackage** (task 4.21): ✅
- [x] `search_hackage()` uses two-step lookup (preferred.json → metadata)
- [x] Returns correct fields
- [x] GitHub/GitLab homepage → repository field promotion

**Registry — Lua/LuaRocks** (task 4.22): ✅
- [x] `search_luarocks()` uses GitHub dual-search (convention + broad)
- [x] Ecosystem tagged as "lua" (Neovim scope)
- [x] Config reference boost via GitHub code search (Semaphore(2))

**Registry — search_all** (task 4.9): ✅
- [x] `search_all()` merges results from all 11 registries concurrently (`tokio::join!`)
- [x] Sorted by downloads (descending)
- [x] One registry failure doesn't fail the whole search (`unwrap_or_log` pattern)
- [x] Registries with no download counts (Go, Java, Haskell, Lua) sort last

**Registry — shared infrastructure** (not in original plan): ✅
- [x] `http.rs` — `check_response()` centralizes 429 rate-limit handling + `Retry-After` parsing
- [x] JoinSet drain loops handle `JoinError` (log + continue, don't abort)
- [x] `error.rs` — `RegistryError` with `RateLimited { retry_after_secs }` variant
- [x] `PackageInfo` derives `PartialEq, Eq` for test assertions
- [x] `RegistryClient` implements `Default` trait
- [x] `search()` dispatch supports all ecosystem aliases (26 aliases → 11 registries)

**Overall**:
- [x] `cargo test -p zen-search -p zen-registry` all pass
- [x] `cargo clippy -p zen-search -p zen-registry --no-deps -- -D warnings` clean
- [x] Spike modules remain behind `#[cfg(test)]` (not removed)

### What This Unlocks

Phase 4 completion unblocks:
- **Phase 5** (CLI Shell): `znt search`, `znt grep`, `znt cache`, `znt research registry` commands
- **Phase 5** task 5.24: `znt search --mode recursive` with budget flags
- **Phase 6** (PRD Workflow): Search integration for research-driven workflows

---

## 13. Validation Traceability Matrix

### Spike Evidence (from Phase 0)

| Area | Claim | Status | Spike/Test Evidence | Source |
|------|-------|--------|---------------------|--------|
| DuckDB cosine similarity | `array_cosine_similarity()` works with FLOAT[384] cast | Validated | `spike_duckdb_float_array_cosine` | `zen-lake/src/spike_duckdb.rs` (spike 0.4) |
| DuckDB JSON columns | JSON operators work on stored data | Validated | `spike_duckdb_json_columns` | `zen-lake/src/spike_duckdb.rs` (spike 0.4) |
| grep crate regex | `RegexMatcher` compiles patterns with flags | Validated | `spike_grep_regex_matcher` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| grep crate Searcher | `Searcher` + `UTF8` sink with line numbers | Validated | `spike_grep_searcher_utf8` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| grep custom Sink | Custom Sink captures context lines | Validated | `spike_grep_custom_sink_context` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| ignore crate walking | `.gitignore` aware, override globs | Validated | `spike_ignore_gitignore_aware` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| ignore filter_entry | Test file skipping is pre-I/O | Validated | `spike_ignore_test_file_skipping` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| DuckDB source_files grep | Fetch + Rust regex faster than SQL line split | Validated | `spike_source_files_crud`, `spike_duckdb_regexp_grep` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| Symbol correlation | batch query + binary search per matched file | Validated | `spike_symbol_correlation` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| Combined grep pipeline | Walk → grep → correlate end-to-end | Validated | `spike_combined_pipeline` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| source_cached flag | Boolean tracking on indexed_packages | Validated | `spike_source_cached_flag` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| All-packages grep | Cross-package search works | Validated | `spike_all_packages_search` | `zen-search/src/spike_grep.rs` (spike 0.14) |
| Recursive metadata-only root | Root loop sees metadata, not source content | Validated | `spike_recursive_metadata_only_root_and_budget` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Recursive filter AST + docs | AST kind + doc keyword filters work | Validated | `spike_recursive_filter_ast_and_docs` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Recursive budget enforcement | max_chunks, max_bytes, max_total | Validated | `spike_recursive_budget_*` (3 tests) | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Recursive determinism | Two runs yield identical output | Validated | `spike_recursive_stability` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Extended impl queries | +580 matches vs baseline on Arrow monorepo | Validated | `spike_impl_query_delta` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Reference graph persistence | symbol_refs + ref_edges in DuckDB | Validated | `spike_reference_graph_persistence` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Signature lookup by ref_id | Stable ref_id → signature query | Validated | `spike_reference_graph_persistence` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Reference categorization | same_module, other_module_same_crate, other_crate_workspace, external | Validated | `spike_recursive_pairwise_task` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| JSON summary output | summary_json, summary_json_pretty | Validated | `spike_recursive_pairwise_task` | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| rustworkx-core toposort | DAG topological sort | Validated | `spike_toposort_*` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| rustworkx-core centrality | Betweenness centrality | Validated | `spike_centrality_*` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| rustworkx-core shortest path | Dijkstra shortest path | Validated | `spike_shortest_path_*` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| Connected components | petgraph connected components | Validated | `spike_connected_components` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| Cycle detection | petgraph is_cyclic_directed | Validated | `spike_cycle_detection` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| Budget caps on graph ops | Centrality skipped for large graphs | Validated | `spike_budget_caps_*` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| Deterministic hash | Stable graph output across runs | Validated | `spike_deterministic_hash` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |
| Graph visibility filtering | Visibility-scoped subgraph construction | Validated | `spike_visibility_filtering` | `zen-search/src/spike_graph_algorithms.rs` (spike 0.22) |

### Production Evidence (from Phase 2 and Phase 3)

| Area | Claim | Status | Evidence | Source |
|------|-------|--------|----------|--------|
| FTS5 repos | All entity types have search_*() methods | **DONE** | 15 repo modules with FTS5 MATCH queries | `zen-db/src/repos/*.rs` |
| FTS5 porter stemming | "runtime" matches "runtimes" | **DONE** | `fts5_search_works` test | `zen-db/src/lib.rs` tests |
| ZenLake store + query | Symbols and chunks stored and queryable | **DONE** | 8 tests | `zen-lake/src/lib.rs` tests |
| SourceFileStore | Source files stored in separate DuckDB | **DONE** | 4 tests | `zen-lake/src/lib.rs` tests |
| Cosine similarity on stored data | `array_cosine_similarity()` with FLOAT[] cast | **DONE** | `cosine_similarity_query` test | `zen-lake/src/lib.rs` tests |
| idx_symbols_file_lines | Index exists on api_symbols | **DONE** | `index_existence` test | `zen-lake/src/lib.rs` tests |
| Walker factory | build_walker with LocalProject/Raw | **DONE** | 6 tests + 1 doc-test | `zen-search/src/walk.rs` |
| EmbeddingEngine | 384-dim AllMiniLML6V2, deterministic | **DONE** | 7 tests | `zen-embeddings/src/lib.rs` tests |
| SearchEngine orchestrator | Vector/FTS/Hybrid/Recursive/Graph dispatch integrated | **DONE** | helper + dispatch tests in `lib.rs`; full crate tests passing | `zen-search/src/lib.rs` |
| Recursive reference graph | in-memory DuckDB persistence + category/signature queries | **DONE** | recursive/ref_graph tests passing | `zen-search/src/recursive.rs`, `zen-search/src/ref_graph.rs` |
| Decision graph | entity_links graph analysis (toposort, centrality, shortest path, components, cycles) | **DONE** | graph tests passing | `zen-search/src/graph.rs` |

---

## 14. Mismatch Log — Plan vs. Implementation

### 14.1 Shared `http.rs` Helper Module — Not in Original Plan

**Original plan**: Each registry module handles 429 rate limiting and non-success status codes inline, with duplicated `if resp.status() == 429 { ... }` blocks in every `search_*()` method.

**Actual implementation**: Created `src/http.rs` with `check_response()` that centralizes: (a) 429 rate-limit detection with `Retry-After` header parsing (falls back to 60s), (b) non-success status → `RegistryError::Api` with body. All 11 registry modules call `check_response(self.http.get(&url).send().await?).await?` instead of inline checks. Removed ~135 lines of duplicated boilerplate.

**Impact**: Positive — single point of maintenance for HTTP error handling. No behavioral change. Added 7 unit tests for the helper (parse_retry_after, check_response variants). Required `http = "1"` dev-dependency for mock `reqwest::Response` construction in tests.

### 14.2 JoinSet Drain Bug — Silent Abort on JoinError

**Original plan**: Plan did not specify JoinSet drain behavior in detail.

**Actual implementation**: Initial implementation used `while let Some(Ok((idx, val))) = set.join_next().await` which silently breaks the loop on the first `JoinError` (task panic or cancellation), abandoning remaining tasks. Fixed to `while let Some(res) = set.join_next().await { match res { Ok(...) => ..., Err(e) => tracing::warn!(...) } }` in `npm.rs`, `lua.rs`, and `php.rs`.

**Impact**: Bug fix — prevents data loss when concurrent download/version fetches fail. All remaining tasks are now always joined.

### 14.3 `search_all()` Uses `unwrap_or_log` Instead of `unwrap_or_default`

**Original plan**: `results.extend(crates.unwrap_or_default())` — silent error swallowing.

**Actual implementation**: `unwrap_or_log` closure logs registry name and error via `tracing::warn!` before returning empty vec. Provides observability for production debugging.

**Impact**: Better diagnostics. Registry failures are now visible in logs.

### 14.4 `search()` Dispatch — Additional Ecosystem Alias `"cargo"`

**Original plan**: `"rust"` maps to `search_crates_io()`.

**Actual implementation**: Both `"rust"` and `"cargo"` map to `search_crates_io()`. Total: 26 aliases → 11 registries.

**Impact**: Minor usability improvement. No breaking change.

### 14.5 `PackageInfo` Derives `PartialEq, Eq`

**Original plan**: `PackageInfo` derives `Debug, Clone, Serialize, Deserialize`.

**Actual implementation**: Also derives `PartialEq, Eq` for test assertions.

**Impact**: None for production. Enables `assert_eq!` in tests.

### 14.6 `RegistryClient` Implements `Default`

**Original plan**: Only `new()` constructor.

**Actual implementation**: `impl Default for RegistryClient` delegates to `new()`. Required by clippy pedantic lint `new_without_default`.

**Impact**: API completeness. Follows Rust API guidelines.

### 14.7 Test Fixtures Are Inline, Not Separate Files

**Original plan**: "recorded JSON responses — 11 registries" suggesting separate fixture files.

**Actual implementation**: JSON fixtures are `const &str` literals inside each module's `#[cfg(test)] mod tests` block (e.g., `CRATES_IO_FIXTURE`, `GITHUB_FIXTURE`). This keeps tests self-contained and avoids file I/O.

**Impact**: Simpler test setup. No file paths to manage. Tests are fully self-contained.

### 14.8 Lua/Neovim Uses GitHub API Instead of LuaRocks

**Original plan**: `lua.rs` searches LuaRocks API at `https://luarocks.org/search?q=` with HTML parsing.

**Actual implementation**: `lua.rs` searches GitHub API with dual strategy: (a) `{query}.nvim in:name` convention-based, (b) `{query} neovim plugin language:lua` broad search. Results merged, deduplicated. Config reference boost via GitHub code search on `init.lua`, `lazy.lua`, `plugins.lua`. Uses stargazers_count as download proxy.

**Impact**: Better results for Neovim ecosystem (most plugins are on GitHub, not LuaRocks). Function still named `search_luarocks()` for API compatibility. Ecosystem field is `"lua"`.

### 14.9 Haskell Uses Two-Step Lookup Instead of Single JSON Endpoint

**Original plan**: "package info at `https://hackage.haskell.org/package/{name}.json`".

**Actual implementation**: Two-step lookup: (1) `GET /package/{name}/preferred.json` → latest stable version from `normal-version` array, (2) `GET /package/{name}-{version}` with `Accept: application/json` → metadata (synopsis, description, license, homepage). GitHub/GitLab homepage URLs are promoted to `repository` field.

**Impact**: More accurate version resolution (uses preferred/stable version, not just latest). Richer metadata from step 2.

### 14.10 NuGet Has SPDX License Extraction and GitHub URL Detection

**Original plan**: `csharp.rs` maps `licenseUrl` directly to `license` field and `projectUrl` to `homepage`.

**Actual implementation**: `extract_license()` parses SPDX identifiers from `licenses.nuget.org/{spdx-id}` URLs (e.g., `https://licenses.nuget.org/MIT` → `"MIT"`). `split_project_url()` detects GitHub/GitLab URLs to populate `repository` field separately from `homepage`.

**Impact**: Cleaner license data (SPDX ID instead of URL). Better repository/homepage separation.

### 14.11 PHP Packagist Uses JoinSet for Version/License Resolution

**Original plan**: "Version requires follow-up call to `https://repo.packagist.org/p2/{vendor}/{package}.json`".

**Actual implementation**: Follow-up calls are batched via `tokio::task::JoinSet` with `Semaphore(5)` concurrency limit. Each task fetches the p2 endpoint to resolve latest version and license. Results are joined back and merged into the search results.

**Impact**: Concurrent version resolution — faster for multi-result searches.

### 14.12 Go Module `search_go` Parses HTML from pkg.go.dev

**Original plan**: Mentioned HTML parsing as an option.

**Actual implementation**: `search_go()` fetches `https://pkg.go.dev/search?q={query}&limit={limit}` and parses `<a href="/{module-path}">` links from the HTML response. `lookup_go_module()` is a separate method for single-module resolution via proxy.golang.org. `encode_go_module_path()` implements the module proxy protocol (uppercase → `!lowercase`).

**Impact**: Search returns multiple results (not just single-module lookup). Download counts remain 0 (no API available).

### 14.13 `lua.rs` Silent Error Swallowing in `search_github_repos` — Post-Review Fix

**Original plan**: Not specified.

**Actual implementation**: Initial implementation silently returned empty `Vec` on HTTP errors in `search_github_repos()` with no logging. Post-review fix added `tracing::warn!` to all three failure paths (request failure, non-success status with status code, JSON parse failure).

**Impact**: GitHub rate limits and errors are now observable in logs.

### 14.14 SearchEngine Uses `ZenService` (not `ZenDb`) and Dispatches Recursive Mode

**Original plan**: Orchestrator examples mixed `ZenDb` and `ZenService`, and described recursive mode as direct `RecursiveQueryEngine` usage outside orchestrator.

**Actual implementation**: `SearchEngine` stores `&ZenService` (matching `fts.rs` API), and `SearchMode::Recursive` is routed through orchestrator helper logic:
- package-scoped path (`ecosystem + package + version`) uses `from_source_store()`
- fallback path uses `from_directory(".")`

**Impact**: Unified mode dispatch surface in `SearchEngine`, simpler CLI integration, and package-mode recursive queries without extra CLI-side branching.

### 14.15 Reference Graph Schema Tightened (`NOT NULL`) After Review

**Original plan**: `symbol_refs.doc` and `ref_edges.evidence` were nullable in DDL while Rust types used `String`.

**Actual implementation**: DDL updated to `doc TEXT NOT NULL` and `evidence TEXT NOT NULL` to match Rust model and insertion behavior.

**Impact**: Schema/type consistency and clearer invariants for recursive graph persistence.

### 14.16 Budget Exhaustion Loop Exit Corrected in Recursive Engine

**Original plan**: Did not specify exact control-flow for budget exhaustion in nested loops.

**Actual implementation**: Budget check now exits the outer file loop (labeled break) instead of only the inner symbol loop.

**Impact**: Prevents unnecessary iteration after budget exhaustion; behavior is deterministic and easier to reason about.

### Template for Future Entries

```
### 14.X <Title>

**Original plan**: ...
**Actual implementation**: ...
**Impact**: ...
```

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Data architecture (Lance + Turso): [02-data-architecture.md](./02-data-architecture.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs (zen-registry §9, zen-search §10): [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan (Phase 4 tasks): [07-implementation-plan.md](./07-implementation-plan.md)
- Zen grep design: [13-zen-grep-design.md](./13-zen-grep-design.md)
- Phase 1 plan: [19-phase1-foundation-plan.md](./19-phase1-foundation-plan.md)
- Phase 2 plan: [20-phase2-storage-layer-plan.md](./20-phase2-storage-layer-plan.md)
- Phase 3 plan: [23-phase3-parsing-indexing-plan.md](./23-phase3-parsing-indexing-plan.md)
- Recursive query spike (RLM): [21-rlm-recursive-query-spike-plan.md](./21-rlm-recursive-query-spike-plan.md)
- Decision graph spike: [22-decision-graph-rustworkx-spike-plan.md](./22-decision-graph-rustworkx-spike-plan.md)
- Validated spike code:
  - `zen-lake/src/spike_duckdb.rs` (spike 0.4 — FLOAT[], cosine similarity)
  - `zen-search/src/spike_grep.rs` (spike 0.14 — 26 tests, two-engine grep)
  - `zen-search/src/spike_recursive_query.rs` (spike 0.21 — 17 tests, RLM recursive query)
  - `zen-search/src/spike_graph_algorithms.rs` (spike 0.22 — 54 tests, rustworkx-core)

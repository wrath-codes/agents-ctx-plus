# Phase 3: Parsing & Indexing Pipeline — Implementation Plan

**Version**: 2026-02-13 (rev 4 — delta plan from implemented zen-parser baseline)
**Status**: In Progress — Stream A substantially complete, Streams B/C/D pending
**Depends on**: Phase 1 (all tasks DONE — 127 tests), Phase 0 (spikes 0.4, 0.5, 0.6, 0.8, 0.14, 0.18, 0.19, 0.20, 0.21)
**Produces**: Milestone 3 — `cargo test -p zen-parser -p zen-embeddings -p zen-lake` passes, full clone→parse→embed→store pipeline works end-to-end

> **⚠️ Storage Scope**: Phase 3 implements a **local-only DuckDB cache backend** (`LocalLakeBackend`) for offline search. This is explicitly a **temporary stepping stone** — not the production storage architecture. Production persistence (Lance datasets on R2 + Turso catalog registration with visibility scoping) is Phase 8/9. See [Phase Boundary §13](#13-phase-boundary--what-phase-89-replaces) for the exact replacement map.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State (as of 2026-02-13)](#2-current-state-as-of-2026-02-13)
3. [Key Decisions](#3-key-decisions)
4. [Architecture: Three-Crate Split](#4-architecture-three-crate-split)
5. [PR 1 — Stream A: zen-parser (Reconciliation)](#5-pr-1--stream-a-zen-parser-reconciliation)
6. [PR 2 — Stream B: zen-embeddings](#6-pr-2--stream-b-zen-embeddings)
7. [PR 3 — Stream C: zen-lake Storage](#7-pr-3--stream-c-zen-lake-storage)
8. [PR 4 — Stream D: Indexing Pipeline + Walker](#8-pr-4--stream-d-indexing-pipeline--walker)
9. [Execution Order](#9-execution-order)
10. [Gotchas & Warnings](#10-gotchas--warnings)
11. [Milestone 3 Validation](#11-milestone-3-validation)
12. [Validation Traceability Matrix](#12-validation-traceability-matrix)
13. [Phase Boundary — What Phase 8/9 Replaces](#13-phase-boundary--what-phase-89-replaces)
14. [Mismatch Log — Plan vs. Implementation](#14-mismatch-log--plan-vs-implementation)

---

## 1. Overview

**Goal**: ast-grep-based extraction across all supported languages, fastembed integration, **local DuckDB cache storage** (temporary — production storage is Lance on R2 + Turso catalog in Phase 8/9), source file caching for `znt grep`, and the local indexing pipeline (clone → walk → parse → extract → embed → store to DuckDB cache).

**Crate status summary**:
- `zen-parser` — **substantially implemented**: 24 language dispatchers (20 builtin + 4 custom-lane), 399 source files in extractors tree, 1250 tests passing, types module tree with per-language `*MetadataExt` traits, 30 test fixture files
- `zen-embeddings` — **stub only**: spike module behind `#[cfg(test)]`, no production code
- `zen-lake` — **stub only**: 4 spike modules behind `#[cfg(test)]`, no production code
- `zen-search` — **stub only**: 3 spike modules behind `#[cfg(test)]`, no `walk.rs` yet
- `zen-cli` — **no pipeline module**: `pipeline.rs` does not exist yet

**Remaining dependency changes**:
- `zen-embeddings`: promote `dirs` from `[dev-dependencies]` to `[dependencies]` (for `~/.zenith/cache/fastembed/` path)
- `zen-search`: add `zen-parser.workspace = true` (for `is_test_file/is_test_dir` in walker)
- Workspace: add `sha2` if not present (for deterministic symbol IDs in pipeline)

**Remaining estimated deliverables**: ~15 new files, ~2000 LOC production code (embeddings + lake + walker + pipeline + parser gaps), ~500 LOC tests (integration)

**PR strategy**: 4 PRs by stream. Stream A is reconciliation/cleanup. Streams B, C, D are new implementation.

| PR | Stream | Contents | Status |
|----|--------|----------|--------|
| PR 1 | A: zen-parser (reconciliation) | `extract_api()` orchestrator, `test_files.rs`, `doc_chunker.rs` | Remaining gaps |
| PR 2 | B: zen-embeddings | EmbeddingEngine, error type | Not started |
| PR 3 | C: zen-lake | DuckDB local cache schema, ZenLake struct, store_symbols/store_doc_chunks | Not started |
| PR 3b | C2: source_files | Separate DuckDB for source file caching (`.zenith/source_files.duckdb`) | Not started |
| PR 4 | D: Pipeline + Walker | Walk factory (zen-search), indexing pipeline (**zen-cli**), doc chunker integration | Not started |

---

## 2. Current State (as of 2026-02-13)

### zen-parser — Substantially Implemented

| Aspect | Status | Detail |
|--------|--------|--------|
| **Language dispatchers** | 24/24 | All 20 builtin `SupportLang` + 4 custom-lane (Markdown, TOML, RST, Svelte). Every dispatcher has `pub fn extract()` in `src/extractors/dispatcher/<lang>.rs`. |
| **Extractor processors** | 24/24 | Each language has a `<lang>/processors/` directory or `processors.rs` file. C has 5 processor files, C++ has 5, PHP has 6. |
| **Extractor tests** | 24/24 | Every language directory has a `tests/` subdirectory. Total: **1250 tests passing**. |
| **Test fixtures** | 30 files | `tests/fixtures/` contains sample files for: `.rs`, `.py`, `.ts`, `.tsx`, `.js`, `.go`, `.ex`, `.c`, `.cpp`, `.cs`, `.css`, `.hs`, `.html`, `.java`, `.json`, `.lua`, `.php`, `.rb`, `.sh`, `.yaml`, `.md`, `.toml`, `.rst`, `.svelte` plus edge cases. |
| **Types** | Refactored | `src/types/` module tree: `ParsedItem`, `SymbolKind` (19 variants), `Visibility`, `SymbolMetadata`, `DocSections`. Per-language `*MetadataExt` traits in `symbol_metadata/`. TYPES_REFACTOR_PLAN.md Sessions 1+2 complete. |
| **Conformance** | Validated | `dispatcher/conformance.rs`: cross-language Constructor, Property, Field, owner_name/owner_kind taxonomy. |
| **Custom parsers** | Working | `parser.rs`: `MarkdownLang`, `TomlLang`, `RstLang`, `SvelteLang` with `detect_language_ext()`. |
| **Shared helpers** | Working | `extractors/helpers.rs`: `extract_source()`, `extract_signature()`. |
| **`extract_api()`** | **MISSING** | No top-level orchestrator. Callers must manually dispatch. |
| **`test_files.rs`** | **MISSING** | Test file/dir detection not implemented. |
| **`doc_chunker.rs`** | **MISSING** | Document section chunking not implemented. |

**Dispatcher signature families** (design note for `extract_api()` orchestrator):
- `extract(root)` — 16 dispatchers (csharp, css, elixir, go, haskell, html, java, javascript, json, lua, markdown, php, python, ruby, svelte, toml, rst, yaml)
- `extract(root, source: &str)` — 4 dispatchers (bash, c, cpp, rust)
- `extract(root, lang: SupportLang)` — 2 dispatchers (typescript, tsx)

The custom-lane dispatchers (markdown, rst, svelte, toml) use generic `Doc` bounds (no `Lang = SupportLang`), requiring separate parse functions (`parse_markdown_source()` etc.) from `parser.rs`.

### zen-embeddings — Stub Only

| Aspect | Status | Detail |
|--------|--------|--------|
| **Production code** | None | `lib.rs` only contains `#[cfg(test)] mod spike_fastembed;` |
| **Spike** | Validated | `spike_fastembed.rs` confirms `AllMiniLML6V2` 384-dim, determinism, batch, `&mut self` API |
| **Cargo.toml** | Ready | `fastembed`, `dirs` (dev-only), `thiserror`, `tracing` already declared |

### zen-lake — Stub Only

| Aspect | Status | Detail |
|--------|--------|--------|
| **Production code** | None | `lib.rs` only contains 4 `#[cfg(test)] mod spike_*;` declarations |
| **Spikes** | Validated | `spike_duckdb.rs` (CRUD, Appender, FLOAT[], JSON, persistence), `spike_duckdb_vss.rs`, `spike_r2_parquet.rs`, `spike_native_lance.rs` |
| **Cargo.toml** | Ready | `duckdb` (bundled), `lancedb`/`arrow-*`/`serde_arrow` in dev-deps only |

### zen-search — Stub Only

| Aspect | Status | Detail |
|--------|--------|--------|
| **Production code** | None | 3 spike modules behind `#[cfg(test)]` |
| **`walk.rs`** | Not started | `grep` + `ignore` crate deps are in Cargo.toml |

### zen-cli — No Pipeline

| Aspect | Status | Detail |
|--------|--------|--------|
| **`pipeline.rs`** | Not started | Orchestration module not created |

### zen-core — Phase 1 DONE

Unchanged. 15 entity structs, 14 enums. `ParsedItem`/`SymbolKind`/`Visibility` live in zen-parser (not zen-core).

All extraction patterns exist in validated spike code (`spike_ast_grep.rs`, 19 tests) and design docs (`05-crate-designs.md` §7). They need to be promoted to production modules.

---

## 3. Key Decisions

All decisions are backed by validated spike results.

### 3.1 KindMatcher-First Extraction Strategy

**Decision**: Use ast-grep `KindMatcher` as the primary extraction strategy for all languages, with pattern matching reserved for specific structural queries.

**Rationale**: Spike 0.8 proved that pattern matching is fragile for Rust — `fn $NAME() { $$$ }` does NOT match functions with return types or generics. `KindMatcher` finds all nodes of a given kind regardless of structure. This matches the klaw approach.

**Validated in**: spike 0.8, finding (a): "Pattern matching is fragile for Rust — use `KindMatcher` as primary extraction strategy (klaw approach), patterns only for specific structural queries."

### 3.2 Two-Tier Extraction Fallback

**Decision**: ast-grep extraction → regex fallback. If ast-grep returns no items, try regex. If regex also fails, return empty `Vec`.

**Rationale**: Some files may fail to parse (encoding issues, syntax errors). Regex catches common patterns even when AST parsing fails. Validated in design docs (`05-crate-designs.md` §7).

### 3.3 ParsedItem Lives in zen-parser, Not zen-core

**Decision**: `ParsedItem`, `SymbolMetadata`, `DocSections`, `SymbolKind`, `Visibility` are defined in `zen-parser::types`. They are NOT zen-core types.

**Rationale**: These types are parsing-specific and carry language-specific metadata. zen-core holds entity types for the database layer. The mapping `ParsedItem → ApiSymbolRow` (for lake storage) happens in the indexing pipeline (`zen-cli/src/pipeline.rs`).

### 3.4 Local DuckDB Cache (Phase 3) — Temporary, Replaced by Lance + Turso (Phase 8/9)

**Decision**: Phase 3 uses two local DuckDB files as **temporary cache backends**:
- `.zenith/lake/cache.duckdb` — `api_symbols`, `doc_chunks`, `indexed_packages` (local cache only)
- `.zenith/source_files.duckdb` — `source_files` table (permanent local store per [02-data-architecture.md §11](./02-data-architecture.md))

R2 writes via lancedb and Turso catalog registration are deferred to Phase 8/9.

**Rationale**: The local DuckDB cache provides immediate offline usability — `znt search` works with brute-force `array_cosine_similarity()`. This keeps Phase 3 focused on the extraction pipeline while the **production architecture** (Lance on R2 for search data + Turso catalog for discovery/visibility/dedup) is implemented in Phase 8/9.

**What is temporary vs permanent**:
- **Temporary** (replaced in Phase 8/9): `api_symbols` and `doc_chunks` tables in `cache.duckdb`. Production equivalents are Lance datasets on R2, discovered via Turso `dl_data_file` catalog. The `indexed_packages` table is a local-only tracking cache — it does NOT replace the Turso catalog's `dl_data_file`/`dl_snapshot` tables which handle global dedup, visibility scoping (public/team/private), and crowdsourced indexing.
- **Permanent**: `source_files` in `.zenith/source_files.duckdb` stays local forever (large, not shared, not vectorized — see [02 §11](./02-data-architecture.md)).

**Phase 8/9 replacement scope**: See [§13 Phase Boundary](#13-phase-boundary--what-phase-89-replaces).

> **Alignment note**: The production architecture per [02-data-architecture.md](./02-data-architecture.md) is: Turso catalog (dl_data_file, dl_snapshot) → lancedb writes to R2 → DuckDB lance extension reads. Phase 3's local DuckDB cache is an offline-only subset that does NOT implement visibility scoping, crowdsourced dedup, or cloud persistence. These arrive in Phase 8/9.

### 3.5 Embeddings Stored as FLOAT[] (Not FLOAT[384]) — Local Cache Only

**Decision**: DuckDB `api_symbols.embedding` and `doc_chunks.embedding` columns use `FLOAT[]` (variable-length), not `FLOAT[384]`. Cast to `FLOAT[384]` at query time for `array_cosine_similarity()`.

**Rationale**: Validated in spike 0.4 — DuckDB `FLOAT[N]` enforces dimension at insert time, which is correct. However, `FLOAT[]` is more flexible for the Appender API and avoids issues with Parquet roundtrip (spike 0.5 finding: Parquet strips fixed array dimensions). The cast `::FLOAT[384]` at query time is trivial.

**Production note**: In Phase 8/9, embeddings move to Lance datasets on R2 using `FixedSizeList(384)` (validated in spike 0.19 — `serde_arrow` requires explicit override for fixed-size arrays). Lance provides persistent IVF-PQ vector indexes and BM25 FTS indexes, replacing the brute-force `array_cosine_similarity()` used here. The DuckDB `FLOAT[]` column schema is **not** the production schema.

### 3.6 fastembed API Is Synchronous (`&mut self`)

**Decision**: `EmbeddingEngine` wraps fastembed's synchronous API. Callers use `tokio::task::spawn_blocking()` from async code.

**Rationale**: Spike 0.6 confirmed `embed()` takes `&mut self`, not `&self`. The ONNX runtime does CPU-bound work that should not block the async executor. `spawn_blocking` is the standard Tokio pattern for this.

### 3.7 Model Choice: AllMiniLML6V2 (384-dim)

**Decision**: Use `EmbeddingModel::AllMiniLML6V2` (Mean pooling, 384-dim, ~80MB).

**Rationale**: Spike 0.6 validated both `BGESmallENV15` and `AllMiniLML6V2`. AllMiniLML6V2 is the design model — smaller, simpler (no query/passage prefix behavior), same dimensionality. BGE requires different prefixes for queries vs passages which adds complexity.

### 3.8 Cache Directory: `~/.zenith/cache/fastembed/`

**Decision**: Use `with_cache_dir()` to set model cache to `~/.zenith/cache/fastembed/`.

**Rationale**: Spike 0.6 gotcha: fastembed default cache is `.fastembed_cache` (relative to CWD), which is unstable and pollutes project directories. `~/.zenith/cache/fastembed/` is a stable, user-global location.

### 3.9 Walker Factory in zen-search, Pipeline Orchestration in zen-cli

**Decision**: The `walk.rs` file walker lives in `zen-search`. The indexing pipeline orchestration (`pipeline.rs`) lives in `zen-cli`, not `zen-lake`.

**Rationale**: The walker is shared between the indexing pipeline (Phase 3, task 3.14) and `znt grep` local mode (Phase 4, task 4.11). `zen-search` has the `ignore` crate dependency already. The pipeline orchestrator lives in `zen-cli` because: (a) it's the only consumer, (b) it needs both `zen-parser` and `zen-search` which would create a circular dependency if placed in `zen-lake` (zen-lake ← zen-search ← zen-lake), and (c) zen-lake should remain a pure storage/query crate per the architecture.

### 3.10 Doc Chunking by Section Headings

**Decision**: Split markdown/rst/txt documentation files by section headings (`# Heading`), chunk to ~512 tokens (~2048 chars). Each chunk gets its own embedding.

**Rationale**: Section-level chunking preserves semantic boundaries. 512 tokens fits comfortably within fastembed's context window while providing enough context for meaningful embeddings.

### 3.11 Signature Extraction: Text Before First `{` or `;`, Whitespace-Normalized

**Decision**: Extract signature as `node.text()` up to the first `{` or `;`, then **normalize whitespace** (collapse newlines to spaces, collapse runs of whitespace to single space). No body leaks.

**Rationale**: Validated in spike 0.8 and `05-crate-designs.md` §7. Simple, reliable, works across all languages. The signature is what gets embedded and stored — including the body would dilute the embedding. Whitespace normalization (spike 0.21 finding) ensures deterministic signatures regardless of source formatting, which matters for embedding stability and deterministic symbol IDs.

### 3.12 async/unsafe Detection via Modifiers Node

**Decision**: Check `function_modifiers` child node for `async`/`unsafe`, not direct text matching.

**Rationale**: Spike 0.8 finding (b): "`async`/`unsafe` appear as children of `function_modifiers` node, not as direct children — walk into modifiers for detection." Direct `text().starts_with("async")` is fragile because `pub async fn` starts with `pub`.

---

## 4. Architecture: Three-Crate Split

### Dependency Flow (Phase 3)

```
zen-core (types)
    │
    ├──► zen-parser (ast-grep extraction)
    │       │
    │       └──► zen-core
    │
    ├──► zen-embeddings (fastembed)
    │       │
    │       └──► zen-core
    │
    ├──► zen-lake (DuckDB local cache storage — NO pipeline logic)
    │       │
    │       └──► zen-core, zen-config, zen-embeddings
    │
    ├──► zen-search (walker factory + search)
    │       │
    │       └──► zen-core, zen-lake, zen-embeddings, zen-parser
    │
    └──► zen-cli (pipeline orchestration — the ONLY consumer)
            │
            └──► zen-core, zen-parser, zen-embeddings, zen-lake, zen-search
```

> **Note**: zen-lake does NOT depend on zen-parser or zen-search. Pipeline orchestration (walk → parse → embed → store) lives in zen-cli, which depends on all crates. This avoids the circular dependency zen-lake ↔ zen-search.

### Module Structure After Phase 3

> **Updated 2026-02-13**: Reflects actual zen-parser implementation + remaining stubs.

```
zen-parser/src/                          # ── IMPLEMENTED (except items marked PENDING) ──
├── lib.rs                               # Public API: detect_language(), parse_source(), extract_api() [PENDING]
├── error.rs                             # ParserError enum
├── parser.rs                            # ast-grep wrapper, SupportLang mapping, custom language parsers
│   ├── markdown_lang.rs                 # MarkdownLang (tree-sitter-md)
│   ├── toml_lang.rs                     # TomlLang (tree-sitter-toml-ng)
│   ├── rst_lang.rs                      # RstLang (tree-sitter-rst)
│   └── svelte_lang.rs                   # SvelteLang (tree-sitter-svelte-next)
├── types/                               # Module tree (TYPES_REFACTOR_PLAN Sessions 1+2 complete)
│   ├── mod.rs                           # Re-exports: ParsedItem, SymbolKind, Visibility, SymbolMetadata, DocSections
│   ├── parsed_item.rs                   # ParsedItem struct
│   ├── symbol_kind.rs                   # SymbolKind enum (19 variants) + Display
│   ├── visibility.rs                    # Visibility enum + Display
│   ├── doc_sections.rs                  # DocSections struct
│   └── symbol_metadata/                 # SymbolMetadata + per-language ext traits
│       ├── mod.rs                       # SymbolMetadata struct (~50+ fields)
│       ├── common.rs                    # CommonMetadataExt trait
│       ├── bash.rs .. typescript.rs     # Per-language *MetadataExt traits (13 files)
├── test_files.rs                        # [PENDING] is_test_file(), is_test_dir()
├── doc_chunker.rs                       # [PENDING] split_into_chunks() for README/docs
├── spike_ast_grep.rs                    # #[cfg(test)] — spike 0.8 validation
└── extractors/
    ├── mod.rs                           # Re-exports all dispatcher modules
    ├── helpers.rs                        # Shared: extract_source(), extract_signature()
    └── dispatcher/
        ├── mod.rs                       # pub mod for all 24 languages + #[cfg(test)] conformance
        ├── rust.rs                      # Rust dispatcher → rust/processors/ via #[path]
        ├── python.rs                    # Python dispatcher → python/processors/ via #[path]
        ├── typescript.rs                # TypeScript dispatcher (root, lang)
        ├── tsx.rs                       # TSX dispatcher (root, lang) — React detection
        ├── javascript.rs                # JavaScript dispatcher (root)
        ├── c.rs                         # C dispatcher (root, source)
        ├── cpp.rs                       # C++ dispatcher (root, source)
        ├── ... (16 more)               # All other languages
        └── conformance.rs               # #[cfg(test)] cross-language taxonomy tests
    # Each language also has: extractors/<lang>/processors/, helpers.rs, tests/
    # These are #[path]-imported by the dispatcher, not mod-declared.

zen-embeddings/src/                      # ── STUB (pending PR 2) ──
├── lib.rs                               # [PENDING] EmbeddingEngine struct
├── error.rs                             # [PENDING] EmbeddingError enum
└── spike_fastembed.rs                   # #[cfg(test)] — spike 0.6 validation

zen-lake/src/                            # ── STUB (pending PR 3) ──
├── lib.rs                               # [PENDING] ZenLake struct, open_local()
├── error.rs                             # [PENDING] LakeError enum
├── schemas.rs                           # [PENDING] DuckDB table DDL (LOCAL CACHE ONLY)
├── store.rs                             # [PENDING] store_symbols(), store_doc_chunks()
├── source_files.rs                      # [PENDING] SourceFileStore (.zenith/source_files.duckdb)
└── spike_*.rs                           # #[cfg(test)] — spikes 0.4, 0.5, 0.18, 0.19

zen-search/src/                          # ── STUB (pending PR 4) ──
├── lib.rs                               # [PENDING] + re-export walk module
├── walk.rs                              # [PENDING] WalkMode, build_walker()
└── spike_*.rs                           # #[cfg(test)] — spikes 0.14, 0.21, 0.22

zen-cli/src/                             # ── PENDING PR 4 ──
└── pipeline.rs                          # [PENDING] IndexingPipeline orchestration
```

---

## 5. PR 1 — Stream A: zen-parser (Reconciliation)

Stream A is **substantially complete**. This PR covers the remaining gaps — not the extraction logic itself, which is already implemented across 24 language dispatchers with 1250 passing tests.

### What Already Exists (DO NOT recreate)

| Artifact | Location | Status |
|----------|----------|--------|
| `ParserError` | `src/error.rs` | **DONE** — `ParseFailed`, `UnsupportedLanguage`, `ExtractionFailed`, `Io` variants |
| `ParsedItem`, `SymbolKind`, `Visibility`, `SymbolMetadata`, `DocSections` | `src/types/` module tree | **DONE** — Full module split with per-language `*MetadataExt` traits. `SymbolKind` has 19 variants (added Constructor, Field, Property, Event, Indexer, Component beyond original plan). |
| `detect_language()` | `src/parser.rs` | **DONE** — 20 builtin extensions mapped |
| `detect_language_ext()` | `src/parser.rs` | **DONE** — Adds Markdown, TOML, RST, Svelte detection |
| `parse_source()` + custom parse functions | `src/parser.rs` | **DONE** — `parse_source()`, `parse_markdown_source()`, `parse_toml_source()`, `parse_rst_source()`, `parse_svelte_source()` |
| Shared extraction helpers | `src/extractors/helpers.rs` | **DONE** — `extract_source()`, `extract_signature()` |
| Language dispatchers | `src/extractors/dispatcher/<lang>.rs` (24 files) | **DONE** — All have `pub fn extract()`. Architecture: dispatcher uses `#[path]` to pull in `<lang>/processors/` and `<lang>/helpers.rs`. |
| Language processors | `src/extractors/<lang>/processors/` (24 dirs) | **DONE** — Rich extraction for all 24 languages |
| Language tests | `src/extractors/<lang>/tests/` (24 dirs) | **DONE** — 1250 tests passing |
| Conformance tests | `src/extractors/dispatcher/conformance.rs` | **DONE** — Cross-language Constructor, Field, Property, owner_name taxonomy |
| Test fixtures | `tests/fixtures/` (30 files) | **DONE** |
| `lib.rs` public API | `src/lib.rs` | **DONE** — Exports `ParserError`, `ParsedItem`, `SymbolKind`, `SymbolMetadata`, `DocSections`, `Visibility`, `detect_language`, `detect_language_ext`, all parse functions |

### What Needs to Be Added (PR 1 scope)

#### A1. `src/test_files.rs` — Test File/Dir Detection (task 3.9)

```rust
const TEST_DIRS: &[&str] = &[
    "test", "tests", "spec", "specs", "__tests__", "__mocks__",
    "__snapshots__", "testdata", "fixtures", "e2e",
    "integration_tests", "unit_tests", "benches", "benchmarks", "examples",
];

pub fn is_test_dir(dir_name: &str) -> bool {
    TEST_DIRS.contains(&dir_name)
}

pub fn is_test_file(file_name: &str) -> bool {
    let name = file_name.to_lowercase();
    // Go
    name.ends_with("_test.go") ||
    // Rust
    name.ends_with("_test.rs") ||
    // JS/TS
    name.ends_with(".test.js") || name.ends_with(".test.ts") ||
    name.ends_with(".test.tsx") || name.ends_with(".test.jsx") ||
    name.ends_with(".spec.js") || name.ends_with(".spec.ts") ||
    name.ends_with(".spec.tsx") || name.ends_with(".spec.jsx") ||
    // Python
    name.starts_with("test_") || name.ends_with("_test.py") ||
    // Elixir
    name.ends_with("_test.exs") ||
    // General
    name == "conftest.py" || name == "setup_test.go"
}
```

**Source**: `03-architecture-overview.md` §7 (test file patterns).

Update `src/lib.rs` to add:
```rust
pub mod test_files;
pub use test_files::{is_test_file, is_test_dir};
```

#### A2. `src/doc_chunker.rs` — Documentation Chunker (task 3.15)

Splits markdown/rst/txt files by section headings, chunks to ~512 tokens.

```rust
#[derive(Debug, Clone)]
pub struct DocChunk {
    pub title: Option<String>,
    pub content: String,
    pub chunk_index: u32,
    pub source_file: String,
    pub format: String,
}

const MAX_CHUNK_CHARS: usize = 2048; // ~512 tokens

pub fn chunk_document(
    content: &str,
    source_file: &str,
) -> Vec<DocChunk> {
    let format = detect_doc_format(source_file);
    let sections = split_by_headings(content, &format);

    let mut chunks = Vec::new();
    let mut chunk_index = 0u32;

    for (title, body) in sections {
        if body.trim().is_empty() {
            continue;
        }
        // Split oversized sections into sub-chunks
        for sub_chunk in split_to_max_size(&body, MAX_CHUNK_CHARS) {
            chunks.push(DocChunk {
                title: if chunk_index == 0 || !title.is_empty() {
                    Some(title.clone())
                } else {
                    None
                },
                content: sub_chunk,
                chunk_index,
                source_file: source_file.to_string(),
                format: format.clone(),
            });
            chunk_index += 1;
        }
    }
    chunks
}
```

Update `src/lib.rs` to add:
```rust
pub mod doc_chunker;
pub use doc_chunker::{chunk_document, DocChunk};
```

#### A3. `extract_api()` — Top-Level Orchestrator (task 3.10)

Unified entrypoint that detects language and dispatches to the correct extractor. Must handle the three different dispatcher signatures:
- `extract(root)` — 16 languages
- `extract(root, source)` — bash, c, cpp, rust
- `extract(root, lang)` — typescript, tsx

Also handles custom-lane languages (markdown, rst, svelte, toml) via `detect_language_ext()` and their separate parse functions.

Two-tier fallback: ast-grep → regex.

```rust
/// Extract API symbols from source code for any supported language.
///
/// Detects the language from `file_path`, parses with ast-grep (or custom parser
/// for Markdown/TOML/RST/Svelte), and extracts symbols. Falls back to regex
/// extraction if ast-grep yields no results.
pub fn extract_api(
    source: &str,
    file_path: &str,
) -> Result<Vec<ParsedItem>, ParserError> {
    let lang = detect_language_ext(file_path)
        .ok_or_else(|| ParserError::UnsupportedLanguage(file_path.to_string()))?;

    // Tier 1: ast-grep extraction via language dispatcher
    let items = match lang {
        DetectedLanguage::Builtin(builtin) => extract_builtin(source, builtin)?,
        DetectedLanguage::Markdown => {
            let root = parse_markdown_source(source);
            extractors::markdown::extract(&root)?
        }
        DetectedLanguage::Toml => {
            let root = parse_toml_source(source);
            extractors::toml::extract(&root)?
        }
        DetectedLanguage::Rst => {
            let root = parse_rst_source(source);
            extractors::rst::extract(&root)?
        }
        DetectedLanguage::Svelte => {
            let root = parse_svelte_source(source);
            extractors::svelte::extract(&root)?
        }
    };

    if !items.is_empty() {
        return Ok(items);
    }

    // Tier 2: Regex fallback
    tracing::debug!("ast-grep returned no items for {file_path}, trying regex fallback");
    extract_with_regex(source)
}

fn extract_builtin(
    source: &str,
    lang: SupportLang,
) -> Result<Vec<ParsedItem>, ParserError> {
    let root = parse_source(source, lang);
    match lang {
        SupportLang::Rust => extractors::rust::extract(&root, source),
        SupportLang::Python => extractors::python::extract(&root),
        SupportLang::TypeScript => extractors::typescript::extract(&root, lang),
        SupportLang::Tsx => extractors::tsx::extract(&root, lang),
        SupportLang::JavaScript => extractors::javascript::extract(&root),
        SupportLang::Go => extractors::go::extract(&root),
        SupportLang::Elixir => extractors::elixir::extract(&root),
        SupportLang::C => extractors::c::extract(&root, source),
        SupportLang::Cpp => extractors::cpp::extract(&root, source),
        SupportLang::CSharp => extractors::csharp::extract(&root),
        SupportLang::Css => extractors::css::extract(&root),
        SupportLang::Haskell => extractors::haskell::extract(&root),
        SupportLang::Html => extractors::html::extract(&root),
        SupportLang::Java => extractors::java::extract(&root),
        SupportLang::Json => extractors::json::extract(&root),
        SupportLang::Lua => extractors::lua::extract(&root),
        SupportLang::Php => extractors::php::extract(&root),
        SupportLang::Ruby => extractors::ruby::extract(&root),
        SupportLang::Bash => extractors::bash::extract(&root, source),
        SupportLang::Yaml => extractors::yaml::extract(&root),
        _ => Err(ParserError::UnsupportedLanguage(format!("{lang:?}"))),
    }
}
```

Update `src/lib.rs` to add:
```rust
pub use extractors::extract_api;  // or wherever extract_api lives
```

**Design note**: The `extract_api()` function takes `file_path: &str` (not `SupportLang`) because it needs to handle custom-lane languages that are outside the `SupportLang` enum. The pipeline in `zen-cli/src/pipeline.rs` calls `extract_api(source, rel_path)` without needing to know about dispatcher signatures.

### A4. Tests for New Modules

**Unit tests** (`src/test_files.rs` tests):
- `is_test_file()` returns true for all test file patterns
- `is_test_file()` returns false for production files
- `is_test_dir()` returns true for all test directory names

**Unit tests** (`src/doc_chunker.rs` tests):
- Chunk a markdown file with headings
- Verify chunk boundaries align with headings
- Verify oversized sections get sub-chunked
- Verify empty sections are skipped

**Integration tests** for `extract_api()`:
- Call `extract_api(source, "sample.rs")` → dispatches to Rust extractor, returns items
- Call `extract_api(source, "sample.py")` → dispatches to Python extractor
- Call `extract_api(source, "sample.md")` → dispatches to Markdown extractor (custom-lane)
- Call `extract_api(source, "unknown.xyz")` → returns `UnsupportedLanguage` error
- Two-tier fallback: empty ast-grep result triggers regex

### A5. Existing Test Coverage (reference — already passing)

The following test suites are already complete and passing (1250 tests total). They are listed here for reference but are **not part of PR 1 scope**.

**Extractor tests** (per-language, in `src/extractors/<lang>/tests/`):
- Rust (5 test files): functions, structs, enums, traits, impl blocks, async/unsafe detection, generics, lifetimes, doc comments, attributes, visibility, error types, PyO3
- Python (7 test files): classes, functions, decorators, docstrings (Google/NumPy/Sphinx), dataclass/pydantic/protocol, generators
- TypeScript (13 test files): functions, classes, interfaces, type aliases, enums, exports, JSDoc, ambient declarations, namespaces
- JavaScript (7 test files): functions, classes, arrow functions, generators, async, exports, constants
- TSX (9 test files): React components, hooks, HOC, forward_ref, memo, error boundaries
- Go (14 test files): exported/unexported functions, types, methods, struct fields, doc comments
- Elixir (15 test files): defmodule, def/defp, defmacro, doc attrs
- C (28 test files): functions, structs, enums, unions, typedefs, preprocessor, variables, arrays, inline
- C++ (38 test files): classes, templates, namespaces, qualified identifiers, template instantiation
- C# (5 test files): types, namespaces, members, visibility, events/indexers/operators, using directives
- Plus: Haskell, Java, Lua, PHP, Ruby, Bash, HTML, CSS, JSON, YAML, Markdown, TOML, RST, Svelte
- Conformance: cross-language Constructor, Property, Field, owner taxonomy

---

## 6. PR 2 — Stream B: zen-embeddings

The smallest PR. Thin wrapper around fastembed.

**Can run in parallel with PR 1** — no dependency between zen-parser and zen-embeddings.

### B1. `src/error.rs` — EmbeddingError

```rust
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Model initialization failed: {0}")]
    InitFailed(String),

    #[error("Embedding generation failed: {0}")]
    EmbedFailed(String),

    #[error("Empty result from embedding model")]
    EmptyResult,
}
```

### B2. `src/lib.rs` — EmbeddingEngine

```rust
use fastembed::{TextEmbedding, EmbeddingModel, InitOptions};

pub mod error;
pub use error::EmbeddingError;

pub struct EmbeddingEngine {
    model: TextEmbedding,
}

impl EmbeddingEngine {
    /// Create a new embedding engine with AllMiniLML6V2 model.
    ///
    /// Model files cached to `~/.zenith/cache/fastembed/`.
    /// **Gotcha (spike 0.6)**: default cache is `.fastembed_cache` (relative CWD) — we override.
    pub fn new() -> Result<Self, EmbeddingError> {
        let cache_dir = dirs::home_dir()
            .map(|h| h.join(".zenith").join("cache").join("fastembed"))
            .unwrap_or_else(|| std::path::PathBuf::from(".fastembed_cache"));

        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            cache_dir,
            show_download_progress: true,
            ..Default::default()
        })
        .map_err(|e| EmbeddingError::InitFailed(e.to_string()))?;

        Ok(Self { model })
    }

    /// Embed a batch of texts. Returns one 384-dim vector per input.
    ///
    /// **Gotcha (spike 0.6)**: `embed()` takes `&mut self`, not `&self`.
    /// Use `tokio::task::spawn_blocking()` from async code.
    pub fn embed_batch(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.model
            .embed(texts, None)
            .map_err(|e| EmbeddingError::EmbedFailed(e.to_string()))
    }

    /// Embed a single text. Convenience wrapper around embed_batch.
    pub fn embed_single(&mut self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut results = self.embed_batch(vec![text.to_string()])?;
        results.pop().ok_or(EmbeddingError::EmptyResult)
    }

    /// Embedding vector dimensionality.
    pub const fn dimension() -> usize {
        384
    }
}

#[cfg(test)]
mod spike_fastembed;
```

### B3. Cargo.toml Update

Add `dirs` to production dependencies (for cache path):

```toml
[dependencies]
zen-core.workspace = true
fastembed.workspace = true
dirs.workspace = true          # NEW — for ~/.zenith/cache/fastembed path
thiserror.workspace = true
tracing.workspace = true
```

### B4. Tests

- **Model loads**: `EmbeddingEngine::new()` succeeds (downloads model on first run)
- **Single embed**: `embed_single("hello world")` returns 384-dim vector
- **Batch embed**: `embed_batch(["a", "b", "c"])` returns 3 vectors, each 384-dim
- **Cosine similarity**: "async runtime" and "tokio spawn" have higher similarity than "async runtime" and "cooking recipe"
- **Determinism**: Same input produces same embedding (bit-for-bit)
- **Empty text**: `embed_single("")` does not panic (returns a valid 384-dim vector)
- **Dimension check**: `EmbeddingEngine::dimension()` returns 384

---

## 7. PR 3 — Stream C: zen-lake Local Cache Storage

DuckDB **local cache** table creation + Appender-based storage. No lancedb writes (deferred to Phase 8/9). No Turso catalog registration (deferred to Phase 8/9).

> **This is NOT the production storage layer.** Production storage = Lance datasets on R2 (written by `lancedb`) + Turso catalog (`dl_data_file`/`dl_snapshot`) for discovery and visibility. See [§13](#13-phase-boundary--what-phase-89-replaces).

**Depends on**: zen-embeddings (for dimension constant only — no runtime dependency yet).

### C1. `src/error.rs` — LakeError

```rust
#[derive(Debug, thiserror::Error)]
pub enum LakeError {
    #[error("DuckDB error: {0}")]
    DuckDb(#[from] duckdb::Error),

    #[error("Lake not initialized: {0}")]
    NotInitialized(String),

    #[error("Package not found: {ecosystem}/{package}/{version}")]
    PackageNotFound {
        ecosystem: String,
        package: String,
        version: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
```

### C2. `src/schemas.rs` — DuckDB Table DDL + Lake Structs (LOCAL CACHE ONLY)

**DuckDB schema** (local cache.duckdb — **temporary**, replaced by Lance datasets in Phase 8/9):

> **Schema alignment note**: Column names and types below are designed to mirror the production Lance schema fields where possible (see [02-data-architecture.md §5](./02-data-architecture.md), spike 0.19 `ApiSymbol` struct). Phase 3's DuckDB schema is a **local superset** — it includes `return_type`, `generics`, and `metadata JSON` columns not present in the spike 0.19 production `ApiSymbol`. In Phase 8/9, a separate `ProductionApiSymbol` struct (matching spike 0.19 with `Option<bool>` fields, `created_at` via `arrow_serde`, and `FixedSizeList(384)` embedding override) will be the `serde_arrow` source for Lance writes. The Phase 3 `ApiSymbolRow` is a local-cache row type, not the Lance write type.

```rust
pub const CREATE_INDEXED_PACKAGES: &str = "
CREATE TABLE IF NOT EXISTS indexed_packages (
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    repo_url TEXT,
    description TEXT,
    license TEXT,
    downloads BIGINT,
    indexed_at TIMESTAMP DEFAULT current_timestamp,
    file_count INTEGER DEFAULT 0,
    symbol_count INTEGER DEFAULT 0,
    doc_chunk_count INTEGER DEFAULT 0,
    source_cached BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (ecosystem, package, version)
);
";

pub const CREATE_API_SYMBOLS: &str = "
CREATE TABLE IF NOT EXISTS api_symbols (
    id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    signature TEXT,
    source TEXT,
    doc_comment TEXT,
    line_start INTEGER,
    line_end INTEGER,
    visibility TEXT,
    is_async BOOLEAN DEFAULT FALSE,
    is_unsafe BOOLEAN DEFAULT FALSE,
    is_error_type BOOLEAN DEFAULT FALSE,
    returns_result BOOLEAN DEFAULT FALSE,
    return_type TEXT,
    generics TEXT,
    attributes TEXT,
    metadata JSON,
    embedding FLOAT[],
    created_at TIMESTAMP DEFAULT current_timestamp,
    PRIMARY KEY (id)
);
";

pub const CREATE_DOC_CHUNKS: &str = "
CREATE TABLE IF NOT EXISTS doc_chunks (
    id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    title TEXT,
    content TEXT NOT NULL,
    source_file TEXT,
    format TEXT,
    embedding FLOAT[],
    created_at TIMESTAMP DEFAULT current_timestamp,
    PRIMARY KEY (id)
);
";

// NOTE: source_files DDL lives in source_files.rs (separate DuckDB file per 02-data-architecture.md §11)

pub const CREATE_SYMBOL_INDEX: &str = "
CREATE INDEX IF NOT EXISTS idx_symbols_pkg
    ON api_symbols(ecosystem, package, version);
CREATE INDEX IF NOT EXISTS idx_symbols_kind
    ON api_symbols(ecosystem, package, version, kind);
CREATE INDEX IF NOT EXISTS idx_symbols_name
    ON api_symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_file_lines
    ON api_symbols(ecosystem, package, version, file_path, line_start, line_end);
";
```

**Lake structs** (for Appender insertion — mirrors DuckDB columns):

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSymbolRow {
    pub id: String,
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub file_path: String,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub source: Option<String>,
    pub doc_comment: Option<String>,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub visibility: Option<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_error_type: bool,
    pub returns_result: bool,
    pub return_type: Option<String>,
    pub generics: Option<String>,
    pub attributes: Option<String>,  // JSON array as string
    pub metadata: Option<String>,    // JSON object as string
    pub embedding: Vec<f32>,         // 384-dim
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocChunkRow {
    pub id: String,
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub chunk_index: i32,
    pub title: Option<String>,
    pub content: String,
    pub source_file: Option<String>,
    pub format: Option<String>,
    pub embedding: Vec<f32>,
}
```

### C3. `src/lib.rs` — ZenLake Struct (tasks 3.12, 3.16)

```rust
use duckdb::Connection;

pub mod error;
pub mod schemas;
pub mod store;
pub mod source_files;

pub use error::LakeError;
pub use schemas::{ApiSymbolRow, DocChunkRow};

pub struct ZenLake {
    conn: Connection,
}

impl ZenLake {
    /// Open or create a local DuckDB lake file.
    ///
    /// Creates all tables (api_symbols, doc_chunks, indexed_packages, source_files)
    /// and indexes if they don't exist.
    pub fn open_local(path: &str) -> Result<Self, LakeError> {
        let conn = Connection::open(path)?;
        let lake = Self { conn };
        lake.init_schema()?;
        Ok(lake)
    }

    /// Open an in-memory lake (for testing).
    pub fn open_in_memory() -> Result<Self, LakeError> {
        let conn = Connection::open_in_memory()?;
        let lake = Self { conn };
        lake.init_schema()?;
        Ok(lake)
    }

    /// Access the DuckDB connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn init_schema(&self) -> Result<(), LakeError> {
        self.conn.execute_batch(schemas::CREATE_INDEXED_PACKAGES)?;
        self.conn.execute_batch(schemas::CREATE_API_SYMBOLS)?;
        self.conn.execute_batch(schemas::CREATE_DOC_CHUNKS)?;
        // NOTE: source_files lives in a SEPARATE DuckDB file — see source_files.rs
        self.conn.execute_batch(schemas::CREATE_SYMBOL_INDEX)?;
        Ok(())
    }
}
```

### C4. `src/store.rs` — Store Methods (task 3.13)

Uses DuckDB Appender for bulk insert (validated in spike 0.4 — 1000 rows).

```rust
use duckdb::params;
use crate::{ZenLake, LakeError, schemas::{ApiSymbolRow, DocChunkRow}};

impl ZenLake {
    /// Store API symbols using Appender for bulk insert.
    ///
    /// **Spike 0.4 validated**: Appender bulk insert (1000 rows) works.
    pub fn store_symbols(&self, symbols: &[ApiSymbolRow]) -> Result<(), LakeError> {
        let mut appender = self.conn.appender("api_symbols")?;
        for sym in symbols {
            appender.append_row(params![
                sym.id,
                sym.ecosystem,
                sym.package,
                sym.version,
                sym.file_path,
                sym.kind,
                sym.name,
                sym.signature,
                sym.source,
                sym.doc_comment,
                sym.line_start,
                sym.line_end,
                sym.visibility,
                sym.is_async,
                sym.is_unsafe,
                sym.is_error_type,
                sym.returns_result,
                sym.return_type,
                sym.generics,
                sym.attributes,
                sym.metadata,
                // Note: embedding as Vec<f32> → FLOAT[] via duckdb params
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    /// Store doc chunks using Appender.
    pub fn store_doc_chunks(&self, chunks: &[DocChunkRow]) -> Result<(), LakeError> {
        let mut appender = self.conn.appender("doc_chunks")?;
        for chunk in chunks {
            appender.append_row(params![
                chunk.id,
                chunk.ecosystem,
                chunk.package,
                chunk.version,
                chunk.chunk_index,
                chunk.title,
                chunk.content,
                chunk.source_file,
                chunk.format,
                // embedding
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    /// Register a package as indexed.
    pub fn register_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
        repo_url: Option<&str>,
        description: Option<&str>,
        license: Option<&str>,
        downloads: Option<i64>,
        file_count: i32,
        symbol_count: i32,
        doc_chunk_count: i32,
    ) -> Result<(), LakeError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO indexed_packages
             (ecosystem, package, version, repo_url, description, license, downloads,
              file_count, symbol_count, doc_chunk_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                ecosystem, package, version, repo_url, description, license,
                downloads, file_count, symbol_count, doc_chunk_count
            ],
        )?;
        Ok(())
    }

    /// Check if a package is already indexed.
    pub fn is_package_indexed(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<bool, LakeError> {
        let mut stmt = self.conn.prepare(
            "SELECT 1 FROM indexed_packages WHERE ecosystem = ? AND package = ? AND version = ?"
        )?;
        let exists = stmt.query_row(params![ecosystem, package, version], |_| Ok(true))
            .unwrap_or(false);
        Ok(exists)
    }
}
```

### C5. `src/source_files.rs` — Source File Caching in Separate DuckDB (tasks 3.16, 3.17)

**Per [02-data-architecture.md §11](./02-data-architecture.md)**: source files live in a **separate** DuckDB file (`.zenith/source_files.duckdb`), not in the lake cache. They are large, not shared, and don't need vector search. This is a **permanent** local store (not replaced in Phase 8/9).

```rust
use duckdb::{Connection, params};
use crate::LakeError;

const CREATE_SOURCE_FILES: &str = "
CREATE TABLE IF NOT EXISTS source_files (
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
CREATE INDEX IF NOT EXISTS idx_source_pkg
    ON source_files(ecosystem, package, version);
CREATE INDEX IF NOT EXISTS idx_source_lang
    ON source_files(ecosystem, package, version, language);
";

pub struct SourceFileStore {
    conn: Connection,
}

pub struct SourceFile {
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub file_path: String,
    pub content: String,
    pub language: Option<String>,
    pub size_bytes: i32,
    pub line_count: i32,
}

impl SourceFileStore {
    /// Open or create the source files DuckDB at `.zenith/source_files.duckdb`.
    pub fn open(path: &str) -> Result<Self, LakeError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(CREATE_SOURCE_FILES)?;
        Ok(Self { conn })
    }

    /// Open an in-memory store (for testing).
    pub fn open_in_memory() -> Result<Self, LakeError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(CREATE_SOURCE_FILES)?;
        Ok(Self { conn })
    }

    /// Store source files for znt grep using Appender.
    ///
    /// Called during indexing pipeline (step 8 in 02-data-architecture.md §8).
    /// Source content is already in memory from the parsing step — zero extra I/O.
    pub fn store_source_files(&self, files: &[SourceFile]) -> Result<(), LakeError> {
        let mut appender = self.conn.appender("source_files")?;
        for f in files {
            appender.append_row(params![
                f.ecosystem, f.package, f.version, f.file_path,
                f.content, f.language, f.size_bytes, f.line_count
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
```

### C6. Tests

**ZenLake (cache.duckdb) tests**:
- **Schema creation**: `open_in_memory()` succeeds. Query `information_schema.tables` — verify 3 tables exist (api_symbols, doc_chunks, indexed_packages — NOT source_files).
- **Store + query symbols**: Insert 10 `ApiSymbolRow` via `store_symbols()`, SELECT back, verify all fields match.
- **Store + query doc chunks**: Insert 5 `DocChunkRow` via `store_doc_chunks()`, SELECT back, verify fields.
- **Register package**: `register_package()` → `is_package_indexed()` returns true.
- **Duplicate package**: `register_package()` twice with same key → INSERT OR REPLACE succeeds.
- **Embedding roundtrip**: Insert with 384-dim `Vec<f32>`, query back, verify dimensions preserved.
- **Cosine similarity query**: Insert 2 symbols with known embeddings, verify `array_cosine_similarity(embedding::FLOAT[384], ...)` returns expected score.
- **Appender bulk insert**: Insert 1000 rows via Appender, verify count matches (validated in spike 0.4).
- **File persistence**: Open file-backed lake, insert data, close, reopen, verify data persists.
- **Index existence**: After schema init, verify `idx_symbols_file_lines` and other indexes exist.

**SourceFileStore (source_files.duckdb) tests**:
- **Schema creation**: `SourceFileStore::open_in_memory()` succeeds, source_files table exists.
- **Store + query**: Insert 3 `SourceFile` → SELECT back content, verify fields.
- **Separate from lake**: SourceFileStore and ZenLake are independent connections to different files.

---

## 8. PR 4 — Stream D: Indexing Pipeline + Walker

The integration PR that ties everything together. End-to-end: clone → walk → parse → embed → store.

**Depends on**: PR 1 (zen-parser), PR 2 (zen-embeddings), PR 3 (zen-lake).

### D1. `zen-search/src/walk.rs` — Walker Factory (task 3.18)

Uses `ignore` crate for gitignore-aware file walking.

**Validated in**: spike 0.14 (5 tests on ignore crate).

```rust
use std::path::Path;
use ignore::WalkBuilder;

pub enum WalkMode {
    /// Local project: respect .gitignore, skip .zenith/, support .zenithignore
    LocalProject,
    /// Raw: no filters (for internal use, e.g., indexing cloned repos)
    Raw,
}

pub fn build_walker(
    root: &Path,
    mode: WalkMode,
    skip_tests: bool,
    include_glob: Option<&str>,
    exclude_glob: Option<&str>,
) -> ignore::Walk {
    let mut builder = WalkBuilder::new(root);
    builder.hidden(false); // Don't skip hidden files by default

    match mode {
        WalkMode::LocalProject => {
            builder.add_custom_ignore_filename(".zenithignore");
            // .zenith/ always skipped (via override)
            let mut overrides = ignore::overrides::OverrideBuilder::new(root);
            overrides.add("!.zenith/").expect("valid override");
            if let Some(glob) = include_glob {
                overrides.add(glob).expect("valid include glob");
            }
            if let Some(glob) = exclude_glob {
                overrides.add(&format!("!{glob}")).expect("valid exclude glob");
            }
            builder.overrides(overrides.build().expect("valid overrides"));
        }
        WalkMode::Raw => {
            builder.ignore(false);
            builder.git_ignore(false);
        }
    }

    if skip_tests {
        builder.filter_entry(|entry| {
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                let name = entry.file_name().to_string_lossy();
                !zen_parser::is_test_dir(&name)
            } else {
                let name = entry.file_name().to_string_lossy();
                !zen_parser::is_test_file(&name)
            }
        });
    }

    builder.build()
}
```

**Cargo.toml update** for zen-search — add `zen-parser` as production dependency (for `is_test_file/is_test_dir`):

```toml
[dependencies]
zen-parser.workspace = true    # NEW — for test file/dir detection in walker
```

### D2. `zen-cli/src/pipeline.rs` — Indexing Pipeline (task 3.14)

Orchestrates the full walk → parse → embed → store pipeline. Lives in **zen-cli** (not zen-lake) to avoid circular dependencies and because zen-cli is the only consumer.

> **Why zen-cli, not zen-lake**: zen-lake is a pure storage/query crate. The pipeline needs zen-parser (for extraction), zen-search (for walker), zen-embeddings (for vectors), AND zen-lake (for storage). Placing it in zen-lake would require zen-lake → zen-search → zen-lake (circular). zen-cli already depends on all crates.

```rust
use std::path::Path;
use zen_lake::{ZenLake, LakeError, schemas::{ApiSymbolRow, DocChunkRow}, source_files::{SourceFileStore, SourceFile}};

pub struct IndexingPipeline {
    lake: ZenLake,
    source_store: SourceFileStore,
}

pub struct IndexResult {
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub file_count: i32,
    pub symbol_count: i32,
    pub doc_chunk_count: i32,
    pub source_file_count: i32,
}

impl IndexingPipeline {
    pub fn new(lake: ZenLake, source_store: SourceFileStore) -> Self {
        Self { lake, source_store }
    }

    /// Index a local directory (already cloned/extracted).
    ///
    /// Steps:
    /// 1. Walk source files (respecting .gitignore, skipping tests)
    /// 2. Parse each file with ast-grep, extract symbols
    /// 3. Chunk documentation files (README, docs/*)
    /// 4. Generate fastembed vectors (batch)
    /// 5. Store symbols + doc chunks + source files in DuckDB
    /// 6. Register package in indexed_packages
    pub fn index_directory(
        &self,
        dir: &Path,
        ecosystem: &str,
        package: &str,
        version: &str,
        embedder: &mut zen_embeddings::EmbeddingEngine,
        skip_tests: bool,
    ) -> Result<IndexResult, LakeError> {
        let mut symbols = Vec::new();
        let mut doc_chunks = Vec::new();
        let mut source_files = Vec::new();
        let mut file_count = 0i32;

        // Step 1+2: Walk and parse
        let walker = zen_search::walk::build_walker(
            dir,
            zen_search::walk::WalkMode::Raw,
            skip_tests,
            None,
            None,
        );

        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();
            let rel_path = path.strip_prefix(dir).unwrap_or(path);
            let rel_path_str = rel_path.to_string_lossy().to_string();

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue, // Skip binary/unreadable files
            };

            // Detect language
            let lang = zen_parser::detect_language(&rel_path_str);

            // Store source file (for znt grep)
            let lang_str = lang.map(|l| format!("{l:?}").to_lowercase());
            source_files.push(SourceFile {
                ecosystem: ecosystem.to_string(),
                package: package.to_string(),
                version: version.to_string(),
                file_path: rel_path_str.clone(),
                content: content.clone(),
                language: lang_str.clone(),
                size_bytes: content.len() as i32,
                line_count: content.lines().count() as i32,
            });

            if let Some(lang) = lang {
                // Parse source files
                let items = zen_parser::extract_api(&content, lang)
                    .unwrap_or_default();

                for item in &items {
                    symbols.push(parsed_item_to_row(
                        item, ecosystem, package, version, &rel_path_str,
                    ));
                }
                file_count += 1;
            }

            // Check if documentation file
            if is_doc_file(&rel_path_str) {
                let chunks = zen_parser::chunk_document(&content, &rel_path_str);
                for chunk in chunks {
                    doc_chunks.push((chunk, ecosystem.to_string(), package.to_string(), version.to_string()));
                }
            }
        }

        // Step 4: Generate embeddings (batch)
        let embed_texts: Vec<String> = symbols
            .iter()
            .map(|s| {
                format!(
                    "{} {} {}",
                    s.name,
                    s.signature.as_deref().unwrap_or(""),
                    s.doc_comment.as_deref().unwrap_or("")
                )
            })
            .collect();

        let symbol_embeddings = if !embed_texts.is_empty() {
            embedder.embed_batch(embed_texts)
                .map_err(|e| LakeError::Other(format!("Embedding failed: {e}")))?
        } else {
            Vec::new()
        };

        for (sym, emb) in symbols.iter_mut().zip(symbol_embeddings.into_iter()) {
            sym.embedding = emb;
        }

        let doc_embed_texts: Vec<String> = doc_chunks
            .iter()
            .map(|(c, _, _, _)| c.content.clone())
            .collect();

        let doc_embeddings = if !doc_embed_texts.is_empty() {
            embedder.embed_batch(doc_embed_texts)
                .map_err(|e| LakeError::Other(format!("Embedding failed: {e}")))?
        } else {
            Vec::new()
        };

        let mut doc_chunk_rows: Vec<DocChunkRow> = doc_chunks
            .into_iter()
            .zip(doc_embeddings.into_iter())
            .map(|((chunk, eco, pkg, ver), emb)| DocChunkRow {
                id: generate_chunk_id(&eco, &pkg, &ver, &chunk.source_file, chunk.chunk_index),
                ecosystem: eco,
                package: pkg,
                version: ver,
                chunk_index: chunk.chunk_index as i32,
                title: chunk.title,
                content: chunk.content,
                source_file: Some(chunk.source_file),
                format: Some(chunk.format),
                embedding: emb,
            })
            .collect();

        let symbol_count = symbols.len() as i32;
        let doc_chunk_count = doc_chunk_rows.len() as i32;
        let source_file_count = source_files.len() as i32;

        // Step 5: Store in local DuckDB cache (temporary — replaced by Lance + Turso in Phase 8/9)
        self.lake.store_symbols(&symbols)?;
        self.lake.store_doc_chunks(&doc_chunk_rows)?;
        // Source files go to separate DuckDB (permanent local store per 02 §11)
        self.source_store.store_source_files(&source_files)?;

        // Step 6: Register package
        self.lake.register_package(
            ecosystem, package, version,
            None, None, None, None,
            file_count, symbol_count, doc_chunk_count,
        )?;

        Ok(IndexResult {
            ecosystem: ecosystem.to_string(),
            package: package.to_string(),
            version: version.to_string(),
            file_count,
            symbol_count,
            doc_chunk_count,
            source_file_count,
        })
    }
}
```

**Helper functions** in pipeline.rs:

```rust
fn parsed_item_to_row(
    item: &zen_parser::ParsedItem,
    ecosystem: &str,
    package: &str,
    version: &str,
    file_path: &str,
) -> ApiSymbolRow {
    use sha2::{Sha256, Digest};
    let id_input = format!("{ecosystem}:{package}:{version}:{file_path}:{:?}:{}", item.kind, item.name);
    let hash = format!("{:x}", Sha256::digest(id_input.as_bytes()));
    let id = hash[..16].to_string(); // 16-char hex

    ApiSymbolRow {
        id,
        ecosystem: ecosystem.to_string(),
        package: package.to_string(),
        version: version.to_string(),
        file_path: file_path.to_string(),
        kind: format!("{:?}", item.kind).to_lowercase(),
        name: item.name.clone(),
        signature: Some(item.signature.clone()),
        source: item.source.clone(),
        doc_comment: if item.doc_comment.is_empty() { None } else { Some(item.doc_comment.clone()) },
        line_start: Some(item.start_line as i32),
        line_end: Some(item.end_line as i32),
        visibility: Some(format!("{:?}", item.visibility).to_lowercase()),
        is_async: item.metadata.is_async,
        is_unsafe: item.metadata.is_unsafe,
        is_error_type: item.metadata.is_error_type,
        returns_result: item.metadata.returns_result,
        return_type: item.metadata.return_type.clone(),
        generics: item.metadata.generics.clone(),
        attributes: if item.metadata.attributes.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&item.metadata.attributes).unwrap_or_default())
        },
        metadata: Some(serde_json::to_string(&item.metadata).unwrap_or_default()),
        embedding: Vec::new(), // Filled in batch after all symbols extracted
    }
}

fn is_doc_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md") || lower.ends_with(".rst") || lower.ends_with(".txt")
        || lower.starts_with("readme") || lower.starts_with("docs/")
        || lower.starts_with("doc/") || lower.contains("/docs/")
        || lower.contains("/doc/") || lower == "changelog.md"
        || lower == "contributing.md"
}

fn generate_chunk_id(ecosystem: &str, package: &str, version: &str, source_file: &str, index: u32) -> String {
    use sha2::{Sha256, Digest};
    let input = format!("{ecosystem}:{package}:{version}:{source_file}:{index}");
    let hash = format!("{:x}", Sha256::digest(input.as_bytes()));
    hash[..16].to_string()
}
```

**Cargo.toml update** for zen-cli — pipeline needs all crates:

```toml
[dependencies]
# ... existing zen-cli deps ...
zen-parser.workspace = true       # for extract_api(), ParsedItem, detect_language()
zen-embeddings.workspace = true   # for EmbeddingEngine
zen-lake.workspace = true         # for ZenLake, SourceFileStore
zen-search.workspace = true       # for walk::build_walker()
sha2.workspace = true             # for deterministic symbol/chunk IDs
```

> **No zen-lake Cargo.toml changes**: zen-lake does NOT gain zen-parser or zen-search dependencies. Pipeline orchestration lives in zen-cli.

### D3. Tests

**Integration tests** (require zen-parser + zen-embeddings + zen-lake):

- **Full pipeline**: Create a temp directory with sample Rust/Python files, run `index_directory()`, verify:
  - Symbols stored in DuckDB cache with correct names, kinds, signatures
  - Doc chunks stored with correct titles and content
  - Source files cached in **separate** DuckDB with correct content
  - Package registered in local `indexed_packages` cache
  - Embeddings are 384-dim for all symbols and chunks
- **Empty directory**: Index an empty directory → 0 symbols, 0 chunks, package still registered
- **Mixed languages**: Directory with Rust + Python + TypeScript files → all languages parsed
- **Binary files skipped**: Directory contains a `.png` — it's skipped without error
- **Test files skipped**: With `skip_tests=true`, `_test.rs` files are not indexed
- **Large batch**: 500+ symbols → Appender handles without error
- **Doc chunking integration**: README.md with 5 sections → 5 doc chunks with embeddings
- **Walker integration**: `build_walker()` with `WalkMode::Raw` walks all files, `LocalProject` respects .gitignore

---

## 9. Execution Order

```
Phase 3 Remaining Execution (from 2026-02-13 baseline):

 ┌──────────────────────────────────────────────────────────┐
 │ PR 1 (A), PR 2 (B), and PR 3 (C) can run in parallel   │
 │ — no runtime dependencies between them                   │
 └──────────────────────────────────────────────────────────┘

 1. [A1]   Create zen-parser/src/test_files.rs
 2. [A2]   Create zen-parser/src/doc_chunker.rs
 3. [A3]   Add extract_api() top-level orchestrator (new module or in lib.rs)
 4. [A4]   Update zen-parser/src/lib.rs (add test_files, doc_chunker, extract_api exports)
 5. [A5]   Write tests for new modules (test_files, doc_chunker, extract_api)
    ─── cargo test -p zen-parser passes (existing 1250 + new) ───

    ┌─────────────────────────────────────┐
    │ PR 2 (B) in parallel with PR 1 (A) │
    └─────────────────────────────────────┘

 6. [B1]   Create zen-embeddings/src/error.rs
 7. [B2]   Rewrite zen-embeddings/src/lib.rs
 8. [B3]   Update zen-embeddings/Cargo.toml (promote dirs to deps)
 9. [B4]   Write zen-embeddings tests
    ─── cargo test -p zen-embeddings passes ───

    ┌─────────────────────────────────────┐
    │ PR 3 (C) in parallel with PR 1 (A) │
    └─────────────────────────────────────┘

10. [C1]   Create zen-lake/src/error.rs
11. [C2]   Create zen-lake/src/schemas.rs
12. [C3]   Rewrite zen-lake/src/lib.rs
13. [C4]   Create zen-lake/src/store.rs
14. [C5]   Create zen-lake/src/source_files.rs
15. [C6]   Write zen-lake tests
    ─── cargo test -p zen-lake passes ───

    ┌─────────────────────────────────────────────────┐
    │ PR 4 (D) must wait for all 3 PRs above to land │
    └─────────────────────────────────────────────────┘

16. [D1]   Create zen-search/src/walk.rs
17. [D1b]  Update zen-search/src/lib.rs + Cargo.toml (add zen-parser dep)
18. [D2]   Create zen-cli/src/pipeline.rs (pipeline orchestration)
19. [D2b]  Update zen-cli/Cargo.toml (add zen-parser, sha2 deps)
20. [D3]   Write integration tests
    ─── cargo test --workspace passes (Phase 3) ───
```

Steps 1–5, 6–9, and 10–15 are independent and can be parallelized.
Steps 16–20 depend on all three PRs above.

---

## 10. Gotchas & Warnings

### 10.1 KindMatcher with Any Requires Homogeneous Types

**Spike 0.8 finding (c)**: `All::new()` requires homogeneous matcher types. When using `Any` with `KindMatcher`, all matchers must be the same variant. Use `ops::Op` for mixed types.

**Action**: All extractors use `Any::new(vec![KindMatcher, ...])` — this is homogeneous and works. Don't mix `KindMatcher` with `Pattern` in the same `Any`.

### 10.2 Multi-Metavar Returns None for get_match()

**Spike 0.8 finding (d)**: `$$$` multi-metavars return `None` for `get_match()`. Must use `get_multiple_matches()` instead.

**Action**: Any pattern using `$$$PARAMS` or `$$$BODY` must use `get_multiple_matches()`.

### 10.3 Position::column() Takes &Node Argument

**Spike 0.8 finding (e)**: `Position::column()` requires `&Node` argument unlike `line()`. `line()` is a direct method; `column()` is not.

**Action**: Use `node.start_pos().line()` freely. If column is needed (unlikely for our use), pass `&node`.

### 10.4 text()/kind() Return Cow<str>

**Spike 0.8 finding (f)**: `text()` and `kind()` return `Cow<str>`, not `&str` or `String`. Use `.as_ref()` or `.to_string()` as needed.

**Action**: All comparisons use `kind.as_ref() == "function_item"`.

### 10.5 fastembed embed() Takes &mut self

**Spike 0.6 gotcha**: `embed()` takes `&mut self`, not `&self`. Cannot share `EmbeddingEngine` across threads without a `Mutex` or `spawn_blocking`.

**Action**: Pipeline passes `&mut EmbeddingEngine`. CLI creates one engine and passes it to the pipeline.

### 10.6 DuckDB Appender Embedding Column

**Risk**: DuckDB Appender may not directly accept `Vec<f32>` for `FLOAT[]` columns. The spike used `execute_batch` for FLOAT array inserts.

**Mitigation**: If Appender doesn't work with `Vec<f32>`, fall back to parameterized INSERT statements. Test this in PR 3 C6 early.

### 10.7 Circular Dependency: zen-lake ↔ zen-search — RESOLVED

**Problem**: Pipeline needs zen-parser, zen-search, and zen-lake. zen-search already depends on zen-lake. Placing pipeline in zen-lake would create a cycle.

**Resolution (decided)**: `pipeline.rs` lives in **zen-cli** — the only consumer. zen-lake remains a pure storage/query crate with NO dependency on zen-parser or zen-search. The pipeline directly calls walker, parser, embedder, and store methods. No ambiguity.

### 10.8 sha2 Workspace Dependency

**Problem**: `pipeline.rs` (in zen-cli) uses `sha2` for deterministic symbol IDs.

**Action**: Add `sha2` to zen-cli's `[dependencies]` and to workspace Cargo.toml if not present.

### 10.9 tree-sitter StreamingIterator (Raw Fallback)

**Spike 0.8 finding (h)**: `tree-sitter` 0.26 `QueryMatches` uses `StreamingIterator`, not `Iterator`. Only relevant if we use raw tree-sitter queries as fallback. Our KindMatcher approach avoids this.

**Action**: None needed for Phase 3. If raw tree-sitter queries are ever added, use `streaming_iterator` crate.

### 10.10 Impl Header Extraction — Complex Generics (Spike 0.21)

**Spike 0.21 finding**: `field("trait")`/`field("type")` on `impl_item` may miss complex impl headers — generic types (`impl<T> Foo<T>`), scoped types (`impl crate::Foo`), and trait-on-generic (`impl Trait for Foo<T>`). Spike 0.21 needed extended tree-sitter `Query` patterns to capture these at scale (600+ files in Arrow monorepo, +580 matches vs baseline).

**Action**: Implement `field("trait")`/`field("type")` as primary. Add a **conservative fallback** for Rust impl items where trait_name/for_type extraction yields None: parse `node.text()` with a regex for `impl\s+(?:(\S+)\s+for\s+)?(\S+)` to capture trait and type names. This avoids the tree-sitter `Query` API complexity while recovering most missing cases. If the regex fallback proves insufficient, promote to tree-sitter `Query` patterns matching spike 0.21's `extended_impl_query()`.

### 10.11 Doc Comment Extraction — Line-Based Fallback (Spike 0.21)

**Spike 0.21 finding**: The line-based `leading_doc_comment(source, start_line)` approach proved more robust on large real-world repos than pure AST sibling walking. It handles `///` and `//!`, tolerates blank lines between doc and item, and doesn't depend on AST sibling structure.

**Action**: Phase 3's `extract_doc_comments_rust()` uses AST sibling walk as primary, with line-based scan as fallback when sibling walk yields empty. See updated helpers.rs code in §A4.

### 10.12 Scope: Reference Graph and Decision Traces (Spikes 0.21, 0.22)

**Not in Phase 3 scope**:
- `symbol_refs` + `ref_edges` DuckDB schema (spike 0.21) — arrives in Phase 4 (Search & Registry)
- Decision traces as first-class entities + rustworkx-core graph algorithms (spike 0.22) — arrives in Phase 2b (zen-db storage layer)
- Recursive context query (RLM-style) (spike 0.21) — arrives in Phase 4

**Action**: Ensure `rustworkx-core` stays behind `#[cfg(test)]` in zen-search during Phase 3. No production dependency coupling.

---

## 11. Milestone 3 Validation

### Command

```bash
cargo test -p zen-parser -p zen-embeddings -p zen-lake -p zen-search
```

### Acceptance Criteria

**Already passing (as of 2026-02-13)**:
- [x] `zen-parser` extracts rich API symbols from all 24 supported languages (20 builtin + 4 custom-lane)
- [x] All extracted `ParsedItem` structs have correct: kind, name, signature (no body), visibility, start/end lines
- [x] Rust extractor: async/unsafe detection, generics, lifetimes, doc comments, attributes, impl block methods, enum variants, struct fields, error types
- [x] Python extractor: classes, decorators, docstrings (Google/NumPy/Sphinx), dataclass/pydantic/protocol
- [x] TypeScript extractor: exports, interfaces, type aliases, JSDoc, ambient declarations, namespaces
- [x] Go extractor: exported detection, doc comments, methods, struct fields with owner metadata
- [x] Cross-language taxonomy conformance: Constructor, Field, Property, owner_name/owner_kind
- [x] 1250 zen-parser tests passing
- [x] `cargo build --workspace` succeeds

**Remaining gates**:
- [ ] `extract_api()` orchestrator dispatches to all 24 languages via file path
- [ ] Test file detection: `is_test_file()` and `is_test_dir()` correct for all patterns
- [ ] Doc chunker: section-based splitting with ~512 token max
- [ ] `zen-embeddings` generates 384-dim vectors, similar texts cluster
- [ ] `zen-lake` stores and retrieves symbols, doc chunks in DuckDB local cache (`.zenith/lake/cache.duckdb`)
- [ ] `SourceFileStore` stores and retrieves source files in separate DuckDB (`.zenith/source_files.duckdb`)
- [ ] `array_cosine_similarity()` works on stored embeddings (FLOAT[] → FLOAT[384] cast) — local cache only
- [ ] Walker: `build_walker()` with `WalkMode::Raw` and `WalkMode::LocalProject` both produce correct file lists
- [ ] Full pipeline: index a temp directory with mixed-language source → all tables populated correctly
- [ ] `cargo test -p zen-parser -p zen-embeddings -p zen-lake -p zen-search` all pass

### What This Unlocks

Phase 3 completion unblocks:
- **Phase 4** (Search & Registry): Vector/FTS/hybrid search over local DuckDB cache; grep over source_files; recursive query
- **Phase 5** (CLI): `znt install`, `znt search`, `znt grep`, `znt cache` commands (local mode)
- **Phase 8/9** (Cloud): lancedb writes to R2 + Turso catalog registration (introduces `ProductionApiSymbol`/`ProductionDocChunk` structs as `serde_arrow` sources per spike 0.19; replaces DuckDB cache tables with Lance datasets + Turso `dl_data_file`)

---

## 12. Validation Traceability Matrix

### Spike Evidence (from Phase 0)

| Area | Claim | Status | Spike/Test Evidence | Source |
|------|-------|--------|---------------------|--------|
| ast-grep core parsing | All 26 built-in languages parse | Validated | `spike_core_parsing_all_rich_languages` + 26 grammar test | `zen-parser/src/spike_ast_grep.rs` |
| KindMatcher extraction | `KindMatcher` + `Any` find all target nodes | Validated | `spike_kind_matcher_extracts_all_rust_items` | `zen-parser/src/spike_ast_grep.rs` |
| Pattern matching fragility | `fn $NAME() { $$$ }` misses generic/return-type fns | Validated | `spike_pattern_matching_fragile_for_rust` | `zen-parser/src/spike_ast_grep.rs` |
| Field access | `field("name")`, `field("parameters")`, `field("return_type")` work | Validated | `spike_node_field_access` | `zen-parser/src/spike_ast_grep.rs` |
| Doc comment extraction | `prev()` sibling walking collects `///` comments | Validated | `spike_doc_comment_extraction_via_prev` | `zen-parser/src/spike_ast_grep.rs` |
| impl discrimination | `field("trait")` distinguishes inherent vs trait impl | Validated | `spike_impl_discrimination_via_field_trait` | `zen-parser/src/spike_ast_grep.rs` |
| Async/unsafe modifiers | `function_modifiers` child node contains async/unsafe | Validated | `spike_async_unsafe_in_function_modifiers` | `zen-parser/src/spike_ast_grep.rs` |
| Position lines | `start_pos().line()` is zero-based | Validated | `spike_position_zero_based` | `zen-parser/src/spike_ast_grep.rs` |
| text()/kind() Cow | Returns `Cow<str>`, use `.as_ref()` | Validated | Spike 0.8 finding (f) | implementation plan |
| fastembed 384-dim | `AllMiniLML6V2` produces 384-dim vectors | Validated | `spike_fastembed_all_mini_lm` | `zen-embeddings/src/spike_fastembed.rs` |
| fastembed determinism | Same input → same embedding | Validated | `spike_fastembed_deterministic` | `zen-embeddings/src/spike_fastembed.rs` |
| fastembed &mut self | `embed()` requires mutable reference | Validated | Spike 0.6 gotcha | implementation plan |
| fastembed cache dir | `with_cache_dir()` overrides CWD-relative default | Validated | Spike 0.6 gotcha | implementation plan |
| DuckDB CRUD | Create table, insert, query rows | Validated | `spike_duckdb_crud` | `zen-lake/src/spike_duckdb.rs` |
| DuckDB Appender | Bulk insert 1000 rows via Appender | Validated | `spike_duckdb_appender_bulk` | `zen-lake/src/spike_duckdb.rs` |
| DuckDB FLOAT[] arrays | `array_cosine_similarity()` works with FLOAT[384] cast | Validated | `spike_duckdb_float_array_cosine` | `zen-lake/src/spike_duckdb.rs` |
| DuckDB JSON columns | JSON storage and operators work | Validated | `spike_duckdb_json_columns` | `zen-lake/src/spike_duckdb.rs` |
| DuckDB persistence | File-backed DB persists across connections | Validated | `spike_duckdb_file_persistence` | `zen-lake/src/spike_duckdb.rs` |
| DuckDB sync strategy | DuckDB is synchronous; use `spawn_blocking` | Validated | Spike 0.4 note | implementation plan |
| ignore crate | `.gitignore` aware walking + override globs | Validated | `spike_ignore_gitignore_aware` | `zen-search/src/spike_grep.rs` |
| ignore filter_entry | `filter_entry` is evaluated before file I/O | Validated | `spike_ignore_filter_entry_free` | `zen-search/src/spike_grep.rs` |
| source_files table | DuckDB CRUD + Appender for source storage | Validated | `spike_source_files_crud` | `zen-search/src/spike_grep.rs` |
| Source cached flag | `source_cached` boolean on indexed_packages | Validated | `spike_source_cached_flag` | `zen-search/src/spike_grep.rs` |
| Walker + test skip | `filter_entry` skips test files/dirs | Validated | `spike_ignore_test_file_skipping` | `zen-search/src/spike_grep.rs` |
| Signature normalization | Whitespace collapse for deterministic signatures | Validated | `extract_signature_from_node_text()` in 600+ files | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Doc comment line-based fallback | Line-based `leading_doc_comment()` robust on large repos | Validated | 14,929 symbols extracted from Arrow monorepo | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Extended impl extraction | Generic/scoped/trait impl patterns need extended queries | Validated | +580 matches vs baseline on Arrow monorepo | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Lance production write path | `serde_arrow` → `lancedb` (not DuckDB COPY) | Validated | `spike_serde_arrow_production_path` (50 rows round-trip) | `zen-lake/src/spike_native_lance.rs` (spike 0.19) |
| serde_arrow FixedSizeList override | `embedding` must be overridden to `FixedSizeList(384)` | Validated | Spike 0.19 test M1 | `zen-lake/src/spike_native_lance.rs` (spike 0.19) |
| Turso catalog visibility | `dl_data_file` with public/team/private scoping | Validated | 9/9 tests | `zen-db/src/spike_catalog_visibility.rs` (spike 0.20) |
| Lance on R2 — vector/FTS/hybrid search | `lance_vector_search`, `lance_fts`, `lance_hybrid_search` | Validated | 18/18 tests | `zen-lake/src/spike_r2_parquet.rs` (spike 0.18) |

### Production Evidence (from implemented zen-parser, as of 2026-02-13)

| Area | Claim | Status | Evidence | Source |
|------|-------|--------|----------|--------|
| Full language extraction (24 langs) | Dedicated extractor for every supported language | **Implemented** | 24 dispatcher modules, 24 processor directories | `zen-parser/src/extractors/dispatcher/*.rs` |
| Cross-language taxonomy | Constructor/Field/Property/owner normalization | **Implemented** | Conformance tests pass | `zen-parser/src/extractors/dispatcher/conformance.rs` |
| Types module tree | Split `types.rs` into module tree | **Implemented** | 18 files in `src/types/`, per-language `*MetadataExt` traits | `zen-parser/src/types/mod.rs` |
| Custom language parsers | Markdown, TOML, RST, Svelte via tree-sitter | **Implemented** | `MarkdownLang`, `TomlLang`, `RstLang`, `SvelteLang` | `zen-parser/src/parser.rs` |
| Test fixtures | Sample files for all languages | **Implemented** | 30 fixture files | `zen-parser/tests/fixtures/` |
| Test coverage | Comprehensive extractor tests | **Implemented** | 1250 tests passing | `cargo test -p zen-parser` |
| Two-tier fallback | ast-grep empty → regex produces items | **Design-only** | Will be in `extract_api()` orchestrator | Pending (PR 1, task A3) |
| Doc chunking | Split by heading, max ~512 tokens | **Design-only** | Will be in `doc_chunker.rs` | Pending (PR 1, task A2) |
| Full pipeline | clone → walk → parse → embed → store | **Design-only** | Will be in `zen-cli/src/pipeline.rs` | Pending (PR 4, task D2) |

---

## 13. Phase Boundary — What Phase 8/9 Replaces

This section makes the Phase 3 → Phase 8/9 migration boundary explicit. Phase 3 implements **local-only DuckDB cache** storage. Phase 8/9 replaces specific components with the production architecture (Lance on R2 + Turso catalog).

### Replacement Map

| Phase 3 Component | Location | Status | Phase 8/9 Replacement | Notes |
|---|---|---|---|---|
| `api_symbols` table | `.zenith/lake/cache.duckdb` | **TEMPORARY** | Lance dataset on R2 (`symbols.lance`) | Written by `lancedb` via `serde_arrow`. `ApiSymbolRow` struct reused as source type. |
| `doc_chunks` table | `.zenith/lake/cache.duckdb` | **TEMPORARY** | Lance dataset on R2 (`doc_chunks.lance`) | Same pattern as api_symbols. |
| `indexed_packages` table | `.zenith/lake/cache.duckdb` | **TEMPORARY** | Turso `dl_data_file` + `dl_snapshot` catalog tables | Gains visibility scoping (public/team/private), crowdsourced dedup, Clerk JWT auth. |
| `source_files` table | `.zenith/source_files.duckdb` | **PERMANENT** | No change — stays local | Large content, not shared, not vectorized. Separate DuckDB file per [02 §11](./02-data-architecture.md). |
| `FLOAT[]` embeddings | DuckDB column | **TEMPORARY** | Lance `FixedSizeList(384)` + IVF-PQ vector index | Brute-force `array_cosine_similarity()` replaced by `lance_vector_search()`. |
| `ZenLake.store_symbols()` | `zen-lake/src/store.rs` | **KEPT** (local cache) | Add `LanceLakeBackend` alongside | Local DuckDB cache may remain for offline mode; Lance is primary. |
| `IndexingPipeline` | `zen-cli/src/pipeline.rs` | **EXTENDED** | Add steps 7+8: Lance write + Turso registration | Pipeline gains `lancedb::create_table()` + `INSERT INTO dl_data_file`. |
| Dedup check | Local `is_package_indexed()` | **REPLACED** | Turso `SELECT 1 FROM dl_data_file WHERE ...` | Global catalog dedup with visibility scoping. |

### Schema Alignment

Phase 3 DuckDB column names are intentionally aligned with the production Lance schema fields:
- `ApiSymbolRow` fields map directly to Lance `api_symbols` dataset columns
- `DocChunkRow` fields map directly to Lance `doc_chunks` dataset columns
- The `embedding` field changes type: DuckDB `FLOAT[]` → Lance `FixedSizeList(Float32, 384)`

Phase 8/9 introduces a `ProductionApiSymbol` struct (matching spike 0.19) for `serde_arrow::to_record_batch()`. The mapping from `ApiSymbolRow` → `ProductionApiSymbol` is straightforward — shared field names, `bool` → `Option<bool>` wrapping, `created_at` addition.

### What Does NOT Change in Phase 8/9

- `zen-parser` (extraction logic, types, extractors) — untouched
- `zen-embeddings` (fastembed wrapper) — untouched
- `SourceFileStore` (separate DuckDB) — untouched
- `zen-search/walk.rs` (walker factory) — untouched
- `ParsedItem` → `ApiSymbolRow` mapping — untouched

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Data architecture (Lance + Turso): [02-data-architecture.md](./02-data-architecture.md)
- DuckDB data model (legacy, reference): [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- Crate designs (zen-parser §7, zen-embeddings §8, zen-lake §6): [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan (Phase 3 tasks): [07-implementation-plan.md](./07-implementation-plan.md)
- Zen grep design (source_files, walker): [13-zen-grep-design.md](./13-zen-grep-design.md)
- Phase 1 plan (foundation): [19-phase1-foundation-plan.md](./19-phase1-foundation-plan.md)
- Phase 2 plan (storage layer): [20-phase2-storage-layer-plan.md](./20-phase2-storage-layer-plan.md)
- R2 Lance export spike: [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md)
- Native lancedb writes spike: [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md)
- Catalog visibility spike: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md)
- Recursive query spike (RLM): [21-rlm-recursive-query-spike-plan.md](./21-rlm-recursive-query-spike-plan.md)
- Decision graph spike: [22-decision-graph-rustworkx-spike-plan.md](./22-decision-graph-rustworkx-spike-plan.md)
- Validated spike code:
  - `zen-parser/src/spike_ast_grep.rs` (19/19 — KindMatcher, patterns, field access, doc comments, impl blocks)
  - `zen-embeddings/src/spike_fastembed.rs` (spike 0.6 — 384-dim, determinism, batch, cache)
  - `zen-lake/src/spike_duckdb.rs` (spike 0.4 — CRUD, Appender, FLOAT[], JSON, persistence)
  - `zen-lake/src/spike_r2_parquet.rs` (spike 0.18 — Parquet + Lance on R2, vector/FTS/hybrid search)
  - `zen-lake/src/spike_native_lance.rs` (spike 0.19 — serde_arrow production path, ApiSymbol struct)
  - `zen-search/src/spike_grep.rs` (spike 0.14 — grep, ignore, source_files, walker, symbol correlation)
  - `zen-search/src/spike_recursive_query.rs` (spike 0.21 — signature normalization, doc comment fallback, extended impl extraction)

---

## 14. Mismatch Log — Plan vs. Implementation

This section documents where the original plan text (rev 3 and earlier) diverges from the actual implementation as of 2026-02-13. It serves as an audit trail for the delta update.

### 14.1 Language Coverage: "7 rich + 19 generic" → 24 dedicated

**Original plan**: 7 rich extractors (Rust, Python, TypeScript/JS/TSX, Go, Elixir) + 1 generic kind-based extractor for the remaining 19 built-in languages.

**Actual implementation**: Every language has a dedicated extractor with its own dispatcher, processors, helpers, and tests. There is no generic extractor. Coverage far exceeds the original target:
- 20 builtin `SupportLang` dispatchers: Rust, Python, TypeScript, TSX, JavaScript (separate from TS, not shared), Go, Elixir, C (5 processor files), C++ (5 processor files), C# (3 processor files), CSS, Haskell, HTML, Java (3 processor files), JSON, Lua (3 processor files), PHP (6 processor files), Ruby, Bash (5 processor files), YAML
- 4 custom-lane dispatchers: Markdown (tree-sitter-md), TOML (tree-sitter-toml-ng), RST (tree-sitter-rst), Svelte (tree-sitter-svelte-next)

### 14.2 Module Architecture: Flat extractors → Dispatcher + #[path] pattern

**Original plan**: `src/extractors/mod.rs` with direct `pub mod rust;`, `pub mod python;`, etc. `extract_api()` lives in `extractors/mod.rs`.

**Actual implementation**: Two-level architecture:
- `src/extractors/dispatcher/mod.rs` declares 24 language modules
- Each `dispatcher/<lang>.rs` uses `#[path = "../<lang>/processors/mod.rs"] mod processors;` to pull in the implementation from `src/extractors/<lang>/` directories
- `src/extractors/mod.rs` re-exports all dispatcher modules via `pub use dispatcher::<lang>;`
- The `<lang>/` directories are filesystem-only organizational units, not Rust modules declared in `mod.rs`

### 14.3 SymbolKind: 13 → 19 variants

**Original plan**: Function, Method, Struct, Enum, Trait, Interface, Class, TypeAlias, Const, Static, Macro, Module, Union.

**Actual implementation**: Adds Constructor, Field, Property, Event, Indexer, Component. This supports cross-language member taxonomy and Svelte component detection.

### 14.4 SymbolMetadata: Flat struct expanded

**Original plan**: ~30 fields covering Common, Rust, Python, TypeScript, Documentation, Error detection.

**Actual implementation**: ~50+ fields. Added sections for HTML-specific (`tag_name`, `element_id`, `class_names`, `html_attributes`, `is_custom_element`, `is_self_closing`), CSS-specific (`selector`, `media_query`, `at_rule_name`, `css_properties`, `is_custom_property`), TSX/React-specific (`is_component`, `is_hook`, `is_hoc`, `is_forward_ref`, `is_memo`, `is_lazy`, `is_class_component`, `is_error_boundary`, `component_directive`, `props_type`, `hooks_used`, `jsx_elements`), and owner metadata (`owner_name`, `owner_kind`, `is_static_member`).

### 14.5 TypeScript/JS/TSX: Shared → Separate

**Original plan**: Single `typescript.rs` extractor shared across TypeScript, JavaScript, and TSX via language parameter.

**Actual implementation**: Three separate dispatchers:
- `dispatcher/typescript.rs` — takes `(root, lang: SupportLang)` for TypeScript
- `dispatcher/javascript.rs` — takes `(root)` for JavaScript
- `dispatcher/tsx.rs` — takes `(root, lang: SupportLang)` for TSX with React-specific detection

### 14.6 Dispatcher Signature Inconsistency

The plan assumed a uniform `extract(root)` or `extract(source, language)` signature. The actual implementation has three families:
- `extract(root)` — 16 dispatchers
- `extract(root, source: &str)` — 4 dispatchers (bash, c, cpp, rust — need raw source for doc comment fallback)
- `extract(root, lang: SupportLang)` — 2 dispatchers (typescript, tsx — need lang for shared processor logic)

This means the `extract_api()` orchestrator must handle all three signatures, which is why it takes `(source: &str, file_path: &str)` and internally constructs the right arguments.

### 14.7 detect_language: Plan included unsupported languages

**Original plan** (`§A3`): `detect_language()` mapped Hcl, Kotlin, Nix, Scala, Solidity, Swift extensions.

**Actual implementation**: `detect_language()` maps only languages with corresponding dispatchers. Hcl, Kotlin, Nix, Scala, Solidity, Swift are **not** mapped (no dispatchers exist for them).

### 14.8 Code examples in plan sections A4–A14 are historical reference

The inline Rust code for `helpers.rs`, `rust.rs`, `python.rs`, `typescript.rs`, `go.rs`, `elixir.rs`, `generic.rs`, `mod.rs`, `test_files.rs`, `doc_chunker.rs`, `lib.rs` shown in the original plan (§A4–A14) are **historical design sketches**. The actual implementations in `src/extractors/` diverge significantly — they are richer, handle more edge cases, and follow the dispatcher+processors+helpers architecture instead of flat modules. These code blocks are preserved in this document as design archaeology but should NOT be used as implementation templates.

# Phase 3: Parsing & Indexing Pipeline — Implementation Plan

**Version**: 2026-02-11 (rev 3 — spike 0.18–0.22 alignment review)
**Status**: Ready to Execute
**Depends on**: Phase 1 (all tasks DONE — 127 tests), Phase 0 (spikes 0.4, 0.5, 0.6, 0.8, 0.14, 0.18, 0.19, 0.20, 0.21)
**Produces**: Milestone 3 — `cargo test -p zen-parser -p zen-embeddings -p zen-lake` passes, full clone→parse→embed→store pipeline works end-to-end

> **⚠️ Storage Scope**: Phase 3 implements a **local-only DuckDB cache backend** (`LocalLakeBackend`) for offline search. This is explicitly a **temporary stepping stone** — not the production storage architecture. Production persistence (Lance datasets on R2 + Turso catalog registration with visibility scoping) is Phase 8/9. See [Phase Boundary §13](#13-phase-boundary--what-phase-89-replaces) for the exact replacement map.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state)
3. [Key Decisions](#3-key-decisions)
4. [Architecture: Three-Crate Split](#4-architecture-three-crate-split)
5. [PR 1 — Stream A: zen-parser](#5-pr-1--stream-a-zen-parser)
6. [PR 2 — Stream B: zen-embeddings](#6-pr-2--stream-b-zen-embeddings)
7. [PR 3 — Stream C: zen-lake Storage](#7-pr-3--stream-c-zen-lake-storage)
8. [PR 4 — Stream D: Indexing Pipeline + Walker](#8-pr-4--stream-d-indexing-pipeline--walker)
9. [Execution Order](#9-execution-order)
10. [Gotchas & Warnings](#10-gotchas--warnings)
11. [Milestone 3 Validation](#11-milestone-3-validation)
12. [Validation Traceability Matrix](#12-validation-traceability-matrix)
13. [Phase Boundary — What Phase 8/9 Replaces](#13-phase-boundary--what-phase-89-replaces)

---

## 1. Overview

**Goal**: ast-grep-based extraction across all 26 built-in languages (rich extractors for 7, generic for 19), fastembed integration, **local DuckDB cache storage** (temporary — production storage is Lance on R2 + Turso catalog in Phase 8/9), source file caching for `znt grep`, and the local indexing pipeline (clone → walk → parse → extract → embed → store to DuckDB cache).

**Crates touched**:
- `zen-parser` (heavy — all new production code, ~3000–4000 LOC)
- `zen-embeddings` (light — ~200 LOC production, thin wrapper)
- `zen-lake` (medium — ~1000–1500 LOC production, DuckDB local cache tables + appender)
- `zen-search` (light — ~200 LOC, walker factory only)
- `zen-cli` (light — pipeline orchestration module, ~300 LOC)

**Dependency changes**:
- `zen-embeddings`: promote `dirs` from `[dev-dependencies]` to `[dependencies]` (for `~/.zenith/cache/fastembed/` path)
- `zen-search`: add `zen-parser.workspace = true` (for `is_test_file/is_test_dir` in walker)
- Workspace: add `sha2` if not present (for deterministic symbol IDs in pipeline)

**Estimated deliverables**: ~30 new files, ~5000–6000 LOC production code, ~3000 LOC tests

**PR strategy**: 4 PRs by stream. Each PR compiles and tests pass before merging.

| PR | Stream | Contents |
|----|--------|----------|
| PR 1 | A: zen-parser | Types, extractors (7 rich + 1 generic), test detection, orchestrator |
| PR 2 | B: zen-embeddings | EmbeddingEngine, error type |
| PR 3 | C: zen-lake | DuckDB local cache schema, ZenLake struct, store_symbols/store_doc_chunks |
| PR 3b | C2: source_files | Separate DuckDB for source file caching (`.zenith/source_files.duckdb`) |
| PR 4 | D: Pipeline + Walker | Walk factory (zen-search), indexing pipeline (**zen-cli**), doc chunker |

---

## 2. Current State

| Component | Status | What Exists |
|-----------|--------|-------------|
| **zen-parser** | Stub | `spike_ast_grep.rs` (19 tests, behind `#[cfg(test)]`). No production modules. Cargo.toml has all deps. |
| **zen-embeddings** | Stub | `spike_fastembed.rs` (behind `#[cfg(test)]`). No production modules. Cargo.toml has all deps. |
| **zen-lake** | Stub | 4 spike modules (duckdb, duckdb_vss, r2_parquet, native_lance — behind `#[cfg(test)]`). No production modules. Cargo.toml has all deps. Note: `lancedb`, `arrow-*`, `serde_arrow` are in `[dev-dependencies]` only — Phase 3 doesn't need them (lancedb writes are Phase 8/9). |
| **zen-search** | Stub | 3 spike modules (grep, recursive_query, graph_algorithms — behind `#[cfg(test)]`). No production modules. Cargo.toml has `grep` + `ignore` in deps. |
| **zen-core** | Phase 1 DONE | 15 entity structs, 14 enums, `ParsedItem`-compatible types live in design docs but NOT in zen-core. `SymbolKind`, `Visibility` not yet in zen-core (defined in zen-parser). |
| **Fixtures** | NOT STARTED | No `tests/fixtures/` directory with sample source files. |

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

```
zen-parser/src/
├── lib.rs               # Public API: parse_file(), extract_api(), detect_language()
├── error.rs             # ParserError enum
├── types.rs             # ParsedItem, SymbolMetadata, DocSections, SymbolKind, Visibility
├── parser.rs            # ast-grep wrapper, SupportLang mapping, language detection
├── test_files.rs        # is_test_file(), is_test_dir()
├── doc_chunker.rs       # split_into_chunks() for README/docs
└── extractors/
    ├── mod.rs           # Extraction orchestrator (two-tier fallback)
    ├── generic.rs       # Generic kind-based extractor (all 26 languages)
    ├── rust.rs          # Rust rich extractor
    ├── python.rs        # Python rich extractor
    ├── typescript.rs    # TypeScript/JavaScript/TSX rich extractor
    ├── go.rs            # Go rich extractor
    ├── elixir.rs        # Elixir rich extractor
    └── helpers.rs       # Shared extraction helpers (signature, doc comments, visibility)

zen-embeddings/src/
├── lib.rs               # EmbeddingEngine struct
└── error.rs             # EmbeddingError enum

zen-lake/src/
├── lib.rs               # ZenLake struct, open_local()
├── error.rs             # LakeError enum
├── schemas.rs           # DuckDB table DDL constants (LOCAL CACHE ONLY), ApiSymbol/DocChunk structs
├── store.rs             # store_symbols(), store_doc_chunks(), register_package()
└── source_files.rs      # SourceFileStore: separate DuckDB (.zenith/source_files.duckdb)

zen-search/src/
├── lib.rs               # (existing) + re-export walk module
└── walk.rs              # WalkMode, build_walker() — ignore crate integration

zen-cli/src/
└── pipeline.rs          # IndexingPipeline: walk → parse → embed → store (orchestration only)
```

---

## 5. PR 1 — Stream A: zen-parser

The largest and most complex PR. All extraction logic lives here.

### A1. `src/error.rs` — ParserError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Parse failed for {language}: {message}")]
    ParseFailed {
        language: String,
        message: String,
    },

    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### A2. `src/types.rs` — ParsedItem, SymbolMetadata, DocSections

Core data structures for extracted symbols. Ported from `05-crate-designs.md` §7.

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedItem {
    pub kind: SymbolKind,
    pub name: String,
    pub signature: String,
    pub source: Option<String>,     // Full source up to 50 lines
    pub doc_comment: String,
    pub start_line: u32,
    pub end_line: u32,
    pub visibility: Visibility,
    pub metadata: SymbolMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    TypeAlias,
    Const,
    Static,
    Macro,
    Module,
    Union,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    PublicCrate,
    Private,
    Export,
    Protected,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolMetadata {
    // Common
    pub is_async: bool,
    pub is_unsafe: bool,
    pub return_type: Option<String>,
    pub generics: Option<String>,
    pub attributes: Vec<String>,
    pub parameters: Vec<String>,

    // Rust-specific
    pub lifetimes: Vec<String>,
    pub where_clause: Option<String>,
    pub trait_name: Option<String>,
    pub for_type: Option<String>,
    pub associated_types: Vec<String>,
    pub abi: Option<String>,
    pub is_pyo3: bool,

    // Enum/Struct members
    pub variants: Vec<String>,
    pub fields: Vec<String>,
    pub methods: Vec<String>,

    // Python-specific
    pub is_generator: bool,
    pub is_property: bool,
    pub is_classmethod: bool,
    pub is_staticmethod: bool,
    pub is_dataclass: bool,
    pub is_pydantic: bool,
    pub is_protocol: bool,
    pub is_enum: bool,
    pub base_classes: Vec<String>,
    pub decorators: Vec<String>,

    // TypeScript-specific
    pub is_exported: bool,
    pub is_default_export: bool,
    pub type_parameters: Option<String>,
    pub implements: Vec<String>,

    // Documentation
    pub doc_sections: DocSections,

    // Error detection
    pub is_error_type: bool,
    pub returns_result: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocSections {
    pub errors: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
    pub examples: Option<String>,
    pub args: HashMap<String, String>,
    pub returns: Option<String>,
    pub raises: HashMap<String, String>,
    pub yields: Option<String>,
    pub notes: Option<String>,
}
```

**Source**: `05-crate-designs.md` §7, lines 1028–1119.

### A3. `src/parser.rs` — ast-grep Wrapper + Language Detection

Maps file extensions to `ast_grep_language::SupportLang`, wraps `SupportLang::ast_grep()`.

```rust
use ast_grep_language::SupportLang;

pub fn detect_language(file_path: &str) -> Option<SupportLang> {
    let ext = file_path.rsplit('.').next()?;
    match ext {
        "rs" => Some(SupportLang::Rust),
        "py" => Some(SupportLang::Python),
        "ts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "go" => Some(SupportLang::Go),
        "ex" | "exs" => Some(SupportLang::Elixir),
        "c" | "h" => Some(SupportLang::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(SupportLang::Cpp),
        "cs" => Some(SupportLang::CSharp),
        "css" => Some(SupportLang::Css),
        "hs" => Some(SupportLang::Haskell),
        "tf" | "hcl" => Some(SupportLang::Hcl),
        "html" | "htm" => Some(SupportLang::Html),
        "java" => Some(SupportLang::Java),
        "json" => Some(SupportLang::Json),
        "kt" | "kts" => Some(SupportLang::Kotlin),
        "lua" => Some(SupportLang::Lua),
        "nix" => Some(SupportLang::Nix),
        "php" => Some(SupportLang::Php),
        "rb" => Some(SupportLang::Ruby),
        "scala" | "sc" => Some(SupportLang::Scala),
        "sol" => Some(SupportLang::Solidity),
        "swift" => Some(SupportLang::Swift),
        "sh" | "bash" | "zsh" => Some(SupportLang::Bash),
        "yaml" | "yml" => Some(SupportLang::Yaml),
        _ => None,
    }
}

pub fn parse_source(source: &str, lang: SupportLang) -> ast_grep_core::AstGrep<ast_grep_language::SupportLang> {
    lang.ast_grep(source)
}
```

### A4. `src/extractors/helpers.rs` — Shared Extraction Helpers

Shared functions used by all rich extractors:

```rust
use ast_grep_core::Node;

/// Extract signature: everything before first `{` or `;`, whitespace-normalized.
///
/// **Spike 0.21 finding**: Normalize whitespace (collapse newlines/runs to single space)
/// for deterministic signatures regardless of source formatting. This matters for
/// embedding stability and deterministic symbol IDs.
pub fn extract_signature<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let text = node.text().to_string();
    let brace = text.find('{');
    let semi = text.find(';');
    let end = match (brace, semi) {
        (Some(b), Some(s)) => b.min(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => text.len(),
    };
    let sig = text[..end].trim();
    sig.replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract full source up to `max_lines` lines.
pub fn extract_source<D: ast_grep_core::Doc>(node: &Node<D>, max_lines: usize) -> Option<String> {
    let text = node.text().to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        Some(text)
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        Some(format!("{truncated}\n    // ... ({} more lines)", lines.len() - max_lines))
    }
}

/// Extract doc comments by walking backward through siblings.
///
/// **Primary**: Walks `prev()` siblings collecting `///` and `//!` comments (Rust),
/// skipping attribute_item nodes. Stops at non-comment, non-attribute siblings.
/// **Spike 0.8 validated**: `prev()` sibling walking for doc comments works.
///
/// **Fallback** (spike 0.21 finding): If AST sibling walk yields empty, try
/// line-based scan above `start_pos().line()` for `///`/`//!` lines. This is
/// more robust on large repos where AST layout quirks may cause sibling walk
/// to miss comments (e.g., blank lines between doc and item).
pub fn extract_doc_comments_rust<D: ast_grep_core::Doc>(node: &Node<D>, source: &str) -> String {
    // Primary: AST sibling walk (fast, structured)
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind.as_ref() == "line_comment" {
            let text = sibling.text().to_string();
            if text.starts_with("///") || text.starts_with("//!") {
                comments.push(
                    text.trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim()
                        .to_string(),
                );
            } else {
                break;
            }
        } else if kind.as_ref() == "attribute_item" {
            // Skip attributes, keep looking for docs
        } else {
            break;
        }
        current = sibling.prev();
    }
    if !comments.is_empty() {
        comments.reverse();
        return comments.join("\n");
    }

    // Fallback: line-based scan (spike 0.21 approach)
    let lines: Vec<&str> = source.lines().collect();
    let start_line = node.start_pos().line(); // zero-based
    if start_line == 0 || lines.is_empty() {
        return String::new();
    }
    let mut idx = start_line.saturating_sub(1);
    // Skip blank lines between doc and item
    while idx > 0 && lines.get(idx).map_or(false, |l| l.trim().is_empty()) {
        idx -= 1;
    }
    let mut docs = Vec::new();
    loop {
        let line = lines.get(idx).map(|l| l.trim_start()).unwrap_or("");
        if line.starts_with("///") {
            docs.push(line.trim_start_matches("///").trim().to_string());
        } else if line.starts_with("//!") {
            docs.push(line.trim_start_matches("//!").trim().to_string());
        } else {
            break;
        }
        if idx == 0 { break; }
        idx -= 1;
    }
    docs.reverse();
    docs.join("\n")
}

/// Extract attributes from preceding siblings.
///
/// Walks backward collecting `#[attr]` items until a non-attribute,
/// non-comment sibling is found.
pub fn extract_attributes<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind.as_ref() == "attribute_item" {
            let text = sibling.text().to_string();
            let inner = text
                .trim_start_matches("#[")
                .trim_end_matches(']')
                .to_string();
            attrs.push(inner);
        } else if kind.as_ref() == "line_comment" {
            // Skip comments between attributes
        } else {
            break;
        }
        current = sibling.prev();
    }
    attrs.reverse();
    attrs
}

/// Detect visibility from node text or children.
///
/// Rust: checks for `pub`, `pub(crate)`, `pub(super)`.
/// TypeScript/JS: checks for `export` keyword.
/// Python: checks for `_` prefix convention.
pub fn extract_visibility_rust<D: ast_grep_core::Doc>(node: &Node<D>) -> crate::types::Visibility {
    if let Some(vis_node) = node.field("visibility") {
        let text = vis_node.text().to_string();
        if text.contains("pub(crate)") {
            crate::types::Visibility::PublicCrate
        } else if text.starts_with("pub") {
            crate::types::Visibility::Public
        } else {
            crate::types::Visibility::Private
        }
    } else {
        crate::types::Visibility::Private
    }
}

/// Extract return type from function node.
pub fn extract_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("return_type")
        .map(|rt| rt.text().to_string().trim_start_matches("->").trim().to_string())
}

/// Extract generic parameters from node.
pub fn extract_generics<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("type_parameters")
        .map(|tp| tp.text().to_string())
}

/// Extract where clause from function/impl/struct node.
pub fn extract_where_clause<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    for child in node.children() {
        if child.kind().as_ref() == "where_clause" {
            return Some(child.text().to_string());
        }
    }
    None
}

/// Extract lifetimes from generics text.
pub fn extract_lifetimes(generics: &Option<String>) -> Vec<String> {
    match generics {
        Some(g) => {
            let mut lifetimes = Vec::new();
            for part in g.split(',') {
                let part = part.trim();
                if part.starts_with('\'') {
                    let lt = part.split(|c: char| !c.is_alphanumeric() && c != '\'')
                        .next()
                        .unwrap_or(part);
                    lifetimes.push(lt.to_string());
                }
            }
            lifetimes
        }
        None => Vec::new(),
    }
}

/// Check if a function returns Result.
pub fn returns_result(return_type: &Option<String>) -> bool {
    return_type
        .as_deref()
        .is_some_and(|rt| rt.contains("Result"))
}

/// Check if a type name indicates an error type.
pub fn is_error_type_by_name(name: &str) -> bool {
    name.ends_with("Error") || name.ends_with("Err")
}

/// Check if an item has PyO3 attributes.
pub fn is_pyo3(attrs: &[String]) -> bool {
    attrs.iter().any(|a| a.starts_with("pyfunction") || a.starts_with("pyclass") || a.starts_with("pymethods"))
}

/// Detect async/unsafe via function_modifiers child node.
///
/// **Spike 0.8 finding (b)**: `async`/`unsafe` appear as children
/// of `function_modifiers` node, not as direct children of the function.
pub fn detect_modifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> (bool, bool) {
    let mut is_async = false;
    let mut is_unsafe = false;
    for child in node.children() {
        let kind = child.kind();
        let k = kind.as_ref();
        if k == "function_modifiers" {
            let text = child.text().to_string();
            is_async = text.contains("async");
            is_unsafe = text.contains("unsafe");
            break;
        }
        // Some languages put async/unsafe as direct children
        if k == "async" {
            is_async = true;
        }
        if k == "unsafe" {
            is_unsafe = true;
        }
    }
    is_async = is_async || node.text().starts_with("async ");
    is_unsafe = is_unsafe || node.text().starts_with("unsafe ");
    (is_async, is_unsafe)
}
```

### A5. `src/extractors/rust.rs` — Rust Rich Extractor (task 3.3)

Port from `05-crate-designs.md` §7, lines 1166–1303. Uses KindMatcher-first strategy (decision 3.1).

**Extraction targets** (13 node kinds):
- `function_item` → Function
- `struct_item` → Struct (fields, derives)
- `enum_item` → Enum (variants)
- `trait_item` → Trait (methods, associated types)
- `impl_item` → Methods (discriminate inherent vs trait impl via `field("trait")`)
- `type_item` → TypeAlias
- `mod_item` → Module
- `const_item` → Const
- `static_item` → Static
- `macro_definition` → Macro
- `union_item` → Union

**Key patterns validated in spike 0.8**:
- `field("name")` for symbol names
- `field("parameters")` for function params
- `field("return_type")` for return types
- `field("body")` for bodies (excluded from signatures)
- `field("trait")` on `impl_item` to discriminate inherent vs trait impl
- `children()` for enum variants, struct fields, trait methods
- `prev()` sibling walking for doc comments and attributes
- `start_pos().line()` is zero-based — add 1 for human-readable lines

**Special handling**:
- `impl_item` with `field("trait")` → trait_name/for_type extracted; methods get `SymbolKind::Method` with `trait_name` in metadata
- `impl_item` without `field("trait")` → inherent impl; methods get `SymbolKind::Method` with `for_type` only
- Enum variants extracted via `children()` filtering for `enum_variant`
- Struct fields extracted via `children()` filtering for `field_declaration`
- `#[derive()]` attributes parsed for error type detection
- Doc section parsing: `# Errors`, `# Panics`, `# Safety`, `# Examples`

```rust
use ast_grep_core::{matcher::KindMatcher, ops::Any};
use ast_grep_language::SupportLang;

const RUST_ITEM_KINDS: &[&str] = &[
    "function_item", "struct_item", "enum_item", "trait_item",
    "impl_item", "type_item", "mod_item", "const_item",
    "static_item", "macro_definition", "union_item",
];

pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<crate::types::ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher<SupportLang>> = RUST_ITEM_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Rust))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        if let Some(item) = process_rust_node(&node) {
            items.push(item);
        }
    }
    Ok(items)
}
```

**impl_item processing** — extracts each method as a separate `ParsedItem` with `SymbolKind::Method`:

```rust
fn process_impl_item<D: ast_grep_core::Doc>(
    node: &ast_grep_core::Node<D>,
) -> Vec<crate::types::ParsedItem> {
    let trait_name = node.field("trait").map(|n| n.text().to_string());
    let for_type = node.field("type").map(|n| n.text().to_string());

    let mut methods = Vec::new();
    let body = match node.field("body") {
        Some(b) => b,
        None => return methods,
    };

    for child in body.children() {
        if child.kind().as_ref() == "function_item" {
            let name = child.field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            let (is_async, is_unsafe) = helpers::detect_modifiers(&child);
            let attrs = helpers::extract_attributes(&child);
            let generics = helpers::extract_generics(&child);
            let return_type = helpers::extract_return_type(&child);

            methods.push(crate::types::ParsedItem {
                kind: crate::types::SymbolKind::Method,
                name,
                signature: helpers::extract_signature(&child),
                source: helpers::extract_source(&child, 50),
                doc_comment: helpers::extract_doc_comments_rust(&child),
                start_line: child.start_pos().line() as u32 + 1,
                end_line: child.end_pos().line() as u32 + 1,
                visibility: helpers::extract_visibility_rust(&child),
                metadata: crate::types::SymbolMetadata {
                    is_async,
                    is_unsafe,
                    return_type: return_type.clone(),
                    generics: generics.clone(),
                    attributes: attrs.clone(),
                    lifetimes: helpers::extract_lifetimes(&generics),
                    where_clause: helpers::extract_where_clause(&child),
                    trait_name: trait_name.clone(),
                    for_type: for_type.clone(),
                    is_pyo3: helpers::is_pyo3(&attrs),
                    is_error_type: false,
                    returns_result: helpers::returns_result(&return_type),
                    ..Default::default()
                },
            });
        }
    }
    methods
}
```

### A6. `src/extractors/python.rs` — Python Rich Extractor (task 3.4)

Port from klaw `python-treesitter.ts`. Key patterns:

- `class_definition` → Class
  - Detect `dataclass`, `pydantic.BaseModel`, `Protocol`, `Enum` via decorators and base classes
  - Extract `@classmethod`, `@staticmethod`, `@property` methods
  - Google/NumPy/Sphinx docstring parsing
- `function_definition` → Function or Method (depending on parent context)
  - `@decorator` extraction
  - `*args`, `**kwargs` parameter handling
  - Generator detection (`yield` in body)
  - Type annotation extraction from parameters and return type

**Node kinds**: `function_definition`, `class_definition`, `decorated_definition`, `assignment` (module-level constants)

**Docstring parsing**: Look for `expression_statement` as first child of function/class body containing a `string` node. Parse Google-style (`Args:`, `Returns:`, `Raises:`), NumPy-style (`Parameters\n----------`), and Sphinx-style (`:param`, `:returns:`, `:raises:`).

### A7. `src/extractors/typescript.rs` — TypeScript/JS/TSX Rich Extractor (task 3.5)

Shared extractor for TypeScript, JavaScript, and TSX. Key patterns:

- `function_declaration` → Function (check for `export` keyword)
- `class_declaration` → Class
- `interface_declaration` → Interface (TypeScript only)
- `type_alias_declaration` → TypeAlias (TypeScript only)
- `method_definition` → Method (within class body)
- `lexical_declaration` with arrow function → Function (exported arrow functions)
- `export_statement` wrapping → sets `is_exported = true`

**JSDoc extraction**: Look for `comment` nodes preceding the declaration. Parse `@param`, `@returns`, `@throws`, `@example` tags.

### A8. `src/extractors/go.rs` — Go Rich Extractor (task 3.6)

Key patterns:

- `function_declaration` → Function (exported if name starts with uppercase)
- `method_declaration` → Method (has receiver parameter)
- `type_declaration` → Struct/Interface/TypeAlias
- `const_spec` → Const
- `var_spec` → Static (module-level vars)

**Go doc comments**: Preceding `// Comment` lines. Go convention: doc comment must start with the function name.

**Exported detection**: `name[0].is_uppercase()` — Go's visibility convention.

### A9. `src/extractors/elixir.rs` — Elixir Rich Extractor (task 3.7)

Key patterns:

- `call` with function name `defmodule` → Module
- `call` with function name `def` → Function (public)
- `call` with function name `defp` → Function (private)
- `call` with function name `defmacro` → Macro

**Elixir doc comments**: `@doc` and `@moduledoc` attributes preceding the definition.

### A10. `src/extractors/generic.rs` — Generic Kind-Based Extractor (task 3.8)

Works for all 26 built-in languages. Uses a language-specific list of "interesting" node kinds and extracts name + signature.

```rust
pub fn extract<D: ast_grep_core::Doc>(
    root: &ast_grep_core::AstGrep<D>,
    lang: SupportLang,
) -> Result<Vec<crate::types::ParsedItem>, crate::error::ParserError> {
    let kinds = interesting_kinds(lang);
    let matchers: Vec<KindMatcher<SupportLang>> = kinds
        .iter()
        .map(|k| KindMatcher::new(k, lang))
        .collect();

    if matchers.is_empty() {
        return Ok(Vec::new());
    }

    let matcher = Any::new(matchers);
    let mut items = Vec::new();

    for node in root.root().find_all(&matcher) {
        let name = node.field("name")
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| {
                // Fallback: first identifier child
                node.children()
                    .find(|c| c.kind().as_ref() == "identifier" || c.kind().as_ref() == "type_identifier")
                    .map(|c| c.text().to_string())
                    .unwrap_or_default()
            });

        if name.is_empty() {
            continue;
        }

        items.push(crate::types::ParsedItem {
            kind: map_kind(node.kind().as_ref()),
            name,
            signature: helpers::extract_signature(&node),
            source: helpers::extract_source(&node, 50),
            doc_comment: String::new(), // Generic: no doc comment extraction
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: crate::types::Visibility::Public, // Generic: assume public
            metadata: crate::types::SymbolMetadata::default(),
        });
    }
    Ok(items)
}
```

**`interesting_kinds()` mapping**: Returns language-specific node kinds that represent extractable symbols. Key examples:
- C: `function_definition`, `struct_specifier`, `type_definition`, `enum_specifier`
- C++: above + `class_specifier`, `namespace_definition`, `template_declaration`
- Java: `class_declaration`, `method_declaration`, `interface_declaration`, `enum_declaration`
- Ruby: `class`, `module`, `method`, `singleton_method`
- Swift: `class_declaration`, `struct_declaration`, `protocol_declaration`, `function_declaration`

### A11. `src/extractors/mod.rs` — Extraction Orchestrator (task 3.10)

Two-tier fallback: ast-grep → regex.

```rust
use ast_grep_language::SupportLang;

pub mod generic;
pub mod rust;
pub mod python;
pub mod typescript;
pub mod go;
pub mod elixir;
pub(crate) mod helpers;

pub fn extract_api(
    source: &str,
    language: SupportLang,
) -> Result<Vec<crate::types::ParsedItem>, crate::error::ParserError> {
    // Tier 1: ast-grep KindMatcher + Node traversal
    match extract_with_ast_grep(source, language) {
        Ok(items) if !items.is_empty() => return Ok(items),
        Ok(_) => tracing::debug!("ast-grep returned no items for {language:?}"),
        Err(e) => tracing::warn!("ast-grep extraction failed for {language:?}: {e}"),
    }

    // Tier 2: Regex fallback
    match extract_with_regex(source, language) {
        Ok(items) => Ok(items),
        Err(e) => {
            tracing::warn!("regex extraction failed for {language:?}: {e}");
            Ok(vec![])
        }
    }
}

fn extract_with_ast_grep(
    source: &str,
    language: SupportLang,
) -> Result<Vec<crate::types::ParsedItem>, crate::error::ParserError> {
    let root = language.ast_grep(source);
    match language {
        SupportLang::Rust => rust::extract(&root),
        SupportLang::Python => python::extract(&root),
        SupportLang::TypeScript | SupportLang::Tsx | SupportLang::JavaScript => {
            typescript::extract(&root)
        }
        SupportLang::Go => go::extract(&root),
        SupportLang::Elixir => elixir::extract(&root),
        _ => generic::extract(&root, language),
    }
}

fn extract_with_regex(
    source: &str,
    _language: SupportLang,
) -> Result<Vec<crate::types::ParsedItem>, crate::error::ParserError> {
    // Basic regex patterns for common definitions
    // fn/def/function/class patterns across languages
    let mut items = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(item) = try_regex_extract(trimmed, line_num as u32 + 1) {
            items.push(item);
        }
    }
    Ok(items)
}
```

### A12. `src/test_files.rs` — Test File/Dir Detection (task 3.9)

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

### A13. `src/doc_chunker.rs` — Documentation Chunker (task 3.15)

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

### A14. `src/lib.rs` — Public API

```rust
pub mod error;
pub mod types;
pub mod parser;
pub mod extractors;
pub mod test_files;
pub mod doc_chunker;

pub use error::ParserError;
pub use types::{ParsedItem, SymbolKind, SymbolMetadata, DocSections, Visibility};
pub use parser::{detect_language, parse_source};
pub use extractors::extract_api;
pub use test_files::{is_test_file, is_test_dir};
pub use doc_chunker::{chunk_document, DocChunk};

// Keep spike modules behind cfg(test)
#[cfg(test)]
mod spike_ast_grep;
```

### A15. Tests

Test fixtures: create `zen-parser/tests/fixtures/` with small sample source files for each rich language.

**Unit tests** (`src/extractors/rust.rs` tests):
- Parse a Rust file with functions, structs, enums, traits, impl blocks
- Verify `ParsedItem` count and names
- Verify async detection (via `function_modifiers` child, not text prefix)
- Verify unsafe detection
- Verify visibility (`pub`, `pub(crate)`, private)
- Verify generics extraction (`<T: Clone + Send>`)
- Verify lifetime extraction (`<'a, 'b>`)
- Verify where clause extraction
- Verify doc comment extraction (`///` and `//!`)
- Verify attribute extraction (`#[derive()]`, `#[cfg()]`)
- Verify impl block processing (inherent vs trait impl)
- Verify enum variant extraction
- Verify struct field extraction
- Verify signature: no body leaks
- Verify error type detection (name pattern + `derive(Error)`)
- Verify PyO3 detection

**Unit tests** (`src/extractors/python.rs` tests):
- Parse Python file with classes, functions, decorators
- Verify docstring extraction (Google/NumPy/Sphinx styles)
- Verify dataclass/pydantic/protocol detection
- Verify decorator extraction
- Verify `@classmethod`, `@staticmethod`, `@property`
- Verify generator detection (yield)

**Unit tests** (`src/extractors/typescript.rs` tests):
- Parse TS file with functions, classes, interfaces, type aliases
- Verify export detection
- Verify JSDoc extraction
- Verify type parameter extraction

**Unit tests** (`src/extractors/go.rs` tests):
- Parse Go file with exported/unexported functions, types, methods
- Verify exported detection (uppercase first letter)
- Verify Go doc comment extraction

**Unit tests** (`src/extractors/elixir.rs` tests):
- Parse Elixir file with defmodule, def, defp, defmacro
- Verify @doc/@moduledoc extraction

**Unit tests** (`src/extractors/generic.rs` tests):
- Parse C, Java, Ruby, Swift files
- Verify basic name + signature extraction

**Unit tests** (`src/test_files.rs` tests):
- `is_test_file()` returns true for all test file patterns
- `is_test_file()` returns false for production files
- `is_test_dir()` returns true for all test directory names

**Unit tests** (`src/doc_chunker.rs` tests):
- Chunk a markdown file with headings
- Verify chunk boundaries align with headings
- Verify oversized sections get sub-chunked
- Verify empty sections are skipped

**Integration tests** (`tests/integration.rs`):
- Parse real Rust source (include_str! from tokio or similar)
- Parse real Python source
- Two-tier fallback: empty ast-grep result triggers regex

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
Phase 3 Execution:

 ┌──────────────────────────────────────────┐
 │ PR 1 (A) and PR 2 (B) can run in        │
 │ parallel — no dependencies between them  │
 └──────────────────────────────────────────┘

 1. [A1]   Create zen-parser/src/error.rs
 2. [A2]   Create zen-parser/src/types.rs
 3. [A3]   Create zen-parser/src/parser.rs
 4. [A4]   Create zen-parser/src/extractors/helpers.rs
 5. [A5]   Create zen-parser/src/extractors/rust.rs
 6. [A6]   Create zen-parser/src/extractors/python.rs
 7. [A7]   Create zen-parser/src/extractors/typescript.rs
 8. [A8]   Create zen-parser/src/extractors/go.rs
 9. [A9]   Create zen-parser/src/extractors/elixir.rs
10. [A10]  Create zen-parser/src/extractors/generic.rs
11. [A11]  Create zen-parser/src/extractors/mod.rs
12. [A12]  Create zen-parser/src/test_files.rs
13. [A13]  Create zen-parser/src/doc_chunker.rs
14. [A14]  Update zen-parser/src/lib.rs
15. [A15]  Create test fixtures + write all tests
    ─── cargo test -p zen-parser passes ───

    ┌─────────────────────────────────────┐
    │ PR 2 (B) in parallel with PR 1 (A) │
    └─────────────────────────────────────┘

16. [B1]   Create zen-embeddings/src/error.rs
17. [B2]   Rewrite zen-embeddings/src/lib.rs
18. [B3]   Update zen-embeddings/Cargo.toml (add dirs)
19. [B4]   Write zen-embeddings tests
    ─── cargo test -p zen-embeddings passes ───

    ┌────────────────────────────────────────────────────────┐
    │ PR 3 (C) can start after PR 2 lands (dims constant)   │
    │ but no runtime dependency — can start in parallel      │
    └────────────────────────────────────────────────────────┘

20. [C1]   Create zen-lake/src/error.rs
21. [C2]   Create zen-lake/src/schemas.rs
22. [C3]   Rewrite zen-lake/src/lib.rs
23. [C4]   Create zen-lake/src/store.rs
24. [C5]   Create zen-lake/src/source_files.rs
25. [C6]   Write zen-lake tests
    ─── cargo test -p zen-lake passes ───

    ┌─────────────────────────────────────────────────┐
    │ PR 4 (D) must wait for all 3 PRs above to land │
    └─────────────────────────────────────────────────┘

26. [D1]   Create zen-search/src/walk.rs
27. [D1b]  Update zen-search/src/lib.rs + Cargo.toml (add zen-parser dep)
28. [D2]   Create zen-cli/src/pipeline.rs (pipeline orchestration)
29. [D2b]  Update zen-cli/Cargo.toml (add zen-parser, sha2 deps)
30. [D3]   Write integration tests
    ─── cargo test --workspace passes (Phase 3) ───
```

Steps 1–15 and 16–19 are independent and can be parallelized.
Steps 20–25 can start once types are defined (no runtime dependency).

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

- [ ] All tests pass (zen-parser, zen-embeddings, zen-lake, zen-search)
- [ ] `zen-parser` extracts rich API symbols from Rust, Python, TypeScript, Go, Elixir source files
- [ ] `zen-parser` extracts basic symbols from at least 3 non-rich languages (C, Java, Ruby)
- [ ] All extracted `ParsedItem` structs have correct: kind, name, signature (no body), visibility, start/end lines
- [ ] Rust extractor: async/unsafe detection, generics, lifetimes, doc comments, attributes, impl block methods, enum variants, struct fields, error types
- [ ] Python extractor: classes, decorators, docstrings (at least Google style)
- [ ] TypeScript extractor: exports, interfaces, type aliases, JSDoc
- [ ] Go extractor: exported detection, doc comments, methods
- [ ] Generic extractor: produces ≥1 item for C, Java, Ruby test fixtures
- [ ] Test file detection: `is_test_file()` and `is_test_dir()` correct for all patterns
- [ ] `zen-embeddings` generates 384-dim vectors, similar texts cluster
- [ ] `zen-lake` stores and retrieves symbols, doc chunks in DuckDB local cache (`.zenith/lake/cache.duckdb`)
- [ ] `SourceFileStore` stores and retrieves source files in separate DuckDB (`.zenith/source_files.duckdb`)
- [ ] `array_cosine_similarity()` works on stored embeddings (FLOAT[] → FLOAT[384] cast) — local cache only
- [ ] Walker: `build_walker()` with `WalkMode::Raw` and `WalkMode::LocalProject` both produce correct file lists
- [ ] Full pipeline: index a temp directory with mixed-language source → all tables populated correctly
- [ ] `cargo build --workspace` still succeeds (no regressions)

### What This Unlocks

Phase 3 completion unblocks:
- **Phase 4** (Search & Registry): Vector/FTS/hybrid search over local DuckDB cache; grep over source_files; recursive query
- **Phase 5** (CLI): `znt install`, `znt search`, `znt grep`, `znt cache` commands (local mode)
- **Phase 8/9** (Cloud): lancedb writes to R2 + Turso catalog registration (introduces `ProductionApiSymbol`/`ProductionDocChunk` structs as `serde_arrow` sources per spike 0.19; replaces DuckDB cache tables with Lance datasets + Turso `dl_data_file`)

---

## 12. Validation Traceability Matrix

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
| Two-tier fallback | ast-grep empty → regex produces items | Design-only | `05-crate-designs.md` §7 | crate design |
| Doc chunking | Split by heading, max ~512 tokens | Design-only | `02-data-architecture.md` §8 | data architecture |
| Full pipeline | clone → walk → parse → embed → store | Design-only | `02-data-architecture.md` §8, `07-implementation-plan.md` task 3.14 | integration |
| Signature normalization | Whitespace collapse for deterministic signatures | Validated | `extract_signature_from_node_text()` in 600+ files | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Doc comment line-based fallback | Line-based `leading_doc_comment()` robust on large repos | Validated | 14,929 symbols extracted from Arrow monorepo | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Extended impl extraction | Generic/scoped/trait impl patterns need extended queries | Validated | +580 matches vs baseline on Arrow monorepo | `zen-search/src/spike_recursive_query.rs` (spike 0.21) |
| Lance production write path | `serde_arrow` → `lancedb` (not DuckDB COPY) | Validated | `spike_serde_arrow_production_path` (50 rows round-trip) | `zen-lake/src/spike_native_lance.rs` (spike 0.19) |
| serde_arrow FixedSizeList override | `embedding` must be overridden to `FixedSizeList(384)` | Validated | Spike 0.19 test M1 | `zen-lake/src/spike_native_lance.rs` (spike 0.19) |
| Turso catalog visibility | `dl_data_file` with public/team/private scoping | Validated | 9/9 tests | `zen-db/src/spike_catalog_visibility.rs` (spike 0.20) |
| Lance on R2 — vector/FTS/hybrid search | `lance_vector_search`, `lance_fts`, `lance_hybrid_search` | Validated | 18/18 tests | `zen-lake/src/spike_r2_parquet.rs` (spike 0.18) |

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

//! `DuckDB` table DDL and row structs for the local lake cache.
//!
//! **Scope**: This is the Phase 3 *local-only* `DuckDB` cache. Production storage
//! (Lance on R2 + Turso catalog) replaces `api_symbols` and `doc_chunks` tables
//! in Phase 8/9. The `source_files` table (in a separate `DuckDB` file) is permanent.
//! See `23-phase3-parsing-indexing-plan.md` §13 for the replacement map.

use serde::{Deserialize, Serialize};

// ── Table DDL (local cache.duckdb) ─────────────────────────────────────────

/// Indexed packages tracking table.
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

/// API symbols table — tree-sitter-extracted public API symbols with embeddings.
///
/// Embeddings are stored as `FLOAT[]` (variable-length) and cast to `FLOAT[384]`
/// at query time for `array_cosine_similarity()`. See plan §3.5 for rationale.
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

/// Documentation chunks table — split documentation with embeddings.
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

/// Indexes for efficient querying.
pub const CREATE_INDEXES: &str = "
CREATE INDEX IF NOT EXISTS idx_symbols_pkg
    ON api_symbols(ecosystem, package, version);
CREATE INDEX IF NOT EXISTS idx_symbols_kind
    ON api_symbols(ecosystem, package, version, kind);
CREATE INDEX IF NOT EXISTS idx_symbols_name
    ON api_symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_file_lines
    ON api_symbols(ecosystem, package, version, file_path, line_start, line_end);
CREATE INDEX IF NOT EXISTS idx_doc_chunks_pkg
    ON doc_chunks(ecosystem, package, version);
";

// ── Row structs ────────────────────────────────────────────────────────────

/// A row in the `api_symbols` table. Used for insertion and query results.
///
/// Column names are aligned with the production Lance schema (see
/// `02-data-architecture.md` §7) for straightforward Phase 8/9 migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Mirrors DuckDB schema; 4 independent boolean columns
pub struct ApiSymbolRow {
    /// Deterministic hash: `sha256(ecosystem:package:version:file_path:kind:name)[..16]`.
    pub id: String,
    /// Package ecosystem: `rust`, `npm`, `pypi`, `hex`, `go`.
    pub ecosystem: String,
    /// Package name as published in registry.
    pub package: String,
    /// Exact version indexed.
    pub version: String,
    /// Relative path within repo.
    pub file_path: String,
    /// Symbol kind: `function`, `struct`, `enum`, `trait`, `class`, etc.
    pub kind: String,
    /// Symbol name as it appears in source.
    pub name: String,
    /// Full signature line (no body).
    pub signature: Option<String>,
    /// Source code body.
    pub source: Option<String>,
    /// Extracted doc comment.
    pub doc_comment: Option<String>,
    /// Start line in source file.
    pub line_start: Option<i32>,
    /// End line in source file.
    pub line_end: Option<i32>,
    /// Visibility: `pub`, `pub(crate)`, `private`, `export`.
    pub visibility: Option<String>,
    /// Whether the symbol is async.
    pub is_async: bool,
    /// Whether the symbol is unsafe.
    pub is_unsafe: bool,
    /// Whether it's an error type.
    pub is_error_type: bool,
    /// Whether it returns `Result`.
    pub returns_result: bool,
    /// Return type (e.g., `Result<(), Error>`).
    pub return_type: Option<String>,
    /// Generic parameters (e.g., `<T: Clone>`).
    pub generics: Option<String>,
    /// JSON array of attributes/decorators as string.
    pub attributes: Option<String>,
    /// JSON object of language-specific metadata as string.
    pub metadata: Option<String>,
    /// 384-dim fastembed embedding. Empty vec if not yet embedded.
    pub embedding: Vec<f32>,
}

/// A row in the `doc_chunks` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocChunkRow {
    /// Deterministic hash: `sha256(ecosystem:package:version:source_file:chunk_index)[..16]`.
    pub id: String,
    /// Package ecosystem.
    pub ecosystem: String,
    /// Package name.
    pub package: String,
    /// Package version.
    pub version: String,
    /// Sequential index within source file.
    pub chunk_index: i32,
    /// Section heading (if any).
    pub title: Option<String>,
    /// Chunk text content.
    pub content: String,
    /// Relative path: `README.md`, `docs/guide.md`.
    pub source_file: Option<String>,
    /// Document format: `md`, `rst`, `txt`.
    pub format: Option<String>,
    /// 384-dim fastembed embedding. Empty vec if not yet embedded.
    pub embedding: Vec<f32>,
}

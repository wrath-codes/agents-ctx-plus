//! # zen-search
//!
//! Search orchestration for Zenith combining vector and full-text search.
//!
//! Coordinates between:
//! - HNSW vector search in DuckDB (semantic similarity over API symbols and doc chunks)
//! - FTS5 full-text search in Turso (keyword search over findings, tasks, audit trail)
//! - Result ranking and deduplication

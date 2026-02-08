//! # zen-lake
//!
//! DuckDB/DuckLake operations for the Zenith documentation lake.
//!
//! Stores indexed package documentation: API symbols (from ast-grep extraction)
//! and doc chunks (from markdown parsing) with fastembed vector embeddings.
//! Provides HNSW vector search and full-text search over indexed content.

#[cfg(test)]
mod spike_duckdb;

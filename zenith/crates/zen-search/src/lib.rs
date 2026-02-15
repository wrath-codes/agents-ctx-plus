//! # zen-search
//!
//! Search orchestration for Zenith combining vector, full-text, and grep search.
//!
//! Coordinates between:
//! - HNSW vector search in `DuckDB` (semantic similarity over API symbols and doc chunks)
//! - FTS5 full-text search in Turso (keyword search over findings, tasks, audit trail)
//! - Grep search via ripgrep library (local project) and `DuckDB` (indexed packages)
//! - Result ranking and deduplication

pub mod error;
pub mod fts;
pub mod grep;
pub mod hybrid;
pub mod vector;
pub mod walk;

#[cfg(test)]
mod spike_graph_algorithms;
#[cfg(test)]
mod spike_grep;
#[cfg(test)]
mod spike_recursive_query;

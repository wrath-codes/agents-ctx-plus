//! # zen-search
//!
//! Search orchestration for Zenith combining vector, full-text, and grep search.
//!
//! Coordinates between:
//! - HNSW vector search in DuckDB (semantic similarity over API symbols and doc chunks)
//! - FTS5 full-text search in Turso (keyword search over findings, tasks, audit trail)
//! - Grep search via ripgrep library (local project) and DuckDB (indexed packages)
//! - Result ranking and deduplication
//!
//! ## Walking
//!
//! The `walk` module provides a file walker factory using the `ignore` crate.
//! Used by both the indexing pipeline and `znt grep` local mode.
//!
//! ## Grep
//!
//! The grep module (behind `#[cfg(test)]` spikes) validates ripgrep integration.
//! Production grep implementation for local mode will be added in Phase 5.
//!
//! ## Graph Algorithms
//!
//! The graph algorithms module (behind `#[cfg(test)]` spikes) validates `rustworkx-core`
//! for decision trace analytics. This remains test-only in Phase 3 per spec §10.12.
//! Production graph queries will be added in Phase 4 (Search & Registry).

pub mod walk;

#[cfg(test)]
mod spike_graph_algorithms;
#[cfg(test)]
mod spike_grep;
#[cfg(test)]
mod spike_recursive_query;

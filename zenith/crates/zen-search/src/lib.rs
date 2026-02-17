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
pub mod graph;
pub mod grep;
pub mod hybrid;
pub mod recursive;
pub mod ref_graph;
pub mod vector;
pub mod walk;

pub use error::SearchError;
pub use fts::{FtsSearchFilters, FtsSearchResult};
pub use graph::{DecisionGraph, GraphAnalysis, GraphEdge, GraphNode};
pub use grep::{GrepEngine, GrepMatch, GrepOptions, GrepResult, GrepStats, SymbolRef};
pub use hybrid::{HybridSearchResult, HybridSource};
pub use recursive::{
    BudgetUsed, ContextStore, DocSpan, FileContext, RecursiveBudget, RecursiveQuery,
    RecursiveQueryEngine, RecursiveQueryPlan, RecursiveQueryResult, SymbolSpan,
};
pub use ref_graph::{RefCategory, RefEdge, ReferenceGraph, SymbolRefHit};
pub use vector::{VectorSearchFilters, VectorSearchResult, VectorSource};
pub use walk::{WalkMode, build_walker};

use std::cmp::Ordering;
use std::path::Path;

use zen_db::service::ZenService;
use zen_embeddings::EmbeddingEngine;
use zen_lake::{SourceFileStore, ZenLake};

/// Search mode routed by [`SearchEngine`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    /// Semantic vector search over API symbols and doc chunks.
    Vector,
    /// FTS5 keyword search over zen-db entities.
    Fts,
    /// Combined vector + FTS search with alpha blending.
    Hybrid { alpha: f64 },
    /// Recursive context mode. Dispatched through `RecursiveQueryEngine` directly.
    Recursive,
    /// Decision graph analytics mode. Available after Stream C lands.
    Graph,
}

/// Common filters applied to search operations.
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub package: Option<String>,
    pub ecosystem: Option<String>,
    pub kind: Option<String>,
    pub entity_types: Vec<String>,
    pub limit: Option<u32>,
    pub min_score: Option<f64>,
}

/// Unified result type for orchestrated search output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "source")]
pub enum SearchResult {
    #[serde(rename = "vector")]
    Vector(VectorSearchResult),
    #[serde(rename = "fts")]
    Fts(FtsSearchResult),
    #[serde(rename = "hybrid")]
    Hybrid(HybridSearchResult),
    #[serde(rename = "recursive")]
    Recursive(RecursiveQueryResult),
    #[serde(rename = "graph")]
    Graph(GraphAnalysis),
}

/// Orchestrator over vector, FTS, and hybrid search.
///
/// This type borrows all resources from the caller. The embedding engine is
/// mutable because `embed_single()` takes `&mut self`.
pub struct SearchEngine<'a> {
    service: &'a ZenService,
    lake: &'a ZenLake,
    #[allow(dead_code)]
    source_store: &'a SourceFileStore,
    embeddings: &'a mut EmbeddingEngine,
}

impl<'a> SearchEngine<'a> {
    /// Construct a new search engine from borrowed dependencies.
    #[must_use]
    pub const fn new(
        service: &'a ZenService,
        lake: &'a ZenLake,
        source_store: &'a SourceFileStore,
        embeddings: &'a mut EmbeddingEngine,
    ) -> Self {
        Self {
            service,
            lake,
            source_store,
            embeddings,
        }
    }

    /// Execute a query in the requested mode.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::InvalidQuery`] for unsupported/invalid modes and
    /// empty queries, or mode-specific backend errors.
    #[allow(clippy::future_not_send)]
    pub async fn search(
        &mut self,
        query: &str,
        mode: SearchMode,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult>, SearchError> {
        if query.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "search query cannot be empty".to_string(),
            ));
        }

        match mode {
            SearchMode::Vector => {
                let embedding = self.embeddings.embed_single(query)?;
                let limit = normalize_limit(filters.limit, 20);
                let vf = VectorSearchFilters {
                    package: filters.package,
                    ecosystem: filters.ecosystem,
                    kind: filters.kind,
                    limit,
                    min_score: filters.min_score.unwrap_or(0.0),
                };

                let mut vector_results = vector::vector_search_symbols(self.lake, &embedding, &vf)?;
                vector_results.extend(vector::vector_search_doc_chunks(
                    self.lake, &embedding, &vf,
                )?);
                sort_vector_results(&mut vector_results);

                #[allow(clippy::cast_possible_truncation)]
                vector_results.truncate(limit as usize);

                Ok(vector_results
                    .into_iter()
                    .map(SearchResult::Vector)
                    .collect())
            }
            SearchMode::Fts => {
                let limit = normalize_limit(filters.limit, 20);
                let ff = fts::FtsSearchFilters {
                    entity_types: filters.entity_types,
                    limit,
                };

                let results = fts::fts_search(self.service, query, &ff).await?;
                Ok(results.into_iter().map(SearchResult::Fts).collect())
            }
            SearchMode::Hybrid { alpha } => {
                let embedding = self.embeddings.embed_single(query)?;
                let limit = normalize_limit(filters.limit, 20);

                let vf = VectorSearchFilters {
                    package: filters.package,
                    ecosystem: filters.ecosystem,
                    kind: filters.kind,
                    limit: limit.max(40),
                    min_score: 0.0,
                };
                let mut vector_results = vector::vector_search_symbols(self.lake, &embedding, &vf)?;
                vector_results.extend(vector::vector_search_doc_chunks(
                    self.lake, &embedding, &vf,
                )?);

                let ff = fts::FtsSearchFilters {
                    entity_types: filters.entity_types,
                    limit: limit.max(40),
                };
                let fts_results = fts::fts_search(self.service, query, &ff).await?;

                let combined = hybrid::combine_results(&vector_results, &fts_results, alpha, limit);
                Ok(combined.into_iter().map(SearchResult::Hybrid).collect())
            }
            SearchMode::Recursive => {
                let engine = RecursiveQueryEngine::from_directory(
                    Path::new("."),
                    RecursiveBudget::default(),
                )?;
                let rq = RecursiveQuery::from_text(query);
                let result = engine.execute(&rq)?;
                Ok(vec![SearchResult::Recursive(result)])
            }
            SearchMode::Graph => {
                let graph = graph::DecisionGraph::from_service(self.service).await?;
                let analysis = graph.analyze(1_000);
                Ok(vec![SearchResult::Graph(analysis)])
            }
        }
    }
}

const fn normalize_limit(limit: Option<u32>, default_limit: u32) -> u32 {
    match limit {
        Some(0) | None => default_limit,
        Some(v) => v,
    }
}

fn sort_vector_results(results: &mut [VectorSearchResult]) {
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod spike_graph_algorithms;
#[cfg(test)]
mod spike_grep;
#[cfg(test)]
mod spike_recursive_query;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_limit_uses_default_for_none_and_zero() {
        assert_eq!(normalize_limit(None, 20), 20);
        assert_eq!(normalize_limit(Some(0), 20), 20);
        assert_eq!(normalize_limit(Some(7), 20), 7);
    }

    #[test]
    fn sort_vector_results_is_deterministic_on_ties() {
        let mut rows = vec![
            VectorSearchResult {
                id: "b".to_string(),
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.0.0".to_string(),
                kind: "function".to_string(),
                name: "spawn".to_string(),
                signature: None,
                doc_comment: None,
                file_path: "src/lib.rs".to_string(),
                line_start: None,
                line_end: None,
                score: 0.9,
                source_type: VectorSource::ApiSymbol,
            },
            VectorSearchResult {
                id: "a".to_string(),
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.0.0".to_string(),
                kind: "function".to_string(),
                name: "spawn".to_string(),
                signature: None,
                doc_comment: None,
                file_path: "src/lib.rs".to_string(),
                line_start: None,
                line_end: None,
                score: 0.9,
                source_type: VectorSource::ApiSymbol,
            },
        ];

        sort_vector_results(&mut rows);
        assert_eq!(rows[0].id, "a");
        assert_eq!(rows[1].id, "b");
    }
}

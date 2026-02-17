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
    pub version: Option<String>,
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
                let result = execute_recursive_query(self.source_store, query, &filters)?;
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

fn recursive_package_triplet(filters: &SearchFilters) -> Option<(&str, &str, &str)> {
    let ecosystem = filters.ecosystem.as_deref()?;
    let package = filters.package.as_deref()?;
    let version = filters.version.as_deref()?;
    Some((ecosystem, package, version))
}

fn execute_recursive_query(
    source_store: &SourceFileStore,
    query: &str,
    filters: &SearchFilters,
) -> Result<RecursiveQueryResult, SearchError> {
    let engine = if let Some((ecosystem, package, version)) = recursive_package_triplet(filters) {
        RecursiveQueryEngine::from_source_store(
            source_store,
            ecosystem,
            package,
            version,
            RecursiveBudget::default(),
        )?
    } else {
        RecursiveQueryEngine::from_directory(Path::new("."), RecursiveBudget::default())?
    };

    let rq = RecursiveQuery::from_text(query);
    engine.execute(&rq)
}

#[cfg(test)]
mod spike_graph_algorithms;
#[cfg(test)]
mod spike_grep;
#[cfg(test)]
mod spike_recursive_query;

#[cfg(test)]
mod tests {
    use std::env;
    use std::io::Write;
    use std::path::Path;

    use tempfile::TempDir;
    use zen_lake::SourceFile;

    use super::*;

    struct DirGuard {
        previous: std::path::PathBuf,
    }

    impl DirGuard {
        fn enter(path: &Path) -> Self {
            let previous = env::current_dir().expect("read current dir");
            env::set_current_dir(path).expect("set current dir");
            Self { previous }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.previous);
        }
    }

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

    #[test]
    fn recursive_package_triplet_requires_all_fields() {
        let complete = SearchFilters {
            ecosystem: Some("rust".to_string()),
            package: Some("tokio".to_string()),
            version: Some("1.0.0".to_string()),
            ..SearchFilters::default()
        };
        assert_eq!(
            recursive_package_triplet(&complete),
            Some(("rust", "tokio", "1.0.0"))
        );

        let missing_version = SearchFilters {
            ecosystem: Some("rust".to_string()),
            package: Some("tokio".to_string()),
            ..SearchFilters::default()
        };
        assert_eq!(recursive_package_triplet(&missing_version), None);
    }

    #[test]
    fn execute_recursive_query_uses_source_store_when_triplet_present() {
        let store = SourceFileStore::open_in_memory().expect("open source store");
        store
            .store_source_files(&[SourceFile {
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.0.0".to_string(),
                file_path: "src/lib.rs".to_string(),
                content: "/// safety invariant\npub fn spawn() {}".to_string(),
                language: Some("rust".to_string()),
                size_bytes: 40,
                line_count: 2,
            }])
            .expect("seed source files");

        let filters = SearchFilters {
            ecosystem: Some("rust".to_string()),
            package: Some("tokio".to_string()),
            version: Some("1.0.0".to_string()),
            ..SearchFilters::default()
        };

        let result = execute_recursive_query(&store, "safety", &filters).expect("recursive query");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].name, "spawn");
    }

    #[test]
    fn execute_recursive_query_falls_back_to_local_without_triplet() {
        let tmp = TempDir::new().expect("temp dir");
        let _guard = DirGuard::enter(tmp.path());
        std::fs::create_dir_all("src").expect("create src");
        let mut file = std::fs::File::create("src/lib.rs").expect("create file");
        writeln!(file, "/// local safety note").expect("write doc");
        writeln!(file, "pub fn local_fn() {{}} ").expect("write fn");

        let store = SourceFileStore::open_in_memory().expect("open source store");
        let filters = SearchFilters {
            ecosystem: Some("rust".to_string()),
            package: Some("tokio".to_string()),
            version: None,
            ..SearchFilters::default()
        };

        let result = execute_recursive_query(&store, "local", &filters).expect("recursive query");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].name, "local_fn");
    }
}

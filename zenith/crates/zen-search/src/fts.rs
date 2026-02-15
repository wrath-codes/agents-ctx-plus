//! Full-text search adapter over zen-db FTS5-indexed entities.
//!
//! Thin wrapper that queries each entity type's FTS5 virtual table
//! via `ZenService` search methods and normalizes results into a
//! uniform [`FtsSearchResult`] type.
//!
//! FTS5 uses porter stemming: "spawning" matches "spawn", "runtime"
//! matches "runtimes".

use zen_db::service::ZenService;

use crate::error::SearchError;

/// Result from an FTS5 full-text search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtsSearchResult {
    /// Entity type: "finding", "hypothesis", "insight", "research", "task", "issue", "study", "audit".
    pub entity_type: String,
    /// Entity ID (e.g., "fnd-a3f8b2c1").
    pub entity_id: String,
    /// Title or heading (if the entity has one).
    pub title: Option<String>,
    /// Primary text content.
    pub content: String,
    /// FTS5 relevance score (normalized to positive; higher = more relevant).
    pub relevance: f64,
}

/// Filters for FTS5 search queries.
#[derive(Debug, Clone)]
pub struct FtsSearchFilters {
    /// Filter to specific entity types. Empty means search all types.
    pub entity_types: Vec<String>,
    /// Maximum number of results per entity type.
    pub limit: u32,
}

impl Default for FtsSearchFilters {
    fn default() -> Self {
        Self {
            entity_types: Vec::new(),
            limit: 20,
        }
    }
}

/// All searchable entity type names.
const ALL_ENTITY_TYPES: &[&str] = &[
    "finding",
    "hypothesis",
    "insight",
    "research",
    "task",
    "issue",
    "study",
    "audit",
];

/// Search across all FTS5-indexed knowledge entities in zen-db.
///
/// Queries each entity type's FTS5 table via `ZenService` and normalizes
/// results into [`FtsSearchResult`]. Results are sorted by relevance
/// (descending) within each entity type.
///
/// # Arguments
///
/// * `service` — The zen-db service providing search methods.
/// * `query` — The FTS5 search query (supports porter stemming).
/// * `filters` — Optional entity type filtering and result limits.
///
/// # Errors
///
/// Returns [`SearchError::InvalidQuery`] if the query is empty.
/// Returns [`SearchError::Database`] if any FTS5 query fails.
#[allow(clippy::too_many_lines)]
pub async fn fts_search(
    service: &ZenService,
    query: &str,
    filters: &FtsSearchFilters,
) -> Result<Vec<FtsSearchResult>, SearchError> {
    if query.trim().is_empty() {
        return Err(SearchError::InvalidQuery(
            "search query cannot be empty".to_string(),
        ));
    }

    let types_to_search: Vec<&str> = if filters.entity_types.is_empty() {
        ALL_ENTITY_TYPES.to_vec()
    } else {
        filters
            .entity_types
            .iter()
            .map(String::as_str)
            .filter(|t| ALL_ENTITY_TYPES.contains(t))
            .collect()
    };

    let mut results = Vec::new();

    for entity_type in &types_to_search {
        // Collect each entity type's results into a batch, then assign
        // per-type positional relevance scores. zen-db search methods
        // return results ordered by FTS5 rank, so position within a
        // batch reflects true relevance ordering.
        let batch_start = results.len();

        match *entity_type {
            "finding" => {
                let findings = service.search_findings(query, filters.limit).await?;
                for f in findings {
                    results.push(FtsSearchResult {
                        entity_type: "finding".to_string(),
                        entity_id: f.id,
                        title: None,
                        content: f.content,
                        relevance: 0.0,
                    });
                }
            }
            "hypothesis" => {
                let hypotheses = service.search_hypotheses(query, filters.limit).await?;
                for h in hypotheses {
                    results.push(FtsSearchResult {
                        entity_type: "hypothesis".to_string(),
                        entity_id: h.id,
                        title: None,
                        content: h.content,
                        relevance: 0.0,
                    });
                }
            }
            "insight" => {
                let insights = service.search_insights(query, filters.limit).await?;
                for i in insights {
                    results.push(FtsSearchResult {
                        entity_type: "insight".to_string(),
                        entity_id: i.id,
                        title: None,
                        content: i.content,
                        relevance: 0.0,
                    });
                }
            }
            "research" => {
                let items = service.search_research(query, filters.limit).await?;
                for r in items {
                    results.push(FtsSearchResult {
                        entity_type: "research".to_string(),
                        entity_id: r.id,
                        title: Some(r.title),
                        content: r.description.unwrap_or_default(),
                        relevance: 0.0,
                    });
                }
            }
            "task" => {
                let tasks = service.search_tasks(query, filters.limit).await?;
                for t in tasks {
                    results.push(FtsSearchResult {
                        entity_type: "task".to_string(),
                        entity_id: t.id,
                        title: Some(t.title),
                        content: t.description.unwrap_or_default(),
                        relevance: 0.0,
                    });
                }
            }
            "issue" => {
                let issues = service.search_issues(query, filters.limit).await?;
                for i in issues {
                    results.push(FtsSearchResult {
                        entity_type: "issue".to_string(),
                        entity_id: i.id,
                        title: Some(i.title),
                        content: i.description.unwrap_or_default(),
                        relevance: 0.0,
                    });
                }
            }
            "study" => {
                let studies = service.search_studies(query, filters.limit).await?;
                for s in studies {
                    results.push(FtsSearchResult {
                        entity_type: "study".to_string(),
                        entity_id: s.id,
                        title: Some(s.topic),
                        content: s.summary.unwrap_or_default(),
                        relevance: 0.0,
                    });
                }
            }
            "audit" => {
                let entries = service.search_audit(query, filters.limit).await?;
                for a in entries {
                    results.push(FtsSearchResult {
                        entity_type: "audit".to_string(),
                        entity_id: a.id,
                        title: None,
                        content: a
                            .detail
                            .map_or_else(|| a.entity_id, |v| v.to_string()),
                        relevance: 0.0,
                    });
                }
            }
            _ => {}
        }

        // Assign positional relevance within this entity type's batch.
        // Position 0 (most relevant per FTS5 rank) gets score 1.0,
        // last position gets score approaching 0.0.
        let batch_len = results.len() - batch_start;
        if batch_len > 0 {
            #[allow(clippy::cast_precision_loss)]
            for (i, result) in results[batch_start..].iter_mut().enumerate() {
                result.relevance = (batch_len - i) as f64 / batch_len as f64;
            }
        }
    }

    Ok(results)
}

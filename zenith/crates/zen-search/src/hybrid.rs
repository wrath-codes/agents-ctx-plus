//! Hybrid search combining vector similarity and FTS5 relevance.
//!
//! Uses configurable alpha blending to merge results from `DuckDB` vector
//! search and libSQL FTS5 search. Handles score normalization, deduplication,
//! and ranking.
//!
//! Alpha controls the blend:
//! - `0.0` = FTS only
//! - `1.0` = vector only  
//! - `0.7` (default) = favors semantic similarity

use std::collections::HashMap;

use crate::fts::FtsSearchResult;
use crate::vector::{VectorSearchResult, VectorSource};

/// Result from a hybrid search combining vector and FTS results.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridSearchResult {
    /// Result identifier.
    pub id: String,
    /// Symbol name, chunk title, or entity title.
    pub name: String,
    /// Kind: "function", "struct", "doc\_chunk", "finding", etc.
    pub kind: String,
    /// Primary text content.
    pub content: String,
    /// Normalized vector similarity score (if present).
    pub vector_score: Option<f64>,
    /// Normalized FTS relevance score (if present).
    pub fts_score: Option<f64>,
    /// Alpha-blended combined score.
    pub combined_score: f64,
    /// Source of the result.
    pub source: HybridSource,
}

/// Source of a hybrid search result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum HybridSource {
    /// From vector search on `api_symbols`.
    VectorSymbol,
    /// From vector search on `doc_chunks`.
    VectorDocChunk,
    /// From FTS5 full-text search.
    Fts,
}

/// Normalize a cosine similarity score from [-1, 1] to [0, 1].
const fn normalize_vector_score(score: f64) -> f64 {
    f64::midpoint(score, 1.0)
}

/// Combine vector and FTS results with alpha blending.
///
/// `alpha` controls the blend: `0.0` = FTS only, `1.0` = vector only.
/// Results are deduplicated by name and ranked by combined score.
///
/// # Arguments
///
/// * `vector_results` — Results from vector similarity search.
/// * `fts_results` — Results from FTS5 search.
/// * `alpha` — Blending weight: `0.0` (FTS only) to `1.0` (vector only).
/// * `limit` — Maximum number of results to return.
#[must_use]
pub fn combine_results(
    vector_results: &[VectorSearchResult],
    fts_results: &[FtsSearchResult],
    alpha: f64,
    limit: u32,
) -> Vec<HybridSearchResult> {
    let alpha = alpha.clamp(0.0, 1.0);

    // Dedup key -> accumulated result
    let mut merged: HashMap<String, HybridSearchResult> = HashMap::new();

    // Add vector results
    for vr in vector_results {
        let norm_score = normalize_vector_score(vr.score);

        let key = vr.name.to_lowercase();
        let entry = merged.entry(key).or_insert_with(|| HybridSearchResult {
            id: vr.id.clone(),
            name: vr.name.clone(),
            kind: vr.kind.clone(),
            content: vr.doc_comment.clone().unwrap_or_default(),
            vector_score: None,
            fts_score: None,
            combined_score: 0.0,
            source: match vr.source_type {
                VectorSource::ApiSymbol => HybridSource::VectorSymbol,
                VectorSource::DocChunk => HybridSource::VectorDocChunk,
            },
        });
        entry.vector_score = Some(norm_score);
        entry.combined_score = alpha * norm_score + (1.0 - alpha) * entry.fts_score.unwrap_or(0.0);
    }

    // Add FTS results
    for fr in fts_results {
        let key = fr.title.as_deref().unwrap_or(&fr.entity_id).to_lowercase();

        let entry = merged.entry(key).or_insert_with(|| HybridSearchResult {
            id: fr.entity_id.clone(),
            name: fr.title.clone().unwrap_or_else(|| fr.entity_id.clone()),
            kind: fr.entity_type.clone(),
            content: fr.content.clone(),
            vector_score: None,
            fts_score: None,
            combined_score: 0.0,
            source: HybridSource::Fts,
        });
        entry.fts_score = Some(fr.relevance);
        entry.combined_score =
            alpha * entry.vector_score.unwrap_or(0.0) + (1.0 - alpha) * fr.relevance;
    }

    let mut results: Vec<HybridSearchResult> = merged.into_values().collect();
    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    #[allow(clippy::cast_possible_truncation)]
    results.truncate(limit as usize);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fts::FtsSearchResult;
    use crate::vector::{VectorSearchResult, VectorSource};

    fn make_vector_result(id: &str, name: &str, score: f64) -> VectorSearchResult {
        VectorSearchResult {
            id: id.to_string(),
            ecosystem: "rust".to_string(),
            package: "tokio".to_string(),
            version: "1.0.0".to_string(),
            kind: "function".to_string(),
            name: name.to_string(),
            signature: None,
            doc_comment: Some(format!("Doc for {name}")),
            file_path: "src/lib.rs".to_string(),
            line_start: None,
            line_end: None,
            score,
            source_type: VectorSource::ApiSymbol,
        }
    }

    fn make_fts_result(id: &str, title: &str, relevance: f64) -> FtsSearchResult {
        FtsSearchResult {
            entity_type: "finding".to_string(),
            entity_id: id.to_string(),
            title: Some(title.to_string()),
            content: format!("Content about {title}"),
            relevance,
        }
    }

    #[test]
    fn alpha_1_vector_only() {
        let vec_results = vec![
            make_vector_result("v1", "spawn", 0.9),
            make_vector_result("v2", "select", 0.7),
        ];
        let fts_results = vec![make_fts_result("f1", "unrelated", 1.0)];

        let results = combine_results(&vec_results, &fts_results, 1.0, 10);

        // With alpha=1.0, FTS scores contribute 0. Vector results should dominate.
        let spawn = results.iter().find(|r| r.name == "spawn").unwrap();
        assert!(spawn.fts_score.is_none(), "spawn should not have FTS score");
        assert!(spawn.vector_score.is_some());
        // FTS-only result should have 0 combined score
        let unrelated = results.iter().find(|r| r.name == "unrelated").unwrap();
        assert!(
            unrelated.combined_score < f64::EPSILON,
            "alpha=1.0 should zero out FTS-only results"
        );
    }

    #[test]
    fn alpha_0_fts_only() {
        let vec_results = vec![make_vector_result("v1", "spawn", 0.9)];
        let fts_results = vec![
            make_fts_result("f1", "runtime", 0.9),
            make_fts_result("f2", "async", 0.5),
        ];

        let results = combine_results(&vec_results, &fts_results, 0.0, 10);

        // With alpha=0.0, vector scores contribute 0
        let spawn = results.iter().find(|r| r.name == "spawn").unwrap();
        assert!(
            spawn.combined_score < f64::EPSILON,
            "alpha=0.0 should zero out vector-only results"
        );
        let runtime = results.iter().find(|r| r.name == "runtime").unwrap();
        assert!(runtime.combined_score > 0.0);
    }

    #[test]
    fn alpha_05_equal_blend() {
        let vec_results = vec![make_vector_result("v1", "spawn", 0.8)]; // normalized: (0.8+1)/2 = 0.9
        let fts_results = vec![make_fts_result("f1", "runtime", 0.6)];

        let results = combine_results(&vec_results, &fts_results, 0.5, 10);

        let spawn = results.iter().find(|r| r.name == "spawn").unwrap();
        let expected_spawn = 0.5 * normalize_vector_score(0.8);
        assert!(
            (spawn.combined_score - expected_spawn).abs() < 0.01,
            "spawn combined should be {expected_spawn:.3}, got {:.3}",
            spawn.combined_score
        );

        let runtime = results.iter().find(|r| r.name == "runtime").unwrap();
        let expected_runtime = 0.5 * 0.6;
        assert!(
            (runtime.combined_score - expected_runtime).abs() < 0.01,
            "runtime combined should be {expected_runtime:.3}, got {:.3}",
            runtime.combined_score
        );
    }

    #[test]
    fn dedup_same_entity() {
        // Same name appears in both vector and FTS results
        let vec_results = vec![make_vector_result("v1", "spawn", 0.8)];
        let fts_results = vec![make_fts_result("f1", "spawn", 0.6)];

        let results = combine_results(&vec_results, &fts_results, 0.5, 10);

        // Should be merged into one result
        let spawn_results: Vec<_> = results
            .iter()
            .filter(|r| r.name.to_lowercase() == "spawn")
            .collect();
        assert_eq!(spawn_results.len(), 1, "duplicate names should be merged");

        let spawn = &spawn_results[0];
        assert!(spawn.vector_score.is_some());
        assert!(spawn.fts_score.is_some());
    }

    #[test]
    fn combined_ranking() {
        // An item appearing in both should rank higher than either alone
        let vec_results = vec![
            make_vector_result("v1", "both", 0.7),
            make_vector_result("v2", "vector_only", 0.7),
        ];
        let fts_results = vec![
            make_fts_result("f1", "both", 0.7),
            make_fts_result("f2", "fts_only", 0.7),
        ];

        let results = combine_results(&vec_results, &fts_results, 0.5, 10);

        let both = results.iter().find(|r| r.name == "both").unwrap();
        let vector_only = results.iter().find(|r| r.name == "vector_only").unwrap();
        let fts_only = results.iter().find(|r| r.name == "fts_only").unwrap();

        assert!(
            both.combined_score > vector_only.combined_score,
            "item in both should rank higher than vector-only"
        );
        assert!(
            both.combined_score > fts_only.combined_score,
            "item in both should rank higher than FTS-only"
        );
    }
}

//! Vector similarity search over DuckDB-stored embeddings.
//!
//! Queries `api_symbols` and `doc_chunks` tables using `array_cosine_similarity()`
//! with brute-force scan. Embeddings stored as `FLOAT[]` are cast to `FLOAT[384]`
//! at query time for the fixed-length array function.
//!
//! **Phase 4 only** — replaced by Lance vector search in Phase 8/9.

use zen_lake::ZenLake;

use crate::error::SearchError;

/// Convert a `duckdb::Error` into `SearchError` via `LakeError`.
const fn duck_err(e: duckdb::Error) -> SearchError {
    SearchError::Lake(zen_lake::LakeError::DuckDb(e))
}

/// Result from a vector similarity search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorSearchResult {
    /// Symbol or chunk ID.
    pub id: String,
    /// Package ecosystem.
    pub ecosystem: String,
    /// Package name.
    pub package: String,
    /// Package version.
    pub version: String,
    /// Symbol kind (for `api_symbols`) or "`doc_chunk`" (for `doc_chunks`).
    pub kind: String,
    /// Symbol name or chunk title.
    pub name: String,
    /// Signature (`api_symbols` only).
    pub signature: Option<String>,
    /// Doc comment or chunk content.
    pub doc_comment: Option<String>,
    /// File path (source file or doc file).
    pub file_path: String,
    /// Start line (`api_symbols` only).
    pub line_start: Option<i32>,
    /// End line (`api_symbols` only).
    pub line_end: Option<i32>,
    /// Cosine similarity score.
    pub score: f64,
    /// Which table the result came from.
    pub source_type: VectorSource,
}

/// Source table for a vector search result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum VectorSource {
    /// From the `api_symbols` table.
    ApiSymbol,
    /// From the `doc_chunks` table.
    DocChunk,
}

/// Filters for vector search queries.
#[derive(Debug, Clone)]
pub struct VectorSearchFilters {
    /// Filter to a specific package name.
    pub package: Option<String>,
    /// Filter to a specific ecosystem.
    pub ecosystem: Option<String>,
    /// Filter to a specific symbol kind (e.g., "function", "struct").
    pub kind: Option<String>,
    /// Maximum number of results to return.
    pub limit: u32,
    /// Minimum cosine similarity score (results below are excluded).
    pub min_score: f64,
}

impl Default for VectorSearchFilters {
    fn default() -> Self {
        Self {
            package: None,
            ecosystem: None,
            kind: None,
            limit: 20,
            min_score: 0.0,
        }
    }
}

/// Format a float slice as a `DuckDB` array literal: `[0.1, 0.2, ...]`.
fn vec_to_sql(v: &[f32]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(v.len() * 10 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        let _ = write!(s, "{x}");
    }
    s.push(']');
    s
}

/// Search `api_symbols` by vector similarity.
///
/// Embeds the query externally (via `EmbeddingEngine`), then uses brute-force
/// `array_cosine_similarity()` on `DuckDB` `FLOAT[]` columns cast to `FLOAT[384]`.
///
/// # Errors
///
/// Returns [`SearchError::Lake`] if the `DuckDB` query fails.
pub fn vector_search_symbols(
    lake: &ZenLake,
    query_embedding: &[f32],
    filters: &VectorSearchFilters,
) -> Result<Vec<VectorSearchResult>, SearchError> {
    let embedding_sql = vec_to_sql(query_embedding);

    let mut where_clauses = vec!["embedding IS NOT NULL".to_string()];
    let mut param_values: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

    if let Some(ref pkg) = filters.package {
        where_clauses.push("package = ?".to_string());
        param_values.push(Box::new(pkg.clone()));
    }
    if let Some(ref eco) = filters.ecosystem {
        where_clauses.push("ecosystem = ?".to_string());
        param_values.push(Box::new(eco.clone()));
    }
    if let Some(ref kind) = filters.kind {
        where_clauses.push("kind = ?".to_string());
        param_values.push(Box::new(kind.clone()));
    }

    let where_sql = where_clauses.join(" AND ");

    let sql = format!(
        "SELECT id, ecosystem, package, version, kind, name, signature, doc_comment,
                file_path, line_start, line_end,
                array_cosine_similarity(embedding::FLOAT[384], '{embedding_sql}'::FLOAT[384]) AS score
         FROM api_symbols
         WHERE {where_sql}
         ORDER BY score DESC
         LIMIT {limit}",
        limit = filters.limit,
    );

    let conn = lake.conn();
    let mut stmt = conn.prepare(&sql).map_err(duck_err)?;

    let param_refs: Vec<&dyn duckdb::ToSql> = param_values
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(VectorSearchResult {
                id: row.get(0)?,
                ecosystem: row.get(1)?,
                package: row.get(2)?,
                version: row.get(3)?,
                kind: row.get(4)?,
                name: row.get(5)?,
                signature: row.get(6)?,
                doc_comment: row.get(7)?,
                file_path: row.get(8)?,
                line_start: row.get(9)?,
                line_end: row.get(10)?,
                score: row.get(11)?,
                source_type: VectorSource::ApiSymbol,
            })
        })
        .map_err(duck_err)?;

    let mut results = Vec::new();
    for row in rows {
        let result = row.map_err(duck_err)?;
        if result.score >= filters.min_score {
            results.push(result);
        }
    }
    Ok(results)
}

/// Search `doc_chunks` by vector similarity.
///
/// Same approach as [`vector_search_symbols`] but against the `doc_chunks` table.
/// Chunk `title` maps to `name`, `content` maps to `doc_comment`, and
/// `source_file` maps to `file_path` in the result.
///
/// # Errors
///
/// Returns [`SearchError::Lake`] if the `DuckDB` query fails.
pub fn vector_search_doc_chunks(
    lake: &ZenLake,
    query_embedding: &[f32],
    filters: &VectorSearchFilters,
) -> Result<Vec<VectorSearchResult>, SearchError> {
    let embedding_sql = vec_to_sql(query_embedding);

    let mut where_clauses = vec!["embedding IS NOT NULL".to_string()];
    let mut param_values: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

    if let Some(ref pkg) = filters.package {
        where_clauses.push("package = ?".to_string());
        param_values.push(Box::new(pkg.clone()));
    }
    if let Some(ref eco) = filters.ecosystem {
        where_clauses.push("ecosystem = ?".to_string());
        param_values.push(Box::new(eco.clone()));
    }

    // kind filter is not applicable to doc_chunks (no kind column)

    let where_sql = where_clauses.join(" AND ");

    let sql = format!(
        "SELECT id, ecosystem, package, version, title, content, source_file,
                array_cosine_similarity(embedding::FLOAT[384], '{embedding_sql}'::FLOAT[384]) AS score
         FROM doc_chunks
         WHERE {where_sql}
         ORDER BY score DESC
         LIMIT {limit}",
        limit = filters.limit,
    );

    let conn = lake.conn();
    let mut stmt = conn.prepare(&sql).map_err(duck_err)?;

    let param_refs: Vec<&dyn duckdb::ToSql> = param_values
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let title: Option<String> = row.get(4)?;
            let content: Option<String> = row.get(5)?;
            let source_file: Option<String> = row.get(6)?;
            Ok(VectorSearchResult {
                id: row.get(0)?,
                ecosystem: row.get(1)?,
                package: row.get(2)?,
                version: row.get(3)?,
                kind: "doc_chunk".to_string(),
                name: title.unwrap_or_default(),
                signature: None,
                doc_comment: content,
                file_path: source_file.unwrap_or_default(),
                line_start: None,
                line_end: None,
                score: row.get(7)?,
                source_type: VectorSource::DocChunk,
            })
        })
        .map_err(duck_err)?;

    let mut results = Vec::new();
    for row in rows {
        let result = row.map_err(duck_err)?;
        if result.score >= filters.min_score {
            results.push(result);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic 384-dim embedding from a seed.
    fn synthetic_embedding(seed: u32) -> Vec<f32> {
        (0..384)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let base = (seed as f32) / 100.0;
                #[allow(clippy::cast_precision_loss)]
                let variation = (i as f32) / 384.0;
                (base + variation).sin()
            })
            .collect()
    }

    fn sample_symbol(
        id: &str,
        name: &str,
        kind: &str,
        package: &str,
        embedding: Vec<f32>,
    ) -> zen_lake::ApiSymbolRow {
        zen_lake::ApiSymbolRow {
            id: id.to_string(),
            ecosystem: "rust".to_string(),
            package: package.to_string(),
            version: "1.0.0".to_string(),
            file_path: "src/lib.rs".to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
            signature: Some(format!("pub fn {name}()")),
            source: None,
            doc_comment: Some(format!("Doc for {name}")),
            line_start: Some(1),
            line_end: Some(10),
            visibility: Some("public".to_string()),
            is_async: false,
            is_unsafe: false,
            is_error_type: false,
            returns_result: false,
            return_type: None,
            generics: None,
            attributes: None,
            metadata: None,
            embedding,
        }
    }

    fn sample_chunk(
        id: &str,
        index: i32,
        package: &str,
        embedding: Vec<f32>,
    ) -> zen_lake::DocChunkRow {
        zen_lake::DocChunkRow {
            id: id.to_string(),
            ecosystem: "rust".to_string(),
            package: package.to_string(),
            version: "1.0.0".to_string(),
            chunk_index: index,
            title: Some(format!("Section {index}")),
            content: format!("Content about topic {index}"),
            source_file: Some("README.md".to_string()),
            format: Some("md".to_string()),
            embedding,
        }
    }

    #[test]
    fn self_match_highest_score() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);
        lake.store_symbols(&[
            sample_symbol("s1", "spawn", "function", "tokio", emb.clone()),
            sample_symbol("s2", "select", "function", "tokio", synthetic_embedding(50)),
        ])
        .unwrap();

        let results = vector_search_symbols(&lake, &emb, &VectorSearchFilters::default()).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "s1", "self-match should be highest score");
        assert!(
            results[0].score > 0.99,
            "self-match score should be ~1.0, got {}",
            results[0].score
        );
    }

    #[test]
    fn ranking_by_cosine_similarity() {
        let lake = ZenLake::open_in_memory().unwrap();
        let query_emb = synthetic_embedding(1);
        lake.store_symbols(&[
            sample_symbol("s1", "close", "function", "tokio", synthetic_embedding(2)),
            sample_symbol("s2", "far", "function", "tokio", synthetic_embedding(100)),
        ])
        .unwrap();

        let results =
            vector_search_symbols(&lake, &query_emb, &VectorSearchFilters::default()).unwrap();

        assert!(results.len() >= 2);
        assert!(
            results[0].score >= results[1].score,
            "results should be ranked by descending score"
        );
    }

    #[test]
    fn package_filter() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);
        lake.store_symbols(&[
            sample_symbol("s1", "spawn", "function", "tokio", emb.clone()),
            sample_symbol("s2", "get", "function", "reqwest", synthetic_embedding(2)),
        ])
        .unwrap();

        let filters = VectorSearchFilters {
            package: Some("tokio".to_string()),
            ..Default::default()
        };
        let results = vector_search_symbols(&lake, &emb, &filters).unwrap();

        assert!(results.iter().all(|r| r.package == "tokio"));
    }

    #[test]
    fn kind_filter() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);
        lake.store_symbols(&[
            sample_symbol("s1", "Runtime", "struct", "tokio", emb.clone()),
            sample_symbol("s2", "spawn", "function", "tokio", synthetic_embedding(2)),
        ])
        .unwrap();

        let filters = VectorSearchFilters {
            kind: Some("struct".to_string()),
            ..Default::default()
        };
        let results = vector_search_symbols(&lake, &emb, &filters).unwrap();

        assert!(results.iter().all(|r| r.kind == "struct"));
    }

    #[test]
    fn min_score_filter() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);
        lake.store_symbols(&[
            sample_symbol("s1", "spawn", "function", "tokio", emb.clone()),
            sample_symbol("s2", "far", "function", "tokio", synthetic_embedding(200)),
        ])
        .unwrap();

        let filters = VectorSearchFilters {
            min_score: 0.99,
            ..Default::default()
        };
        let results = vector_search_symbols(&lake, &emb, &filters).unwrap();

        assert!(
            results.iter().all(|r| r.score >= 0.99),
            "all results should meet min_score"
        );
    }

    #[test]
    fn empty_lake_returns_empty() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);

        let results = vector_search_symbols(&lake, &emb, &VectorSearchFilters::default()).unwrap();
        assert!(results.is_empty());

        let results =
            vector_search_doc_chunks(&lake, &emb, &VectorSearchFilters::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn doc_chunk_search() {
        let lake = ZenLake::open_in_memory().unwrap();
        let emb = synthetic_embedding(1);
        lake.store_doc_chunks(&[
            sample_chunk("c1", 0, "tokio", emb.clone()),
            sample_chunk("c2", 1, "tokio", synthetic_embedding(50)),
        ])
        .unwrap();

        let results =
            vector_search_doc_chunks(&lake, &emb, &VectorSearchFilters::default()).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "c1");
        assert_eq!(results[0].source_type, VectorSource::DocChunk);
        assert_eq!(results[0].kind, "doc_chunk");
    }
}

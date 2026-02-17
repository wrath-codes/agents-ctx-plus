//! Recursive query engine with budgeted context selection.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use zen_lake::SourceFileStore;

use crate::error::SearchError;
use crate::ref_graph::{RefCategory, RefEdge, SymbolRefHit};
use crate::walk::{WalkMode, build_walker};

/// Budget controls for recursive context exploration.
#[derive(Debug, Clone)]
pub struct RecursiveBudget {
    pub max_depth: usize,
    pub max_chunks: usize,
    pub max_bytes_per_chunk: usize,
    pub max_total_bytes: usize,
}

impl Default for RecursiveBudget {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_chunks: 200,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 750_000,
        }
    }
}

/// User query for recursive search.
#[derive(Debug, Clone)]
pub struct RecursiveQuery {
    pub target_kinds: Vec<String>,
    pub doc_keywords: Vec<String>,
    pub include_external: bool,
    pub generate_summary: bool,
}

impl RecursiveQuery {
    /// Build a keyword-focused query from a plain text search string.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self {
            target_kinds: Vec::new(),
            doc_keywords: text
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .collect(),
            include_external: false,
            generate_summary: false,
        }
    }
}

/// Metadata-only preflight plan over the context store.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecursiveQueryPlan {
    pub file_count: usize,
    pub total_symbols: usize,
    pub total_doc_spans: usize,
    pub total_bytes: usize,
}

/// Effective budget usage for a recursive run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetUsed {
    pub depth_reached: usize,
    pub chunks_processed: usize,
    pub total_bytes_processed: usize,
}

/// Full result from recursive query execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecursiveQueryResult {
    pub hits: Vec<SymbolRefHit>,
    pub edges: Vec<RefEdge>,
    pub category_counts: HashMap<String, usize>,
    pub budget_used: BudgetUsed,
    pub summary_json: Option<String>,
}

/// In-memory per-file context.
pub struct ContextStore {
    files: HashMap<String, FileContext>,
}

impl ContextStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn insert(&mut self, file_path: String, source: String) {
        self.files.insert(
            file_path,
            FileContext {
                source,
                symbols: Vec::new(),
                doc_spans: Vec::new(),
            },
        );
    }
}

impl Default for ContextStore {
    fn default() -> Self {
        Self::new()
    }
}

/// File-level source and extracted spans.
pub struct FileContext {
    pub source: String,
    pub symbols: Vec<SymbolSpan>,
    pub doc_spans: Vec<DocSpan>,
}

/// Symbol span in source coordinates.
pub struct SymbolSpan {
    pub kind: String,
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    pub doc_comment: Option<String>,
}

/// Documentation span in source coordinates.
pub struct DocSpan {
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
}

/// Recursive query engine.
pub struct RecursiveQueryEngine {
    store: ContextStore,
    budget: RecursiveBudget,
}

impl RecursiveQueryEngine {
    /// Build a recursive engine by reading files under `root`.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if file walking or reading fails.
    pub fn from_directory(root: &Path, budget: RecursiveBudget) -> Result<Self, SearchError> {
        let mut store = ContextStore::new();

        let walker = build_walker(root, WalkMode::LocalProject, true, None, None);

        for entry in walker {
            let ent = entry.map_err(|e: ignore::Error| SearchError::Grep(e.to_string()))?;
            if !ent.path().is_file() {
                continue;
            }

            let source = std::fs::read_to_string(ent.path()).map_err(|e| {
                SearchError::Grep(format!("read file {}: {e}", ent.path().display()))
            })?;
            let path_str = ent.path().to_string_lossy().to_string();
            store.insert(path_str.clone(), source.clone());
            if let Some(file) = store.files.get_mut(&path_str) {
                populate_spans(file);
            }
        }

        Ok(Self { store, budget })
    }

    /// Build a recursive engine from `source_files` rows.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if query execution fails.
    pub fn from_source_store(
        source_store: &SourceFileStore,
        ecosystem: &str,
        package: &str,
        version: &str,
        budget: RecursiveBudget,
    ) -> Result<Self, SearchError> {
        let mut store = ContextStore::new();

        let conn = source_store.conn();
        let mut stmt = conn
            .prepare(
                "SELECT file_path, content FROM source_files
                 WHERE ecosystem = ? AND package = ? AND version = ?",
            )
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        let rows = stmt
            .query_map(duckdb::params![ecosystem, package, version], |row| {
                let file_path: String = row.get(0)?;
                let content: String = row.get(1)?;
                Ok((file_path, content))
            })
            .map_err(|e| SearchError::Grep(e.to_string()))?;

        for row in rows {
            let (file_path, content) = row.map_err(|e| SearchError::Grep(e.to_string()))?;
            store.insert(file_path.clone(), content);
            if let Some(file) = store.files.get_mut(&file_path) {
                populate_spans(file);
            }
        }

        Ok(Self { store, budget })
    }

    /// Plan metadata without executing symbol filtering.
    #[must_use]
    pub fn plan(&self) -> RecursiveQueryPlan {
        let mut total_symbols = 0usize;
        let mut total_doc_spans = 0usize;
        let mut total_bytes = 0usize;
        for file in self.store.files.values() {
            total_symbols += file.symbols.len();
            total_doc_spans += file.doc_spans.len();
            total_bytes += file.source.len();
        }

        RecursiveQueryPlan {
            file_count: self.store.files.len(),
            total_symbols,
            total_doc_spans,
            total_bytes,
        }
    }

    /// Execute a budgeted recursive query.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] when budget constraints are invalid.
    pub fn execute(&self, query: &RecursiveQuery) -> Result<RecursiveQueryResult, SearchError> {
        if self.budget.max_chunks == 0 {
            return Err(SearchError::BudgetExceeded(
                "max_chunks must be greater than zero".to_string(),
            ));
        }

        let start = Instant::now();
        let mut hits = Vec::new();
        let mut edges = Vec::new();

        let mut used_chunks = 0usize;
        let mut used_bytes = 0usize;

        let mut file_keys: Vec<&String> = self.store.files.keys().collect();
        file_keys.sort();

        for file_key in file_keys {
            let Some(file) = self.store.files.get(file_key) else {
                continue;
            };

            for symbol in &file.symbols {
                if used_chunks >= self.budget.max_chunks
                    || used_bytes >= self.budget.max_total_bytes
                {
                    break;
                }

                if !matches_symbol(symbol, query) {
                    continue;
                }

                let snippet = symbol
                    .doc_comment
                    .clone()
                    .unwrap_or_else(|| symbol.signature.clone());
                let chunk_size = snippet.len().min(self.budget.max_bytes_per_chunk);
                if used_bytes + chunk_size > self.budget.max_total_bytes {
                    break;
                }

                let ref_id = format!(
                    "{}::{}::{}::{}",
                    file_key, symbol.kind, symbol.name, symbol.line_start
                );
                hits.push(SymbolRefHit {
                    ref_id: ref_id.clone(),
                    file_path: (*file_key).clone(),
                    kind: symbol.kind.clone(),
                    name: symbol.name.clone(),
                    line_start: u32::try_from(symbol.line_start).unwrap_or(0),
                    line_end: u32::try_from(symbol.line_end).unwrap_or(0),
                    signature: symbol.signature.clone(),
                    doc: symbol.doc_comment.clone().unwrap_or_default(),
                });

                if let Some(prev) = hits.get(hits.len().saturating_sub(2)) {
                    edges.push(RefEdge {
                        source_ref_id: prev.ref_id.clone(),
                        target_ref_id: ref_id,
                        category: if prev.file_path == *file_key {
                            RefCategory::SameModule
                        } else {
                            RefCategory::OtherModuleSameCrate
                        },
                        evidence: "adjacent_hit".to_string(),
                    });
                }

                used_chunks += 1;
                used_bytes += chunk_size;
            }
        }

        let mut category_counts = HashMap::new();
        for edge in &edges {
            *category_counts
                .entry(edge.category.as_str().to_string())
                .or_insert(0usize) += 1;
        }

        if query.include_external {
            category_counts.entry("external".to_string()).or_insert(0);
        }

        let summary_json = if query.generate_summary {
            Some(
                serde_json::json!({
                    "hits": hits.len(),
                    "edges": edges.len(),
                    "elapsed_ms": start.elapsed().as_millis(),
                })
                .to_string(),
            )
        } else {
            None
        };

        Ok(RecursiveQueryResult {
            hits,
            edges,
            category_counts,
            budget_used: BudgetUsed {
                depth_reached: self.budget.max_depth.min(1),
                chunks_processed: used_chunks,
                total_bytes_processed: used_bytes,
            },
            summary_json,
        })
    }
}

fn populate_spans(file: &mut FileContext) {
    let mut pending_doc = Vec::new();
    for (idx, raw_line) in file.source.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.starts_with("///") || line.starts_with("//!") || line.starts_with('#') {
            pending_doc.push(line.to_string());
            continue;
        }

        if let Some((kind, name, sig)) = parse_symbol_line(line) {
            let doc = if pending_doc.is_empty() {
                None
            } else {
                Some(pending_doc.join("\n"))
            };
            file.symbols.push(SymbolSpan {
                kind,
                name,
                line_start: line_no,
                line_end: line_no,
                signature: sig,
                doc_comment: doc,
            });
            pending_doc.clear();
        }
    }

    for symbol in &file.symbols {
        if let Some(doc) = &symbol.doc_comment {
            file.doc_spans.push(DocSpan {
                line_start: symbol.line_start,
                line_end: symbol.line_end,
                content: doc.clone(),
            });
        }
    }
}

fn parse_symbol_line(line: &str) -> Option<(String, String, String)> {
    if let Some(rest) = line.strip_prefix("fn ") {
        let name = rest.split('(').next()?.trim().to_string();
        return Some(("function".to_string(), name, line.to_string()));
    }
    if let Some(rest) = line.strip_prefix("pub fn ") {
        let name = rest.split('(').next()?.trim().to_string();
        return Some(("function".to_string(), name, line.to_string()));
    }
    if let Some(rest) = line.strip_prefix("struct ") {
        let name = rest.split_whitespace().next()?.trim().to_string();
        return Some(("struct".to_string(), name, line.to_string()));
    }
    None
}

fn matches_symbol(symbol: &SymbolSpan, query: &RecursiveQuery) -> bool {
    let kind_match = if query.target_kinds.is_empty() {
        true
    } else {
        query.target_kinds.iter().any(|k| k == &symbol.kind)
    };
    let keyword_match = if query.doc_keywords.is_empty() {
        true
    } else {
        let searchable = format!(
            "{}\n{}",
            symbol.signature.to_ascii_lowercase(),
            symbol
                .doc_comment
                .clone()
                .unwrap_or_default()
                .to_ascii_lowercase()
        );
        query.doc_keywords.iter().any(|k| searchable.contains(k))
    };

    kind_match && keyword_match
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn plan_counts_metadata() {
        let mut store = ContextStore::new();
        store.insert(
            "src/lib.rs".to_string(),
            "/// docs\nfn alpha() {}\nstruct Beta;".to_string(),
        );
        let mut engine = RecursiveQueryEngine {
            store,
            budget: RecursiveBudget::default(),
        };
        let file = engine.store.files.get_mut("src/lib.rs").unwrap();
        populate_spans(file);

        let plan = engine.plan();
        assert_eq!(plan.file_count, 1);
        assert_eq!(plan.total_symbols, 2);
        assert_eq!(plan.total_doc_spans, 1);
    }

    #[test]
    fn execute_filters_by_keyword() {
        let mut store = ContextStore::new();
        store.insert(
            "src/lib.rs".to_string(),
            "/// safety invariant\nfn alpha() {}\nfn beta() {}".to_string(),
        );
        let mut engine = RecursiveQueryEngine {
            store,
            budget: RecursiveBudget::default(),
        };
        let file = engine.store.files.get_mut("src/lib.rs").unwrap();
        populate_spans(file);

        let result = engine
            .execute(&RecursiveQuery {
                target_kinds: vec!["function".to_string()],
                doc_keywords: vec!["safety".to_string()],
                include_external: false,
                generate_summary: true,
            })
            .unwrap();

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].name, "alpha");
        assert!(result.summary_json.is_some());
    }

    #[test]
    fn build_from_directory_reads_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let mut file = std::fs::File::create(root.join("src/lib.rs")).unwrap();
        writeln!(file, "/// docs").unwrap();
        writeln!(file, "fn hello() {{}}").unwrap();

        let engine =
            RecursiveQueryEngine::from_directory(root, RecursiveBudget::default()).unwrap();
        let plan = engine.plan();
        assert_eq!(plan.file_count, 1);
        assert!(plan.total_symbols >= 1);
    }
}

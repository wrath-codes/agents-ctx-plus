//! Reference graph types and transient `DuckDB` persistence.

use std::collections::HashMap;

use duckdb::params;

use crate::error::SearchError;

/// Symbol reference captured during recursive query execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRefHit {
    /// Stable ID (`file::kind::name::line`).
    pub ref_id: String,
    pub file_path: String,
    pub kind: String,
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub signature: String,
    pub doc: String,
}

/// Directed edge between symbol references.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefEdge {
    pub source_ref_id: String,
    pub target_ref_id: String,
    pub category: RefCategory,
    pub evidence: String,
}

/// Relationship category used in recursive output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RefCategory {
    SameModule,
    OtherModuleSameCrate,
    OtherCrateWorkspace,
    External,
}

impl RefCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SameModule => "same_module",
            Self::OtherModuleSameCrate => "other_module_same_crate",
            Self::OtherCrateWorkspace => "other_crate_workspace",
            Self::External => "external",
        }
    }
}

/// Transient DuckDB-backed reference graph.
pub struct ReferenceGraph {
    conn: duckdb::Connection,
}

const CREATE_REF_GRAPH: &str = "
CREATE TABLE symbol_refs (
    ref_id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    signature TEXT NOT NULL,
    doc TEXT NOT NULL
);

CREATE TABLE ref_edges (
    source_ref_id TEXT NOT NULL,
    target_ref_id TEXT NOT NULL,
    category TEXT NOT NULL,
    evidence TEXT NOT NULL,
    PRIMARY KEY(source_ref_id, target_ref_id)
);

CREATE INDEX ref_edges_category_idx ON ref_edges(category);
";

impl ReferenceGraph {
    /// Create an in-memory graph store.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if opening the in-memory database fails.
    pub fn new() -> Result<Self, SearchError> {
        let conn =
            duckdb::Connection::open_in_memory().map_err(|e| SearchError::Grep(e.to_string()))?;
        conn.execute_batch(CREATE_REF_GRAPH)
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        Ok(Self { conn })
    }

    /// Bulk insert refs and edges.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if an insert fails.
    pub fn insert(&self, refs: &[SymbolRefHit], edges: &[RefEdge]) -> Result<(), SearchError> {
        let mut stmt_ref = self
            .conn
            .prepare(
                "INSERT OR IGNORE INTO symbol_refs
                 (ref_id, file_path, kind, name, line_start, line_end, signature, doc)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(|e| SearchError::Grep(e.to_string()))?;

        for r in refs {
            stmt_ref
                .execute(params![
                    r.ref_id,
                    r.file_path,
                    r.kind,
                    r.name,
                    i64::from(r.line_start),
                    i64::from(r.line_end),
                    r.signature,
                    r.doc
                ])
                .map_err(|e| SearchError::Grep(e.to_string()))?;
        }

        let mut stmt_edge = self
            .conn
            .prepare(
                "INSERT OR IGNORE INTO ref_edges
                 (source_ref_id, target_ref_id, category, evidence)
                 VALUES (?, ?, ?, ?)",
            )
            .map_err(|e| SearchError::Grep(e.to_string()))?;

        for e in edges {
            stmt_edge
                .execute(params![
                    e.source_ref_id,
                    e.target_ref_id,
                    e.category.as_str(),
                    e.evidence
                ])
                .map_err(|e| SearchError::Grep(e.to_string()))?;
        }

        Ok(())
    }

    /// Count edges by category.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if query execution fails.
    pub fn category_counts(&self) -> Result<HashMap<String, usize>, SearchError> {
        let mut stmt = self
            .conn
            .prepare("SELECT category, COUNT(*) FROM ref_edges GROUP BY category")
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((key, usize::try_from(count).unwrap_or(0)))
            })
            .map_err(|e| SearchError::Grep(e.to_string()))?;

        let mut out = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| SearchError::Grep(e.to_string()))?;
            out.insert(k, v);
        }
        Ok(out)
    }

    /// Lookup a symbol signature by ref id.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError`] if query execution fails.
    pub fn lookup_signature(&self, ref_id: &str) -> Result<Option<String>, SearchError> {
        let mut stmt = self
            .conn
            .prepare("SELECT signature FROM symbol_refs WHERE ref_id = ?")
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        let mut rows = stmt
            .query([ref_id])
            .map_err(|e| SearchError::Grep(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| SearchError::Grep(e.to_string()))? {
            return row
                .get::<_, String>(0)
                .map(Some)
                .map_err(|e| SearchError::Grep(e.to_string()));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_counts_roundtrip() {
        let graph = ReferenceGraph::new().unwrap();
        let refs = vec![SymbolRefHit {
            ref_id: "a".to_string(),
            file_path: "src/lib.rs".to_string(),
            kind: "function".to_string(),
            name: "alpha".to_string(),
            line_start: 1,
            line_end: 10,
            signature: "fn alpha()".to_string(),
            doc: "alpha".to_string(),
        }];
        let edges = vec![RefEdge {
            source_ref_id: "a".to_string(),
            target_ref_id: "a".to_string(),
            category: RefCategory::SameModule,
            evidence: "self".to_string(),
        }];

        graph.insert(&refs, &edges).unwrap();
        let counts = graph.category_counts().unwrap();
        assert_eq!(counts.get("same_module"), Some(&1));
        assert_eq!(
            graph.lookup_signature("a").unwrap(),
            Some("fn alpha()".to_string())
        );
    }
}

//! # zen-lake
//!
//! `DuckDB` local cache storage for the Zenith documentation lake.
//!
//! Stores indexed package documentation: API symbols (from ast-grep extraction)
//! and doc chunks (from markdown parsing) with fastembed vector embeddings.
//!
//! ## Storage architecture
//!
//! Phase 3 uses **local-only `DuckDB`** as a temporary cache backend:
//! - `.zenith/lake/cache.duckdb` — `api_symbols`, `doc_chunks`, `indexed_packages`
//! - `.zenith/source_files.duckdb` — `source_files` (permanent, separate file)
//!
//! Production storage (Lance on R2 + Turso catalog) replaces the cache tables
//! in Phase 8/9. The `source_files` table is permanent and never shared.
//! See `23-phase3-parsing-indexing-plan.md` §13 for the replacement map.

pub mod error;
pub mod schemas;
pub mod source_files;
pub mod store;

pub use error::LakeError;
pub use schemas::{ApiSymbolRow, DocChunkRow};
pub use source_files::{SourceFile, SourceFileStore};

use duckdb::Connection;

/// Local `DuckDB` lake for indexed package data.
///
/// Manages the `api_symbols`, `doc_chunks`, and `indexed_packages` tables
/// in a single `DuckDB` file (`.zenith/lake/cache.duckdb`).
///
/// This is a **temporary local cache** — production storage is Lance on R2
/// with a Turso catalog (Phase 8/9).
pub struct ZenLake {
    conn: Connection,
}

impl ZenLake {
    /// Open or create a local `DuckDB` lake file.
    ///
    /// Creates all tables and indexes if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the file cannot be opened or schema creation fails.
    pub fn open_local(path: &str) -> Result<Self, LakeError> {
        let conn = Connection::open(path)?;
        let lake = Self { conn };
        lake.init_schema()?;
        Ok(lake)
    }

    /// Open an in-memory lake (for testing).
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if schema creation fails.
    pub fn open_in_memory() -> Result<Self, LakeError> {
        let conn = Connection::open_in_memory()?;
        let lake = Self { conn };
        lake.init_schema()?;
        Ok(lake)
    }

    /// Access the underlying `DuckDB` connection.
    ///
    /// Exposed for advanced queries (e.g., `array_cosine_similarity` searches).
    /// Prefer the typed methods on [`ZenLake`] for standard operations.
    #[must_use]
    pub const fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Initialize the lake schema (tables + indexes).
    fn init_schema(&self) -> Result<(), LakeError> {
        self.conn.execute_batch(schemas::CREATE_INDEXED_PACKAGES)?;
        self.conn.execute_batch(schemas::CREATE_API_SYMBOLS)?;
        self.conn.execute_batch(schemas::CREATE_DOC_CHUNKS)?;
        self.conn.execute_batch(schemas::CREATE_INDEXES)?;
        Ok(())
    }
}

#[cfg(test)]
mod spike_duckdb;

#[cfg(test)]
mod spike_duckdb_vss;

#[cfg(test)]
mod spike_r2_parquet;

#[cfg(test)]
mod spike_native_lance;

#[cfg(test)]
mod tests {
    use duckdb::params;

    use super::*;

    // ── Helpers ─────────────────────────────────────────────────────────

    fn sample_symbol(id: &str, name: &str, embedding: Vec<f32>) -> ApiSymbolRow {
        ApiSymbolRow {
            id: id.to_string(),
            ecosystem: "rust".to_string(),
            package: "tokio".to_string(),
            version: "1.49.0".to_string(),
            file_path: "src/runtime/mod.rs".to_string(),
            kind: "function".to_string(),
            name: name.to_string(),
            signature: Some(format!("pub async fn {name}()")),
            source: Some(format!("fn {name}() {{ todo!() }}")),
            doc_comment: Some(format!("Documentation for {name}.")),
            line_start: Some(1),
            line_end: Some(10),
            visibility: Some("public".to_string()),
            is_async: true,
            is_unsafe: false,
            is_error_type: false,
            returns_result: true,
            return_type: Some("Result<()>".to_string()),
            generics: None,
            attributes: Some(r##"["#[tokio::main]"]"##.to_string()),
            metadata: Some(r#"{"is_async": true}"#.to_string()),
            embedding,
        }
    }

    fn sample_chunk(id: &str, index: i32, embedding: Vec<f32>) -> DocChunkRow {
        DocChunkRow {
            id: id.to_string(),
            ecosystem: "rust".to_string(),
            package: "tokio".to_string(),
            version: "1.49.0".to_string(),
            chunk_index: index,
            title: Some(format!("Section {index}")),
            content: format!("Content of section {index} about async runtimes."),
            source_file: Some("README.md".to_string()),
            format: Some("md".to_string()),
            embedding,
        }
    }

    /// Deterministic 384-dim embedding from a seed (same as spike).
    fn synthetic_embedding(seed: u32) -> Vec<f32> {
        (0..384)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)] // Test helper; seeds are small
                let base = (seed as f32) / 100.0;
                let variation = (i as f32) / 384.0;
                (base + variation).sin()
            })
            .collect()
    }

    // ── ZenLake tests ──────────────────────────────────────────────────

    #[test]
    fn schema_creation() {
        let lake = ZenLake::open_in_memory().expect("open in-memory lake");

        // Verify tables exist by querying information_schema
        let tables: Vec<String> = {
            let mut stmt = lake
                .conn()
                .prepare(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'main'
                     ORDER BY table_name",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert!(tables.contains(&"api_symbols".to_string()));
        assert!(tables.contains(&"doc_chunks".to_string()));
        assert!(tables.contains(&"indexed_packages".to_string()));
        // source_files is NOT in this DB (it's a separate file)
        assert!(!tables.contains(&"source_files".to_string()));
    }

    #[test]
    fn store_and_query_symbols() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let symbols = vec![
            sample_symbol("sym-001", "spawn", synthetic_embedding(1)),
            sample_symbol("sym-002", "block_on", synthetic_embedding(2)),
            sample_symbol("sym-003", "select", synthetic_embedding(3)),
        ];

        lake.store_symbols(&symbols).expect("store symbols");

        // Query back
        let count: i64 = lake
            .conn()
            .query_row("SELECT count(*) FROM api_symbols", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);

        // Verify fields
        let (name, kind, is_async): (String, String, bool) = lake
            .conn()
            .query_row(
                "SELECT name, kind, is_async FROM api_symbols WHERE id = ?",
                params!["sym-001"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(name, "spawn");
        assert_eq!(kind, "function");
        assert!(is_async);
    }

    #[test]
    fn store_and_query_doc_chunks() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let chunks = vec![
            sample_chunk("chk-001", 0, synthetic_embedding(10)),
            sample_chunk("chk-002", 1, synthetic_embedding(11)),
        ];

        lake.store_doc_chunks(&chunks).expect("store chunks");

        let count: i64 = lake
            .conn()
            .query_row("SELECT count(*) FROM doc_chunks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let (title, content): (String, String) = lake
            .conn()
            .query_row(
                "SELECT title, content FROM doc_chunks WHERE id = ?",
                params!["chk-001"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(title, "Section 0");
        assert!(content.contains("async runtimes"));
    }

    #[test]
    fn register_and_check_package() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        assert!(!lake.is_package_indexed("rust", "tokio", "1.49.0").unwrap());

        lake.register_package(
            "rust",
            "tokio",
            "1.49.0",
            Some("https://github.com/tokio-rs/tokio"),
            Some("An async runtime"),
            Some("MIT"),
            Some(90_000_000),
            42,
            100,
            15,
        )
        .expect("register package");

        assert!(lake.is_package_indexed("rust", "tokio", "1.49.0").unwrap());
        assert!(!lake.is_package_indexed("rust", "tokio", "1.48.0").unwrap());
    }

    #[test]
    fn duplicate_package_replace() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        lake.register_package("rust", "tokio", "1.49.0", None, None, None, None, 10, 50, 5)
            .expect("first register");

        // Re-register with different counts
        lake.register_package(
            "rust", "tokio", "1.49.0", None, None, None, None, 20, 100, 10,
        )
        .expect("second register should replace");

        let symbol_count: i64 = lake
            .conn()
            .query_row(
                "SELECT symbol_count FROM indexed_packages
                 WHERE ecosystem = 'rust' AND package = 'tokio' AND version = '1.49.0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(symbol_count, 100, "should have replaced with new count");
    }

    #[test]
    fn embedding_roundtrip() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let emb = synthetic_embedding(42);
        let symbols = vec![sample_symbol("sym-emb", "test_fn", emb.clone())];
        lake.store_symbols(&symbols).expect("store");

        // Verify embedding has 384 dims
        let dims: i64 = lake
            .conn()
            .query_row(
                "SELECT array_length(embedding) FROM api_symbols WHERE id = 'sym-emb'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dims, 384);
    }

    #[test]
    fn cosine_similarity_query() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let emb1 = synthetic_embedding(1);
        let emb2 = synthetic_embedding(2);
        let symbols = vec![
            sample_symbol("sym-a", "func_a", emb1),
            sample_symbol("sym-b", "func_b", emb2),
        ];
        lake.store_symbols(&symbols).expect("store");

        // Query cosine similarity between the two
        let similarity: f64 = lake
            .conn()
            .query_row(
                "SELECT array_cosine_similarity(
                    (SELECT embedding::FLOAT[384] FROM api_symbols WHERE id = 'sym-a'),
                    (SELECT embedding::FLOAT[384] FROM api_symbols WHERE id = 'sym-b')
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Both are similar (close seeds) so similarity should be > 0
        assert!(
            similarity > 0.0,
            "similar embeddings should have positive cosine similarity, got {similarity}"
        );
    }

    #[test]
    fn delete_package() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let symbols = vec![sample_symbol(
            "sym-del",
            "to_delete",
            synthetic_embedding(1),
        )];
        lake.store_symbols(&symbols).expect("store symbols");
        lake.register_package("rust", "tokio", "1.49.0", None, None, None, None, 1, 1, 0)
            .expect("register");

        assert!(lake.is_package_indexed("rust", "tokio", "1.49.0").unwrap());

        lake.delete_package("rust", "tokio", "1.49.0")
            .expect("delete");

        assert!(!lake.is_package_indexed("rust", "tokio", "1.49.0").unwrap());

        let count: i64 = lake
            .conn()
            .query_row("SELECT count(*) FROM api_symbols", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn list_and_count_indexed_packages() {
        let lake = ZenLake::open_in_memory().expect("open lake");
        assert_eq!(lake.count_indexed_packages().unwrap(), 0);

        lake.register_package("rust", "tokio", "1.49.0", None, None, None, None, 0, 0, 0)
            .unwrap();
        lake.register_package("rust", "serde", "1.0.0", None, None, None, None, 0, 0, 0)
            .unwrap();

        let packages = lake.list_indexed_packages().unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(lake.count_indexed_packages().unwrap(), 2);
        assert_eq!(
            packages[0],
            ("rust".to_string(), "serde".to_string(), "1.0.0".to_string())
        );
        assert_eq!(
            packages[1],
            (
                "rust".to_string(),
                "tokio".to_string(),
                "1.49.0".to_string()
            )
        );
    }

    #[test]
    fn clear_removes_all_lake_tables() {
        let lake = ZenLake::open_in_memory().expect("open lake");
        lake.store_symbols(&[sample_symbol(
            "sym-clear",
            "to_clear",
            synthetic_embedding(1),
        )])
        .unwrap();
        lake.store_doc_chunks(&[sample_chunk("chk-clear", 0, synthetic_embedding(2))])
            .unwrap();
        lake.register_package("rust", "tokio", "1.49.0", None, None, None, None, 1, 1, 1)
            .unwrap();

        lake.clear().unwrap();

        let symbols: i64 = lake
            .conn()
            .query_row("SELECT COUNT(*) FROM api_symbols", [], |row| row.get(0))
            .unwrap();
        let chunks: i64 = lake
            .conn()
            .query_row("SELECT COUNT(*) FROM doc_chunks", [], |row| row.get(0))
            .unwrap();
        let packages: i64 = lake
            .conn()
            .query_row("SELECT COUNT(*) FROM indexed_packages", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(symbols, 0);
        assert_eq!(chunks, 0);
        assert_eq!(packages, 0);
    }

    #[test]
    fn file_persistence() {
        let tmpdir = tempfile::tempdir().unwrap();
        let db_path = tmpdir.path().join("test_lake.duckdb");
        let db_str = db_path.to_str().unwrap();

        // Write data
        {
            let lake = ZenLake::open_local(db_str).expect("open file-backed lake");
            lake.store_symbols(&[sample_symbol("sym-p", "persist", synthetic_embedding(1))])
                .expect("store");
            lake.register_package("rust", "test", "0.1.0", None, None, None, None, 1, 1, 0)
                .expect("register");
        }

        // Reopen and verify
        {
            let lake = ZenLake::open_local(db_str).expect("reopen lake");
            assert!(lake.is_package_indexed("rust", "test", "0.1.0").unwrap());

            let name: String = lake
                .conn()
                .query_row(
                    "SELECT name FROM api_symbols WHERE id = 'sym-p'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(name, "persist");
        }
    }

    #[test]
    fn index_existence() {
        let lake = ZenLake::open_in_memory().expect("open lake");

        let indexes: Vec<String> = {
            let mut stmt = lake
                .conn()
                .prepare(
                    "SELECT index_name FROM duckdb_indexes()
                     WHERE table_name = 'api_symbols'
                     ORDER BY index_name",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert!(
            indexes.contains(&"idx_symbols_file_lines".to_string()),
            "idx_symbols_file_lines should exist, got: {indexes:?}"
        );
        assert!(
            indexes.contains(&"idx_symbols_pkg".to_string()),
            "idx_symbols_pkg should exist"
        );
        assert!(
            indexes.contains(&"idx_symbols_name".to_string()),
            "idx_symbols_name should exist"
        );
    }

    // ── SourceFileStore tests ──────────────────────────────────────────

    #[test]
    fn source_file_store_schema() {
        let store = SourceFileStore::open_in_memory().expect("open source store");

        let tables: Vec<String> = {
            let mut stmt = store
                .conn()
                .prepare(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'main'",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert!(tables.contains(&"source_files".to_string()));
    }

    #[test]
    fn store_and_query_source_files() {
        let store = SourceFileStore::open_in_memory().expect("open source store");

        let files = vec![
            SourceFile {
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.49.0".to_string(),
                file_path: "src/lib.rs".to_string(),
                content: "pub fn main() {}".to_string(),
                language: Some("rust".to_string()),
                size_bytes: 17,
                line_count: 1,
            },
            SourceFile {
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.49.0".to_string(),
                file_path: "src/runtime.rs".to_string(),
                content: "pub struct Runtime;".to_string(),
                language: Some("rust".to_string()),
                size_bytes: 20,
                line_count: 1,
            },
        ];

        store
            .store_source_files(&files)
            .expect("store source files");

        let count: i64 = store
            .conn()
            .query_row("SELECT count(*) FROM source_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let content: String = store
            .conn()
            .query_row(
                "SELECT content FROM source_files WHERE file_path = 'src/lib.rs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(content, "pub fn main() {}");
    }

    #[test]
    fn source_store_separate_from_lake() {
        // Verify ZenLake and SourceFileStore are independent
        let lake = ZenLake::open_in_memory().expect("lake");
        let store = SourceFileStore::open_in_memory().expect("source store");

        // Lake should not have source_files
        let lake_tables: Vec<String> = {
            let mut stmt = lake
                .conn()
                .prepare(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'main' ORDER BY table_name",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert!(!lake_tables.contains(&"source_files".to_string()));

        // Source store should not have api_symbols
        let store_tables: Vec<String> = {
            let mut stmt = store
                .conn()
                .prepare(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'main' ORDER BY table_name",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert!(!store_tables.contains(&"api_symbols".to_string()));
    }

    #[test]
    fn delete_package_sources() {
        let store = SourceFileStore::open_in_memory().expect("open");
        let files = vec![SourceFile {
            ecosystem: "rust".to_string(),
            package: "tokio".to_string(),
            version: "1.49.0".to_string(),
            file_path: "src/lib.rs".to_string(),
            content: "fn main() {}".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 13,
            line_count: 1,
        }];
        store.store_source_files(&files).unwrap();

        store
            .delete_package_sources("rust", "tokio", "1.49.0")
            .unwrap();

        let count: i64 = store
            .conn()
            .query_row("SELECT count(*) FROM source_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn clear_source_file_store() {
        let store = SourceFileStore::open_in_memory().expect("open");
        store
            .store_source_files(&[SourceFile {
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.49.0".to_string(),
                file_path: "src/lib.rs".to_string(),
                content: "fn main() {}".to_string(),
                language: Some("rust".to_string()),
                size_bytes: 12,
                line_count: 1,
            }])
            .unwrap();

        store.clear().unwrap();

        let count: i64 = store
            .conn()
            .query_row("SELECT count(*) FROM source_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}

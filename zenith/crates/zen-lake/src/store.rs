//! Store methods for the local `DuckDB` lake cache.
//!
//! Provides bulk insertion for API symbols, doc chunks, and package registration.
//! Uses parameterized `INSERT` statements with `FLOAT[]` cast for embedding columns.
//!
//! **Note**: `DuckDB`'s Rust `Appender` API does not reliably handle `FLOAT[]` array
//! columns from `Vec<f32>`. Parameterized INSERT with string-serialized arrays and
//! `::FLOAT[]` cast is the validated approach (spike 0.4). For tables without
//! embeddings (e.g., `source_files`), the Appender is used — see [`super::source_files`].

use duckdb::params;

use crate::schemas::{ApiSymbolRow, DocChunkRow};
use crate::{LakeError, ZenLake};

/// Format a `Vec<f32>` as a `DuckDB` array literal string: `[0.1, 0.2, ...]`.
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

impl ZenLake {
    /// Store API symbols in the local `DuckDB` cache.
    ///
    /// Inserts symbols with their embeddings using parameterized INSERT statements.
    /// Embeddings are stored as `FLOAT[]` and cast to `FLOAT[384]` at query time.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if any INSERT fails.
    pub fn store_symbols(&self, symbols: &[ApiSymbolRow]) -> Result<(), LakeError> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO api_symbols (
                id, ecosystem, package, version, file_path, kind, name,
                signature, source, doc_comment, line_start, line_end,
                visibility, is_async, is_unsafe, is_error_type, returns_result,
                return_type, generics, attributes, metadata, embedding
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?::FLOAT[]
            )",
        )?;

        for sym in symbols {
            let embedding_sql = if sym.embedding.is_empty() {
                "NULL".to_string()
            } else {
                vec_to_sql(&sym.embedding)
            };

            stmt.execute(params![
                sym.id,
                sym.ecosystem,
                sym.package,
                sym.version,
                sym.file_path,
                sym.kind,
                sym.name,
                sym.signature,
                sym.source,
                sym.doc_comment,
                sym.line_start,
                sym.line_end,
                sym.visibility,
                sym.is_async,
                sym.is_unsafe,
                sym.is_error_type,
                sym.returns_result,
                sym.return_type,
                sym.generics,
                sym.attributes,
                sym.metadata,
                embedding_sql,
            ])?;
        }

        Ok(())
    }

    /// Store doc chunks in the local `DuckDB` cache.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if any INSERT fails.
    pub fn store_doc_chunks(&self, chunks: &[DocChunkRow]) -> Result<(), LakeError> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO doc_chunks (
                id, ecosystem, package, version, chunk_index,
                title, content, source_file, format, embedding
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?::FLOAT[])",
        )?;

        for chunk in chunks {
            let embedding_sql = if chunk.embedding.is_empty() {
                "NULL".to_string()
            } else {
                vec_to_sql(&chunk.embedding)
            };

            stmt.execute(params![
                chunk.id,
                chunk.ecosystem,
                chunk.package,
                chunk.version,
                chunk.chunk_index,
                chunk.title,
                chunk.content,
                chunk.source_file,
                chunk.format,
                embedding_sql,
            ])?;
        }

        Ok(())
    }

    /// Register a package as indexed in the local cache.
    ///
    /// Uses `INSERT OR REPLACE` so re-indexing the same package overwrites the entry.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the INSERT fails.
    #[allow(clippy::too_many_arguments)]
    pub fn register_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
        repo_url: Option<&str>,
        description: Option<&str>,
        license: Option<&str>,
        downloads: Option<i64>,
        file_count: i32,
        symbol_count: i32,
        doc_chunk_count: i32,
    ) -> Result<(), LakeError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO indexed_packages
             (ecosystem, package, version, repo_url, description, license, downloads,
              file_count, symbol_count, doc_chunk_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                ecosystem,
                package,
                version,
                repo_url,
                description,
                license,
                downloads,
                file_count,
                symbol_count,
                doc_chunk_count
            ],
        )?;
        Ok(())
    }

    /// Check if a package is already indexed in the local cache.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the query fails.
    pub fn is_package_indexed(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<bool, LakeError> {
        let mut stmt = self.conn.prepare(
            "SELECT 1 FROM indexed_packages
             WHERE ecosystem = ? AND package = ? AND version = ?",
        )?;
        let exists = stmt
            .query_row(params![ecosystem, package, version], |_| Ok(true))
            .unwrap_or(false);
        Ok(exists)
    }

    /// Delete all data for a specific package version from the local cache.
    ///
    /// Removes symbols, doc chunks, and the package registration entry.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if any DELETE fails.
    pub fn delete_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<(), LakeError> {
        self.conn.execute(
            "DELETE FROM api_symbols WHERE ecosystem = ? AND package = ? AND version = ?",
            params![ecosystem, package, version],
        )?;
        self.conn.execute(
            "DELETE FROM doc_chunks WHERE ecosystem = ? AND package = ? AND version = ?",
            params![ecosystem, package, version],
        )?;
        self.conn.execute(
            "DELETE FROM indexed_packages WHERE ecosystem = ? AND package = ? AND version = ?",
            params![ecosystem, package, version],
        )?;
        Ok(())
    }
}

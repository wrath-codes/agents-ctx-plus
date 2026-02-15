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
    /// ID generation: If `sym.id` is non-empty, it is used as-is. If empty, the ID
    /// is generated server-side as `substr(md5(concat(ecosystem, ':', package, ':',
    /// version, ':', file_path, ':', kind, ':', name)), 1, 16)` — a deterministic
    /// 16-character hex hash. This avoids a Rust-side hashing dependency.
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
                COALESCE(
                    NULLIF(?, ''),
                    substr(md5(concat(?, ':', ?, ':', ?, ':', ?, ':', ?, ':', ?)), 1, 16)
                ),
                ?, ?, ?, ?, ?, ?,
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
                // For ID: first is candidate, next 6 are md5 inputs (used only if candidate is empty)
                sym.id,        // 1: candidate id
                sym.ecosystem, // 2: md5 input
                sym.package,   // 3
                sym.version,   // 4
                sym.file_path, // 5
                sym.kind,      // 6
                sym.name,      // 7
                sym.ecosystem,      // 8
                sym.package,        // 9
                sym.version,        // 10
                sym.file_path,      // 11
                sym.kind,           // 12
                sym.name,           // 13
                sym.signature,      // 14
                sym.source,         // 15
                sym.doc_comment,    // 16
                sym.line_start,     // 17
                sym.line_end,       // 18
                sym.visibility,     // 19
                sym.is_async,       // 20
                sym.is_unsafe,      // 21
                sym.is_error_type,  // 22
                sym.returns_result, // 23
                sym.return_type,    // 24
                sym.generics,       // 25
                sym.attributes,     // 26
                sym.metadata,       // 27
                embedding_sql,      // 28
            ])?;
        }

        Ok(())
    }

    /// Store doc chunks in the local `DuckDB` cache.
    ///
    /// ID generation: If `chunk.id` is non-empty, it is used as-is. If empty, the ID
    /// is generated server-side as `substr(md5(concat(ecosystem, ':', package, ':',
    /// version, ':', source_file, ':', chunk_index)), 1, 16)`.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if any INSERT fails.
    pub fn store_doc_chunks(&self, chunks: &[DocChunkRow]) -> Result<(), LakeError> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO doc_chunks (
                id, ecosystem, package, version, chunk_index,
                title, content, source_file, format, embedding
            ) VALUES (
                COALESCE(
                    NULLIF(?, ''),
                    substr(md5(concat(?, ':', ?, ':', ?, ':', ?, ':', ?)), 1, 16)
                ),
                ?, ?, ?, ?, ?, ?, ?, ?, ?::FLOAT[]
            )",
        )?;

        for chunk in chunks {
            let embedding_sql = if chunk.embedding.is_empty() {
                "NULL".to_string()
            } else {
                vec_to_sql(&chunk.embedding)
            };

            stmt.execute(params![
                // For ID: candidate then md5 inputs
                chunk.id,          // 1: candidate id
                chunk.ecosystem,   // 2
                chunk.package,     // 3
                chunk.version,     // 4
                chunk.source_file, // 5
                chunk.chunk_index, // 6
                // Remaining columns: ecosystem, package, version, chunk_index, title, content, source_file, format, embedding
                chunk.ecosystem,   // 7
                chunk.package,     // 8
                chunk.version,     // 9
                chunk.chunk_index, // 10
                chunk.title,       // 11
                chunk.content,     // 12
                chunk.source_file, // 13
                chunk.format,      // 14
                embedding_sql,     // 15
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

    /// Mark the source files as cached for a package in `indexed_packages`.
    ///
    /// Sets `source_cached = TRUE` for the given package. Called after the
    /// indexing pipeline stores source files to track cache state (see spike 0.14).
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the UPDATE fails.
    pub fn set_source_cached(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<(), LakeError> {
        self.conn.execute(
            "UPDATE indexed_packages SET source_cached = TRUE
             WHERE ecosystem = ? AND package = ? AND version = ?",
            params![ecosystem, package, version],
        )?;
        Ok(())
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

//! Source file caching in a separate `DuckDB` file.
//!
//! Per `02-data-architecture.md` §11, source files live in a **separate** `DuckDB`
//! file (`.zenith/source_files.duckdb`), not in the lake cache. They are large,
//! not shared across users, and don't need vector search. This is a **permanent**
//! local store — not replaced in Phase 8/9.
//!
//! Used by `znt grep` (Phase 4) to search source code content with Rust regex.

use duckdb::{Connection, params};

use crate::LakeError;

/// DDL for the source files table.
const CREATE_SOURCE_FILES: &str = "
CREATE TABLE IF NOT EXISTS source_files (
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT,
    size_bytes INTEGER,
    line_count INTEGER,
    PRIMARY KEY (ecosystem, package, version, file_path)
);
CREATE INDEX IF NOT EXISTS idx_source_pkg
    ON source_files(ecosystem, package, version);
CREATE INDEX IF NOT EXISTS idx_source_lang
    ON source_files(ecosystem, package, version, language);
";

/// A source file row for insertion.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Package ecosystem.
    pub ecosystem: String,
    /// Package name.
    pub package: String,
    /// Package version.
    pub version: String,
    /// Relative file path within the repo.
    pub file_path: String,
    /// Full file content (UTF-8 text).
    pub content: String,
    /// Detected language (lowercase, e.g., "rust", "python").
    pub language: Option<String>,
    /// File size in bytes.
    pub size_bytes: i32,
    /// Number of lines in the file.
    pub line_count: i32,
}

/// Manages source file storage in a separate `DuckDB` file.
///
/// This is independent from [`super::ZenLake`] — they use different `DuckDB` files.
/// Source files are large and never shared, so they stay local permanently.
pub struct SourceFileStore {
    conn: Connection,
}

impl SourceFileStore {
    /// Open or create the source files `DuckDB` at the given path.
    ///
    /// Typically `.zenith/source_files.duckdb`.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the file cannot be opened or schema creation fails.
    pub fn open(path: &str) -> Result<Self, LakeError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(CREATE_SOURCE_FILES)?;
        Ok(Self { conn })
    }

    /// Open an in-memory source file store (for testing).
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if schema creation fails.
    pub fn open_in_memory() -> Result<Self, LakeError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(CREATE_SOURCE_FILES)?;
        Ok(Self { conn })
    }

    /// Store source files using the `DuckDB` Appender for bulk insert.
    ///
    /// Called during the indexing pipeline. Source content is already in memory
    /// from the parsing step — zero extra I/O.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the Appender fails.
    pub fn store_source_files(&self, files: &[SourceFile]) -> Result<(), LakeError> {
        let mut appender = self.conn.appender("source_files")?;
        for f in files {
            appender.append_row(params![
                f.ecosystem,
                f.package,
                f.version,
                f.file_path,
                f.content,
                f.language,
                f.size_bytes,
                f.line_count
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    /// Delete all source files for a specific package version.
    ///
    /// # Errors
    ///
    /// Returns [`LakeError::DuckDb`] if the DELETE fails.
    pub fn delete_package_sources(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<(), LakeError> {
        self.conn.execute(
            "DELETE FROM source_files WHERE ecosystem = ? AND package = ? AND version = ?",
            params![ecosystem, package, version],
        )?;
        Ok(())
    }

    /// Access the underlying `DuckDB` connection.
    #[must_use]
    pub const fn conn(&self) -> &Connection {
        &self.conn
    }
}

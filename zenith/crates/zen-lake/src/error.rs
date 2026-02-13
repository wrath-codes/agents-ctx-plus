//! Lake error types.

/// Errors that can occur in the documentation lake storage layer.
#[derive(Debug, thiserror::Error)]
pub enum LakeError {
    /// `DuckDB` operation failed.
    #[error("DuckDB error: {0}")]
    DuckDb(#[from] duckdb::Error),

    /// Lake database has not been initialized (schema not created).
    #[error("Lake not initialized: {0}")]
    NotInitialized(String),

    /// Requested package was not found in the index.
    #[error("Package not found: {ecosystem}/{package}/{version}")]
    PackageNotFound {
        /// Package ecosystem (e.g., "rust", "npm").
        ecosystem: String,
        /// Package name.
        package: String,
        /// Package version.
        version: String,
    },

    /// I/O error (file operations on `DuckDB` files).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for other errors.
    #[error("{0}")]
    Other(String),
}

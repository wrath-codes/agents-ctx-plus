//! Database error types for zen-db.

use thiserror::Error;

/// Errors from database operations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// A SQL query failed.
    #[error("Query failed: {0}")]
    Query(String),

    /// Schema migration failed.
    #[error("Migration failed: {0}")]
    Migration(String),

    /// Expected a result row but none was returned.
    #[error("No result returned")]
    NoResult,

    /// Invalid state encountered (e.g., bad data in DB).
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Underlying libSQL error.
    #[error("libSQL error: {0}")]
    LibSql(#[from] libsql::Error),

    /// Catch-all for unexpected errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

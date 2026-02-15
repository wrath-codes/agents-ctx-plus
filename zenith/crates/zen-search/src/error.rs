//! Search error types for zen-search.

/// Errors from search operations across vector, FTS, and grep backends.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// Error from the `DuckDB` lake storage layer.
    #[error("lake error: {0}")]
    Lake(#[from] zen_lake::LakeError),

    /// Error from the libSQL database (FTS5 queries).
    #[error("database error: {0}")]
    Database(#[from] zen_db::error::DatabaseError),

    /// Error from the embedding engine (fastembed/ONNX).
    #[error("embedding error: {0}")]
    Embedding(#[from] zen_embeddings::EmbeddingError),

    /// Error from grep search operations.
    #[error("grep error: {0}")]
    Grep(String),

    /// Error from registry operations.
    #[error("registry error: {0}")]
    Registry(String),

    /// Invalid or empty search query.
    #[error("invalid query: {0}")]
    InvalidQuery(String),

    /// Search returned no results.
    #[error("no results found")]
    NoResults,

    /// Search budget (time or result count) exceeded.
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}

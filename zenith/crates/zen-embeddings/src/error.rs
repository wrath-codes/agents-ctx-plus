//! Embedding error types.

/// Errors that can occur during embedding generation.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// Model initialization failed (download, ONNX runtime, cache issues).
    #[error("Model initialization failed: {0}")]
    InitFailed(String),

    /// Embedding generation failed (inference error, invalid input).
    #[error("Embedding generation failed: {0}")]
    EmbedFailed(String),

    /// Model returned zero embeddings for a non-empty input.
    #[error("Empty result from embedding model")]
    EmptyResult,
}

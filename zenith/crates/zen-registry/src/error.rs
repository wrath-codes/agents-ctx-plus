//! Registry error types.

use thiserror::Error;

/// Errors that can occur when interacting with package registries.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Registry API returned a non-success status code.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code returned by the registry.
        status: u16,
        /// Error message or response body.
        message: String,
    },

    /// Failed to parse a registry response.
    #[error("parse error: {0}")]
    Parse(String),

    /// The requested ecosystem is not supported.
    #[error("unsupported ecosystem: {0}")]
    UnsupportedEcosystem(String),

    /// The registry returned a 429 Too Many Requests response.
    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },
}

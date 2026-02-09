//! Schema validation error types.

use thiserror::Error;

/// Errors from the schema registry.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Requested schema name was not found in the registry.
    #[error("Schema not found: {0}")]
    NotFound(String),

    /// JSON value did not pass schema validation.
    #[error("Validation failed: {errors:?}")]
    ValidationFailed {
        /// Individual error messages from the validator.
        errors: Vec<String>,
    },

    /// Schema generation or compilation error.
    #[error("Schema generation error: {0}")]
    Generation(String),
}

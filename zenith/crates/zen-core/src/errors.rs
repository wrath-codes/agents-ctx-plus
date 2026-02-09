//! Cross-cutting error types for Zenith.
//!
//! This module defines errors that can originate from any crate in the system.
//! Domain-specific errors (e.g., `DatabaseError`, `LakeError`) are defined in
//! their respective crates. A unified `ZenError` is deferred to `zen-cli` where
//! all crate errors converge.

use thiserror::Error;

/// Errors that can be raised by any Zenith crate.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Entity lookup returned no result.
    #[error("Entity not found: {entity_type} {id}")]
    NotFound { entity_type: String, id: String },

    /// A state machine transition was attempted that is not allowed.
    #[error("Invalid state transition: {entity_type} {id} from {from} to {to}")]
    InvalidTransition {
        entity_type: String,
        id: String,
        from: String,
        to: String,
    },

    /// Data failed validation (schema, format, constraints).
    #[error("Validation error: {0}")]
    Validation(String),

    /// Catch-all for unexpected errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

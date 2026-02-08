//! Configuration error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    /// Figment extraction or merge error.
    #[error("Configuration error: {0}")]
    Figment(#[from] figment::Error),

    /// A required configuration section is not configured.
    #[error("Configuration section '{section}' is not configured (missing required fields)")]
    NotConfigured { section: String },

    /// A configuration field has an invalid value.
    #[error("Invalid configuration value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },
}

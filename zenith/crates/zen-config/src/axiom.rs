//! Axiom telemetry configuration.

use serde::{Deserialize, Serialize};

/// Default Axiom API endpoint.
fn default_endpoint() -> String {
    String::from("https://api.axiom.co")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AxiomConfig {
    /// Axiom API token (must start with `xaat-`).
    #[serde(default)]
    pub token: String,

    /// Axiom dataset name.
    #[serde(default)]
    pub dataset: String,

    /// Axiom API endpoint (also used as OTEL exporter endpoint).
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
}

impl Default for AxiomConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            dataset: String::new(),
            endpoint: default_endpoint(),
        }
    }
}

impl AxiomConfig {
    /// Check if the Axiom config has the minimum required fields.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.token.is_empty() && !self.dataset.is_empty() && self.is_valid_token()
    }

    /// Axiom tokens must start with `xaat-`.
    #[must_use]
    pub fn is_valid_token(&self) -> bool {
        self.token.starts_with("xaat-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_configured() {
        let config = AxiomConfig::default();
        assert!(!config.is_configured());
        assert_eq!(config.endpoint, "https://api.axiom.co");
    }

    #[test]
    fn configured_with_valid_token() {
        let config = AxiomConfig {
            token: "xaat-abc123".into(),
            dataset: "traces".into(),
            endpoint: default_endpoint(),
        };
        assert!(config.is_valid_token());
        assert!(config.is_configured());
    }

    #[test]
    fn invalid_token_rejected() {
        let config = AxiomConfig {
            token: "invalid-token".into(),
            dataset: "traces".into(),
            endpoint: default_endpoint(),
        };
        assert!(!config.is_valid_token());
        assert!(!config.is_configured());
    }

    #[test]
    fn not_configured_without_dataset() {
        let config = AxiomConfig {
            token: "xaat-abc123".into(),
            ..Default::default()
        };
        assert!(!config.is_configured());
    }
}

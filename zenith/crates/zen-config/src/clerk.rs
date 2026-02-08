//! Clerk authentication configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClerkConfig {
    /// Clerk publishable key.
    #[serde(default)]
    pub publishable_key: String,

    /// Clerk secret key.
    #[serde(default)]
    pub secret_key: String,

    /// JWKS URL for token verification.
    #[serde(default)]
    pub jwks_url: String,

    /// Backend API URL.
    #[serde(default)]
    pub backend_url: String,

    /// Frontend app URL.
    #[serde(default)]
    pub frontend_url: String,
}

impl ClerkConfig {
    /// Check if the Clerk config has the minimum required fields.
    pub fn is_configured(&self) -> bool {
        !self.publishable_key.is_empty() && !self.secret_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_configured() {
        let config = ClerkConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn configured_when_keys_set() {
        let config = ClerkConfig {
            publishable_key: "pk_test_123".into(),
            secret_key: "sk_test_456".into(),
            ..Default::default()
        };
        assert!(config.is_configured());
    }

    #[test]
    fn not_configured_when_missing_secret() {
        let config = ClerkConfig {
            publishable_key: "pk_test_123".into(),
            ..Default::default()
        };
        assert!(!config.is_configured());
    }
}

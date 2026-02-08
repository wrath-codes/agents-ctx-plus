//! Turso/libSQL configuration.

use serde::{Deserialize, Serialize};

/// Default sync interval in seconds.
const fn default_sync_interval_secs() -> u64 {
    60
}

/// Default read-your-writes setting.
const fn default_read_your_writes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TursoConfig {
    /// Database URL (e.g., `libsql://mydb.turso.io`).
    #[serde(default)]
    pub url: String,

    /// Database auth token (short-lived, minted from platform API key).
    #[serde(default)]
    pub auth_token: String,

    /// Long-lived platform API key for minting database tokens.
    /// From `turso auth api-tokens mint <name>`.
    #[serde(default)]
    pub platform_api_key: String,

    /// Turso organization slug.
    #[serde(default)]
    pub org_slug: String,

    /// Sync interval for embedded replicas, in seconds.
    #[serde(default = "default_sync_interval_secs")]
    pub sync_interval_secs: u64,

    /// Whether to use read-your-writes consistency.
    #[serde(default = "default_read_your_writes")]
    pub read_your_writes: bool,

    /// Local replica path for embedded replica mode.
    #[serde(default)]
    pub local_replica_path: String,
}

impl Default for TursoConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            auth_token: String::new(),
            platform_api_key: String::new(),
            org_slug: String::new(),
            sync_interval_secs: default_sync_interval_secs(),
            read_your_writes: default_read_your_writes(),
            local_replica_path: String::new(),
        }
    }
}

impl TursoConfig {
    /// Check if the Turso config has the minimum required fields for remote access.
    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && !self.auth_token.is_empty()
    }

    /// Check if embedded replica mode is enabled.
    pub fn has_local_replica(&self) -> bool {
        !self.local_replica_path.is_empty()
    }

    /// Check if the platform API key is available for token minting.
    pub fn can_mint_tokens(&self) -> bool {
        !self.platform_api_key.is_empty() && !self.org_slug.is_empty() && !self.url.is_empty()
    }

    /// Extract the database name from the URL.
    ///
    /// URL format: `libsql://{db_name}-{org_slug}.{region}.turso.io`
    /// Returns the `{db_name}` portion, or `None` if the URL doesn't match.
    pub fn db_name(&self) -> Option<&str> {
        let hostname = self.url.strip_prefix("libsql://")?;
        let org_suffix = format!("-{}.", self.org_slug);
        let idx = hostname.find(&org_suffix)?;
        Some(&hostname[..idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_configured() {
        let config = TursoConfig::default();
        assert!(!config.is_configured());
        assert_eq!(config.sync_interval_secs, 60);
        assert!(config.read_your_writes);
        assert!(!config.has_local_replica());
        assert!(!config.can_mint_tokens());
    }

    #[test]
    fn configured_when_url_and_token_set() {
        let config = TursoConfig {
            url: "libsql://mydb.turso.io".into(),
            auth_token: "token123".into(),
            ..Default::default()
        };
        assert!(config.is_configured());
    }

    #[test]
    fn local_replica_detection() {
        let mut config = TursoConfig::default();
        assert!(!config.has_local_replica());

        config.local_replica_path = "./replica.db".into();
        assert!(config.has_local_replica());
    }

    #[test]
    fn can_mint_tokens_when_all_fields_set() {
        let config = TursoConfig {
            url: "libsql://zenith-dev-wrath-codes.aws-us-east-1.turso.io".into(),
            platform_api_key: "eyJ...".into(),
            org_slug: "wrath-codes".into(),
            ..Default::default()
        };
        assert!(config.can_mint_tokens());
        assert!(!config.is_configured()); // no auth_token yet
    }

    #[test]
    fn db_name_extraction() {
        let config = TursoConfig {
            url: "libsql://zenith-dev-wrath-codes.aws-us-east-1.turso.io".into(),
            org_slug: "wrath-codes".into(),
            ..Default::default()
        };
        assert_eq!(config.db_name(), Some("zenith-dev"));
    }

    #[test]
    fn db_name_returns_none_for_invalid_url() {
        let config = TursoConfig {
            url: "not-a-libsql-url".into(),
            org_slug: "wrath-codes".into(),
            ..Default::default()
        };
        assert_eq!(config.db_name(), None);
    }
}

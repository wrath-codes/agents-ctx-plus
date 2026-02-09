//! `MotherDuck` configuration.

use serde::{Deserialize, Serialize};

/// Default database name.
fn default_db_name() -> String {
    String::from("zenith")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MotherDuckConfig {
    /// `MotherDuck` access token.
    #[serde(default)]
    pub access_token: String,

    /// Database name in `MotherDuck`.
    #[serde(default = "default_db_name")]
    pub db_name: String,
}

impl Default for MotherDuckConfig {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            db_name: default_db_name(),
        }
    }
}

impl MotherDuckConfig {
    /// Check if the `MotherDuck` config has the minimum required fields.
    #[must_use]
    pub const fn is_configured(&self) -> bool {
        !self.access_token.is_empty()
    }

    /// Build the `MotherDuck` connection string.
    ///
    /// Format: `md:{db_name}?motherduck_token={token}`
    #[must_use]
    pub fn connection_string(&self) -> String {
        format!("md:{}?motherduck_token={}", self.db_name, self.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_configured() {
        let config = MotherDuckConfig::default();
        assert!(!config.is_configured());
        assert_eq!(config.db_name, "zenith");
    }

    #[test]
    fn configured_when_token_set() {
        let config = MotherDuckConfig {
            access_token: "token123".into(),
            db_name: "mydb".into(),
        };
        assert!(config.is_configured());
    }

    #[test]
    fn connection_string_format() {
        let config = MotherDuckConfig {
            access_token: "token123".into(),
            db_name: "mydb".into(),
        };
        assert_eq!(
            config.connection_string(),
            "md:mydb?motherduck_token=token123"
        );
    }
}

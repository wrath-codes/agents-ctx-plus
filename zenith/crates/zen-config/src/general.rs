//! General application configuration.

use serde::{Deserialize, Serialize};

/// Default result limit.
const fn default_limit() -> u32 {
    20
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    /// Whether to auto-commit JSONL trail files on wrap-up.
    #[serde(default)]
    pub auto_commit: bool,

    /// Default ecosystem filter (e.g., "rust", "npm").
    #[serde(default)]
    pub default_ecosystem: String,

    /// Default result limit for list/search commands.
    #[serde(default = "default_limit")]
    pub default_limit: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            auto_commit: false,
            default_ecosystem: String::new(),
            default_limit: default_limit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let config = GeneralConfig::default();
        assert!(!config.auto_commit);
        assert!(config.default_ecosystem.is_empty());
        assert_eq!(config.default_limit, 20);
    }
}

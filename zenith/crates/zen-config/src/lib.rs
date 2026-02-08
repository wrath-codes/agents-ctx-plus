//! # zen-config
//!
//! Layered configuration loading for Zenith using figment.
//!
//! Configuration sources (in priority order, highest wins):
//! 1. Environment variables (`ZENITH_*` prefix, `__` as separator)
//! 2. Project-level `.zenith/config.toml`
//! 3. User-level `~/.config/zenith/config.toml`
//! 4. Built-in defaults
//!
//! # Environment Variable Mapping
//!
//! Figment maps `ZENITH_TURSO__URL` -> `turso.url`, `ZENITH_R2__ACCOUNT_ID` -> `r2.account_id`, etc.
//! The `__` (double underscore) separates nested config sections.
//!
//! # Usage
//!
//! ```no_run
//! use zen_config::ZenConfig;
//!
//! // Load from all sources (dotenvy + TOML + env):
//! let config = ZenConfig::load_with_dotenv().expect("config");
//!
//! // Or without dotenvy (env vars must already be set):
//! let config = ZenConfig::load().expect("config");
//!
//! if config.turso.is_configured() {
//!     println!("Turso URL: {}", config.turso.url);
//! }
//! ```

mod error;
mod general;
mod motherduck;
mod r2;
mod turso;

pub use error::ConfigError;
pub use general::GeneralConfig;
pub use motherduck::MotherDuckConfig;
pub use r2::R2Config;
pub use turso::TursoConfig;

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ZenConfig {
    #[serde(default)]
    pub turso: TursoConfig,
    #[serde(default)]
    pub motherduck: MotherDuckConfig,
    #[serde(default)]
    pub r2: R2Config,
    #[serde(default)]
    pub general: GeneralConfig,
}

impl ZenConfig {
    /// Load configuration from all sources (TOML files + environment variables).
    ///
    /// Does NOT call `dotenvy` -- use [`load_with_dotenv`] if you need `.env` file loading.
    ///
    /// Precedence (highest to lowest):
    /// 1. Environment variables (`ZENITH_*` prefix)
    /// 2. `.zenith/config.toml` (project-local)
    /// 3. `~/.config/zenith/config.toml` (user-global)
    /// 4. Default values
    pub fn load() -> Result<Self, ConfigError> {
        Self::figment().extract().map_err(ConfigError::from)
    }

    /// Load configuration with `.env` file support.
    ///
    /// Calls `dotenvy` to load the `.env` file from the workspace root before
    /// building the figment. This is the typical entry point for CLI and tests.
    pub fn load_with_dotenv() -> Result<Self, ConfigError> {
        Self::load_dotenv_from_workspace();
        Self::load()
    }

    /// Build the figment provider chain.
    ///
    /// This is public so tests can inspect the figment directly or add
    /// additional providers on top.
    pub fn figment() -> Figment {
        let mut figment = Figment::from(Serialized::defaults(Self::default()));

        // Layer 1: User-global config
        if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                figment = figment.merge(Toml::file(global_path));
            }
        }

        // Layer 2: Project-local config
        let local_path = PathBuf::from(".zenith/config.toml");
        if local_path.exists() {
            figment = figment.merge(Toml::file(local_path));
        }

        // Layer 3: Environment variables (highest priority)
        figment = figment.merge(Env::prefixed("ZENITH_").split("__"));

        figment
    }

    /// Path to the user-global config file.
    fn global_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("zenith").join("config.toml"))
    }

    /// Load `.env` from the workspace root.
    ///
    /// Walks up from `CARGO_MANIFEST_DIR` (if available) or current dir looking
    /// for a `.env` file. Silently does nothing if no `.env` is found.
    fn load_dotenv_from_workspace() {
        // In tests/build: CARGO_MANIFEST_DIR points to the crate dir.
        // Walk up to find workspace root's .env.
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let mut dir = PathBuf::from(manifest_dir);
            // Walk up at most 3 levels (crate -> crates/ -> zenith/)
            for _ in 0..3 {
                let env_path = dir.join(".env");
                if env_path.exists() {
                    let _ = dotenvy::from_path(&env_path);
                    return;
                }
                if !dir.pop() {
                    break;
                }
            }
        }

        // Fallback: try current directory
        let _ = dotenvy::dotenv();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads() {
        let config = ZenConfig::default();
        assert!(!config.turso.is_configured());
        assert!(!config.motherduck.is_configured());
        assert!(!config.r2.is_configured());
        assert!(!config.general.auto_commit);
    }

    #[test]
    fn figment_builds_without_files() {
        let figment = ZenConfig::figment();
        let config: ZenConfig = figment.extract().expect("should extract defaults");
        assert!(!config.turso.is_configured());
        assert!(!config.motherduck.is_configured());
        assert!(!config.r2.is_configured());
        assert_eq!(config.general.default_limit, 20);
    }
}

//! # zen-registry
//!
//! Package registry HTTP clients for Zenith.
//!
//! Resolves package names to repository URLs and metadata via ecosystem-specific
//! registries:
//! - crates.io (Rust)
//! - npm (JavaScript/TypeScript)
//! - `PyPI` (Python)
//! - hex.pm (Elixir)
//! - proxy.golang.org / pkg.go.dev (Go)
//! - rubygems.org (Ruby)
//! - packagist.org (PHP)
//! - search.maven.org (Java)
//! - nuget.org (C#/.NET)
//! - hackage.haskell.org (Haskell)
//! - luarocks.org (Lua/Neovim)

pub mod crates_io;
pub mod csharp;
pub mod go;
pub mod haskell;
pub mod hex;
pub mod java;
pub mod lua;
pub mod npm;
pub mod php;
pub mod pypi;
pub mod ruby;

mod error;
mod http;

pub use error::RegistryError;

use serde::{Deserialize, Serialize};

// ── Types ──────────────────────────────────────────────────────────

/// Normalized package metadata from any registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name (e.g., `tokio`, `express`, `laravel/framework`).
    pub name: String,
    /// Latest or specified version string.
    pub version: String,
    /// Ecosystem identifier (e.g., `rust`, `npm`, `pypi`, `go`).
    pub ecosystem: String,
    /// Package description or summary.
    pub description: String,
    /// Total download count (0 if unavailable from registry).
    pub downloads: u64,
    /// SPDX license identifier or license URL.
    pub license: Option<String>,
    /// Source code repository URL.
    pub repository: Option<String>,
    /// Project homepage URL.
    pub homepage: Option<String>,
}

// ── Client ─────────────────────────────────────────────────────────

/// HTTP client for querying package registries across ecosystems.
pub struct RegistryClient {
    http: reqwest::Client,
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClient {
    /// Create a new registry client with default settings.
    ///
    /// # Panics
    ///
    /// Panics if the underlying `reqwest::Client` fails to build.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("zenith/0.1")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client should build"),
        }
    }

    /// Search all registries concurrently. Returns merged results sorted by
    /// downloads (descending).
    ///
    /// Individual registry failures are logged and treated as empty results —
    /// one failing registry does not fail the entire search.
    pub async fn search_all(&self, query: &str, limit: usize) -> Vec<PackageInfo> {
        let (crates, npm, pypi, hex, go, ruby, php, java, csharp, haskell, lua) = tokio::join!(
            self.search_crates_io(query, limit),
            self.search_npm(query, limit),
            self.search_pypi(query, limit),
            self.search_hex(query, limit),
            self.search_go(query, limit),
            self.search_rubygems(query, limit),
            self.search_packagist(query, limit),
            self.search_maven(query, limit),
            self.search_nuget(query, limit),
            self.search_hackage(query, limit),
            self.search_luarocks(query, limit),
        );

        let unwrap_or_log =
            |result: Result<Vec<PackageInfo>, RegistryError>, registry: &str| -> Vec<PackageInfo> {
                result.unwrap_or_else(|e| {
                    tracing::warn!(registry, %e, "registry search failed");
                    Vec::new()
                })
            };

        let mut results = Vec::new();
        results.extend(unwrap_or_log(crates, "crates.io"));
        results.extend(unwrap_or_log(npm, "npm"));
        results.extend(unwrap_or_log(pypi, "pypi"));
        results.extend(unwrap_or_log(hex, "hex.pm"));
        results.extend(unwrap_or_log(go, "go"));
        results.extend(unwrap_or_log(ruby, "rubygems"));
        results.extend(unwrap_or_log(php, "packagist"));
        results.extend(unwrap_or_log(java, "maven"));
        results.extend(unwrap_or_log(csharp, "nuget"));
        results.extend(unwrap_or_log(haskell, "hackage"));
        results.extend(unwrap_or_log(lua, "luarocks"));
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results.truncate(limit);
        results
    }

    /// Search a specific ecosystem by name.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search(
        &self,
        query: &str,
        ecosystem: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        match ecosystem {
            "rust" | "cargo" => self.search_crates_io(query, limit).await,
            "npm" | "javascript" | "typescript" => self.search_npm(query, limit).await,
            "pypi" | "python" => self.search_pypi(query, limit).await,
            "hex" | "elixir" => self.search_hex(query, limit).await,
            "go" | "golang" => self.search_go(query, limit).await,
            "ruby" | "rubygems" => self.search_rubygems(query, limit).await,
            "php" | "packagist" => self.search_packagist(query, limit).await,
            "java" | "maven" => self.search_maven(query, limit).await,
            "csharp" | "nuget" | "dotnet" => self.search_nuget(query, limit).await,
            "haskell" | "hackage" => self.search_hackage(query, limit).await,
            "lua" | "luarocks" | "neovim" => self.search_luarocks(query, limit).await,
            _ => Err(RegistryError::UnsupportedEcosystem(ecosystem.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_info_serialization_roundtrip() {
        let pkg = PackageInfo {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            ecosystem: "rust".to_string(),
            description: "A test package".to_string(),
            downloads: 42,
            license: Some("MIT".to_string()),
            repository: Some("https://github.com/test/test".to_string()),
            homepage: None,
        };

        let json = serde_json::to_string(&pkg).unwrap();
        let deserialized: PackageInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-pkg");
        assert_eq!(deserialized.downloads, 42);
        assert!(deserialized.homepage.is_none());
    }

    #[test]
    fn registry_client_default() {
        let _client = RegistryClient::default();
    }

    #[tokio::test]
    async fn search_unsupported_ecosystem() {
        let client = RegistryClient::new();
        let result = client.search("test", "fortran", 5).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RegistryError::UnsupportedEcosystem(_)));
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn live_search_per_ecosystem() {
        let client = RegistryClient::new();
        let ecosystems = [
            ("rust", "tokio"),
            ("npm", "express"),
            ("pypi", "requests"),
            ("hex", "phoenix"),
            ("go", "github.com/gin-gonic/gin"),
            ("ruby", "rails"),
            ("php", "laravel"),
            ("java", "guava"),
            ("csharp", "Newtonsoft.Json"),
            ("haskell", "aeson"),
            ("lua", "telescope"),
        ];

        for (eco, query) in &ecosystems {
            let result = client.search(query, eco, 3).await;
            match &result {
                Ok(pkgs) => {
                    println!("\n── {eco} ({query}) ── {} results", pkgs.len());
                    for p in pkgs {
                        println!(
                            "  {} v{} | {} DL | license={} | repo={}",
                            p.name,
                            p.version,
                            p.downloads,
                            p.license.as_deref().unwrap_or("—"),
                            p.repository.as_deref().unwrap_or("—"),
                        );
                    }
                }
                Err(e) => println!("\n── {eco} ({query}) ── ERROR: {e}"),
            }
        }
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn live_search_all() {
        let client = RegistryClient::new();
        let results = client.search_all("http", 10).await;
        println!("\n── search_all(\"http\", 10) ── {} results", results.len());
        for p in &results {
            println!(
                "  [{:>8}] {} v{} | {} DL",
                p.ecosystem, p.name, p.version, p.downloads,
            );
        }
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn search_ecosystem_aliases() {
        let client = RegistryClient::new();
        for ecosystem in &[
            "rust", "cargo", "npm", "javascript", "typescript", "pypi", "python",
            "hex", "elixir", "go", "golang", "ruby", "rubygems", "php",
            "packagist", "java", "maven", "csharp", "nuget", "dotnet",
            "haskell", "hackage", "lua", "luarocks", "neovim",
        ] {
            let result = client.search("test", ecosystem, 1).await;
            assert!(
                !matches!(result, Err(RegistryError::UnsupportedEcosystem(_))),
                "ecosystem '{ecosystem}' should be supported"
            );
        }
    }
}

//! `PyPI` registry client.
//!
//! `PyPI` deprecated its XML-RPC search endpoint. This client supports direct
//! package lookup via `https://pypi.org/pypi/{name}/json`. For search-like
//! behavior, the caller must supply an exact or near-exact package name.

use std::collections::HashMap;

use crate::{PackageInfo, RegistryClient, error::RegistryError};

#[derive(serde::Deserialize)]
struct PyPiResponse {
    info: PyPiInfo,
}

#[derive(serde::Deserialize)]
struct PyPiInfo {
    name: String,
    version: String,
    summary: Option<String>,
    license: Option<String>,
    home_page: Option<String>,
    project_urls: Option<HashMap<String, String>>,
}

impl RegistryClient {
    /// Look up a package on `PyPI` by exact name.
    ///
    /// `PyPI` has no search API — this performs a direct package lookup.
    /// Returns a single-element `Vec` on success, or an empty `Vec` if the
    /// package is not found (404).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_pypi(
        &self,
        query: &str,
        _limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let url = format!("https://pypi.org/pypi/{}/json", urlencoding::encode(query));
        let resp = self.http.get(&url).send().await?;

        if resp.status() == 404 {
            return Ok(Vec::new());
        }
        if resp.status() == 429 {
            return Err(RegistryError::RateLimited {
                retry_after_secs: 60,
            });
        }
        if !resp.status().is_success() {
            return Err(RegistryError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let data: PyPiResponse = resp.json().await?;
        let repo = data.info.project_urls.as_ref().and_then(|urls| {
            urls.get("Repository")
                .or_else(|| urls.get("Source"))
                .or_else(|| urls.get("Source Code"))
                .cloned()
        });

        Ok(vec![PackageInfo {
            name: data.info.name,
            version: data.info.version,
            ecosystem: "pypi".to_string(),
            description: data.info.summary.unwrap_or_default(),
            downloads: 0,
            license: data.info.license,
            repository: repo,
            homepage: data.info.home_page,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "info": {
            "name": "requests",
            "version": "2.32.3",
            "summary": "Python HTTP for Humans.",
            "license": "Apache-2.0",
            "home_page": "https://requests.readthedocs.io",
            "project_urls": {
                "Source": "https://github.com/psf/requests",
                "Documentation": "https://requests.readthedocs.io"
            }
        },
        "releases": {}
    }"#;

    #[test]
    fn parse_pypi_response() {
        let data: PyPiResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.info.name, "requests");
        assert_eq!(data.info.version, "2.32.3");
        assert_eq!(data.info.license.as_deref(), Some("Apache-2.0"));
    }

    #[test]
    fn maps_to_package_info() {
        let data: PyPiResponse = serde_json::from_str(FIXTURE).unwrap();
        let repo = data.info.project_urls.as_ref().and_then(|urls| {
            urls.get("Repository")
                .or_else(|| urls.get("Source"))
                .cloned()
        });

        let pkg = PackageInfo {
            name: data.info.name,
            version: data.info.version,
            ecosystem: "pypi".to_string(),
            description: data.info.summary.unwrap_or_default(),
            downloads: 0,
            license: data.info.license,
            repository: repo,
            homepage: data.info.home_page,
        };

        assert_eq!(pkg.ecosystem, "pypi");
        assert_eq!(pkg.name, "requests");
        assert_eq!(
            pkg.repository.as_deref(),
            Some("https://github.com/psf/requests")
        );
    }
}

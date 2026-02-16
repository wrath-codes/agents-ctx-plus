//! crates.io registry client.

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

#[derive(serde::Deserialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
}

#[derive(serde::Deserialize)]
struct CrateInfo {
    name: String,
    max_version: String,
    description: Option<String>,
    downloads: u64,
    license: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
}

impl RegistryClient {
    /// Search crates.io for packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_crates_io(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(100);
        let url = format!(
            "https://crates.io/api/v1/crates?q={}&per_page={limit}",
            urlencoding::encode(query)
        );
        let resp = check_response(self.http.get(&url).send().await?).await?;

        let data: CratesResponse = resp.json().await?;
        Ok(data
            .crates
            .into_iter()
            .map(|c| PackageInfo {
                name: c.name,
                version: c.max_version,
                ecosystem: "rust".to_string(),
                description: c.description.unwrap_or_default(),
                downloads: c.downloads,
                license: c.license,
                repository: c.repository,
                homepage: c.homepage,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "crates": [
            {
                "name": "tokio",
                "max_version": "1.49.0",
                "description": "An event-driven, non-blocking I/O platform",
                "downloads": 200000000,
                "license": "MIT",
                "repository": "https://github.com/tokio-rs/tokio",
                "homepage": "https://tokio.rs"
            },
            {
                "name": "tokio-util",
                "max_version": "0.7.12",
                "description": "Additional utilities for working with Tokio",
                "downloads": 50000000,
                "license": "MIT",
                "repository": "https://github.com/tokio-rs/tokio",
                "homepage": null
            }
        ]
    }"#;

    #[test]
    fn parse_crates_io_response() {
        let data: CratesResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.crates.len(), 2);

        let first = &data.crates[0];
        assert_eq!(first.name, "tokio");
        assert_eq!(first.max_version, "1.49.0");
        assert_eq!(first.downloads, 200_000_000);
        assert_eq!(first.license.as_deref(), Some("MIT"));
        assert_eq!(
            first.repository.as_deref(),
            Some("https://github.com/tokio-rs/tokio")
        );
    }

    #[test]
    fn maps_to_package_info() {
        let data: CratesResponse = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .crates
            .into_iter()
            .map(|c| PackageInfo {
                name: c.name,
                version: c.max_version,
                ecosystem: "rust".to_string(),
                description: c.description.unwrap_or_default(),
                downloads: c.downloads,
                license: c.license,
                repository: c.repository,
                homepage: c.homepage,
            })
            .collect();

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].ecosystem, "rust");
        assert_eq!(packages[0].name, "tokio");
        assert!(packages[1].homepage.is_none());
    }
}

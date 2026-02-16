//! Packagist (PHP) registry client.

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

#[derive(serde::Deserialize)]
struct PackagistSearchResponse {
    results: Vec<PackagistResult>,
}

#[derive(serde::Deserialize)]
struct PackagistResult {
    name: String,
    description: Option<String>,
    url: Option<String>,
    repository: Option<String>,
    downloads: u64,
}

#[derive(serde::Deserialize)]
struct PackagistPackageResponse {
    packages: std::collections::HashMap<String, Vec<PackagistVersion>>,
}

#[derive(serde::Deserialize)]
struct PackagistVersion {
    version: String,
    #[serde(default)]
    #[allow(dead_code)]
    version_normalized: Option<String>,
    license: Option<Vec<String>>,
}

impl RegistryClient {
    /// Search Packagist for PHP packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_packagist(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(100);
        let url = format!(
            "https://packagist.org/search.json?q={}&per_page={limit}",
            urlencoding::encode(query)
        );
        let resp = check_response(self.http.get(&url).send().await?).await?;

        let data: PackagistSearchResponse = resp.json().await?;
        let results: Vec<PackagistResult> = data.results.into_iter().take(limit).collect();

        let mut set = tokio::task::JoinSet::new();
        let http = self.http.clone();

        for (idx, result) in results.iter().enumerate() {
            let name = result.name.clone();
            let client = http.clone();
            set.spawn(async move {
                let url = format!("https://repo.packagist.org/p2/{name}.json");
                let resp = client.get(&url).send().await.ok();
                let version_info = match resp {
                    Some(r) if r.status().is_success() => {
                        r.json::<PackagistPackageResponse>().await.ok().and_then(|data| {
                            let versions = data.packages.into_values().next()?;
                            let latest = versions.into_iter().next()?;
                            Some((
                                latest.version,
                                latest.license.and_then(|l| l.into_iter().next()),
                            ))
                        })
                    }
                    _ => None,
                };
                (idx, version_info.unwrap_or_default())
            });
        }

        let mut version_data = vec![(String::new(), None::<String>); results.len()];
        while let Some(res) = set.join_next().await {
            match res {
                Ok((idx, data)) => version_data[idx] = data,
                Err(e) => tracing::warn!(%e, "packagist version fetch task failed"),
            }
        }

        let packages = results
            .into_iter()
            .zip(version_data)
            .map(|(result, (version, license))| PackageInfo {
                name: result.name,
                version,
                ecosystem: "php".to_string(),
                description: result.description.unwrap_or_default(),
                downloads: result.downloads,
                license,
                repository: result.repository,
                homepage: result.url,
            })
            .collect();

        Ok(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_FIXTURE: &str = r#"{
        "results": [
            {
                "name": "laravel/framework",
                "description": "The Laravel Framework.",
                "url": "https://packagist.org/packages/laravel/framework",
                "repository": "https://github.com/laravel/framework",
                "downloads": 300000000
            },
            {
                "name": "laravel/laravel",
                "description": "The Laravel Framework application.",
                "url": "https://packagist.org/packages/laravel/laravel",
                "repository": "https://github.com/laravel/laravel",
                "downloads": 20000000
            }
        ]
    }"#;

    const VERSION_FIXTURE: &str = r#"{
        "packages": {
            "laravel/framework": [
                {
                    "version": "v11.36.1",
                    "version_normalized": "11.36.1.0",
                    "license": ["MIT"]
                },
                {
                    "version": "v11.36.0",
                    "version_normalized": "11.36.0.0",
                    "license": ["MIT"]
                }
            ]
        }
    }"#;

    #[test]
    fn parse_packagist_search() {
        let data: PackagistSearchResponse = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        assert_eq!(data.results.len(), 2);
        assert_eq!(data.results[0].name, "laravel/framework");
        assert_eq!(data.results[0].downloads, 300_000_000);
    }

    #[test]
    fn parse_packagist_version() {
        let data: PackagistPackageResponse = serde_json::from_str(VERSION_FIXTURE).unwrap();
        let versions = data.packages.values().next().unwrap();
        assert_eq!(versions[0].version, "v11.36.1");
        assert_eq!(
            versions[0].license.as_ref().and_then(|l| l.first()).map(String::as_str),
            Some("MIT")
        );
    }

    #[test]
    fn maps_to_package_info() {
        let data: PackagistSearchResponse = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .results
            .into_iter()
            .map(|r| PackageInfo {
                name: r.name,
                version: "v11.36.1".to_string(),
                ecosystem: "php".to_string(),
                description: r.description.unwrap_or_default(),
                downloads: r.downloads,
                license: Some("MIT".to_string()),
                repository: r.repository,
                homepage: r.url,
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "php");
        assert_eq!(packages[0].name, "laravel/framework");
    }
}

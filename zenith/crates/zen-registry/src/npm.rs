//! npm registry client.

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

#[derive(serde::Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmObject>,
}

#[derive(serde::Deserialize)]
struct NpmObject {
    package: NpmPackage,
}

#[derive(serde::Deserialize)]
struct NpmPackage {
    name: String,
    version: String,
    description: Option<String>,
    links: Option<NpmLinks>,
    #[serde(default)]
    license: Option<String>,
}

#[derive(serde::Deserialize)]
struct NpmLinks {
    repository: Option<String>,
    homepage: Option<String>,
}

#[derive(serde::Deserialize)]
struct NpmDownloads {
    downloads: u64,
}

impl RegistryClient {
    /// Search the npm registry for packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_npm(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(250);
        let url = format!(
            "https://registry.npmjs.org/-/v1/search?text={}&size={limit}",
            urlencoding::encode(query)
        );
        let resp = check_response(self.http.get(&url).send().await?).await?;

        let data: NpmSearchResponse = resp.json().await?;
        let names: Vec<&str> = data.objects.iter().map(|o| o.package.name.as_str()).collect();
        let download_counts = self.fetch_npm_downloads_batch(&names).await;

        let packages = data
            .objects
            .into_iter()
            .zip(download_counts)
            .map(|(obj, downloads)| {
                let links = obj.package.links.as_ref();
                PackageInfo {
                    name: obj.package.name,
                    version: obj.package.version,
                    ecosystem: "npm".to_string(),
                    description: obj.package.description.unwrap_or_default(),
                    downloads,
                    license: obj.package.license,
                    repository: links.and_then(|l| l.repository.clone()),
                    homepage: links.and_then(|l| l.homepage.clone()),
                }
            })
            .collect();

        Ok(packages)
    }

    async fn fetch_npm_downloads_batch(&self, names: &[&str]) -> Vec<u64> {
        let mut set = tokio::task::JoinSet::new();
        let http = self.http.clone();

        for (idx, name) in names.iter().enumerate() {
            let url = format!(
                "https://api.npmjs.org/downloads/point/last-month/{}",
                urlencoding::encode(name)
            );
            let client = http.clone();
            set.spawn(async move {
                let resp = client.get(&url).send().await.ok();
                let downloads = match resp {
                    Some(r) if r.status().is_success() => {
                        r.json::<NpmDownloads>().await.map_or(0, |d| d.downloads)
                    }
                    _ => 0,
                };
                (idx, downloads)
            });
        }

        let mut results = vec![0u64; names.len()];
        while let Some(res) = set.join_next().await {
            match res {
                Ok((idx, downloads)) => results[idx] = downloads,
                Err(e) => tracing::warn!(%e, "npm download fetch task failed"),
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "objects": [
            {
                "package": {
                    "name": "express",
                    "version": "4.21.2",
                    "description": "Fast, unopinionated, minimalist web framework",
                    "license": "MIT",
                    "links": {
                        "repository": "https://github.com/expressjs/express",
                        "homepage": "https://expressjs.com"
                    }
                }
            },
            {
                "package": {
                    "name": "express-validator",
                    "version": "7.0.1",
                    "description": "Express middleware for validation",
                    "links": {}
                }
            }
        ]
    }"#;

    #[test]
    fn parse_npm_response() {
        let data: NpmSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.objects.len(), 2);

        let first = &data.objects[0].package;
        assert_eq!(first.name, "express");
        assert_eq!(first.version, "4.21.2");
        assert_eq!(first.license.as_deref(), Some("MIT"));
        assert_eq!(
            first.links.as_ref().and_then(|l| l.repository.as_deref()),
            Some("https://github.com/expressjs/express")
        );
    }

    #[test]
    fn maps_to_package_info() {
        let data: NpmSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .objects
            .into_iter()
            .map(|obj| {
                let links = obj.package.links.as_ref();
                PackageInfo {
                    name: obj.package.name,
                    version: obj.package.version,
                    ecosystem: "npm".to_string(),
                    description: obj.package.description.unwrap_or_default(),
                    downloads: 0,
                    license: obj.package.license,
                    repository: links.and_then(|l| l.repository.clone()),
                    homepage: links.and_then(|l| l.homepage.clone()),
                }
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "npm");
        assert_eq!(packages[0].name, "express");
        assert!(packages[1].license.is_none());
    }
}

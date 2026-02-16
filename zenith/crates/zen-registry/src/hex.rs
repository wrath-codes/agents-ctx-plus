//! hex.pm registry client (Elixir/Erlang).

use std::collections::HashMap;

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

#[derive(serde::Deserialize)]
struct HexPackage {
    name: String,
    latest_stable_version: Option<String>,
    meta: HexMeta,
    downloads: Option<HexDownloads>,
}

#[derive(serde::Deserialize)]
struct HexMeta {
    description: Option<String>,
    licenses: Option<Vec<String>>,
    links: Option<HashMap<String, String>>,
}

#[derive(serde::Deserialize)]
struct HexDownloads {
    all: Option<u64>,
}

impl RegistryClient {
    /// Search hex.pm for packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_hex(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(100);
        let url = format!(
            "https://hex.pm/api/packages?search={}&sort=downloads&page=1&per_page={limit}",
            urlencoding::encode(query)
        );
        let resp = check_response(self.http.get(&url).send().await?).await?;

        let data: Vec<HexPackage> = resp.json().await?;
        Ok(data
            .into_iter()
            .map(|p| {
                let links = p.meta.links.as_ref();
                PackageInfo {
                    name: p.name,
                    version: p.latest_stable_version.unwrap_or_default(),
                    ecosystem: "hex".to_string(),
                    description: p.meta.description.unwrap_or_default(),
                    downloads: p.downloads.and_then(|d| d.all).unwrap_or(0),
                    license: p
                        .meta
                        .licenses
                        .as_ref()
                        .and_then(|l| l.first().cloned()),
                    repository: links.and_then(|l| {
                        l.get("GitHub")
                            .or_else(|| l.get("github"))
                            .or_else(|| l.get("Repository"))
                            .cloned()
                    }),
                    homepage: links.and_then(|l| {
                        l.get("Homepage")
                            .or_else(|| l.get("homepage"))
                            .cloned()
                    }),
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
        {
            "name": "phoenix",
            "latest_stable_version": "1.7.14",
            "meta": {
                "description": "Peace of mind from prototype to production",
                "licenses": ["MIT"],
                "links": {
                    "GitHub": "https://github.com/phoenixframework/phoenix",
                    "Homepage": "https://www.phoenixframework.org"
                }
            },
            "downloads": { "all": 30000000 }
        },
        {
            "name": "phoenix_live_view",
            "latest_stable_version": "0.20.17",
            "meta": {
                "description": "Rich, real-time user experiences with server-rendered HTML",
                "licenses": ["MIT"],
                "links": {}
            },
            "downloads": { "all": 15000000 }
        }
    ]"#;

    #[test]
    fn parse_hex_response() {
        let data: Vec<HexPackage> = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].name, "phoenix");
        assert_eq!(
            data[0].latest_stable_version.as_deref(),
            Some("1.7.14")
        );
        assert_eq!(data[0].downloads.as_ref().and_then(|d| d.all), Some(30_000_000));
    }

    #[test]
    fn maps_to_package_info() {
        let data: Vec<HexPackage> = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .into_iter()
            .map(|p| {
                let links = p.meta.links.as_ref();
                PackageInfo {
                    name: p.name,
                    version: p.latest_stable_version.unwrap_or_default(),
                    ecosystem: "hex".to_string(),
                    description: p.meta.description.unwrap_or_default(),
                    downloads: p.downloads.and_then(|d| d.all).unwrap_or(0),
                    license: p.meta.licenses.as_ref().and_then(|l| l.first().cloned()),
                    repository: links.and_then(|l| l.get("GitHub").cloned()),
                    homepage: links.and_then(|l| l.get("Homepage").cloned()),
                }
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "hex");
        assert_eq!(packages[0].name, "phoenix");
        assert_eq!(
            packages[0].repository.as_deref(),
            Some("https://github.com/phoenixframework/phoenix")
        );
        assert!(packages[1].repository.is_none());
    }
}

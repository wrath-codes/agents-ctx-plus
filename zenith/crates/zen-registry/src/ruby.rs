//! `RubyGems` registry client.

use crate::{PackageInfo, RegistryClient, error::RegistryError};

#[derive(serde::Deserialize)]
struct RubyGem {
    name: String,
    version: String,
    info: Option<String>,
    downloads: u64,
    licenses: Option<Vec<String>>,
    homepage_uri: Option<String>,
    source_code_uri: Option<String>,
}

impl RegistryClient {
    /// Search `RubyGems` for packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_rubygems(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        // RubyGems search API returns 30 per page; truncated client-side.
        let url = format!(
            "https://rubygems.org/api/v1/search.json?query={}",
            urlencoding::encode(query)
        );
        let resp = self.http.get(&url).send().await?;

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

        let data: Vec<RubyGem> = resp.json().await?;
        Ok(data
            .into_iter()
            .take(limit)
            .map(|g| PackageInfo {
                name: g.name,
                version: g.version,
                ecosystem: "ruby".to_string(),
                description: g.info.unwrap_or_default(),
                downloads: g.downloads,
                license: g.licenses.and_then(|l| l.into_iter().next()),
                repository: g.source_code_uri,
                homepage: g.homepage_uri,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
        {
            "name": "rails",
            "version": "8.0.2",
            "info": "Ruby on Rails is a full-stack web framework",
            "downloads": 500000000,
            "licenses": ["MIT"],
            "homepage_uri": "https://rubyonrails.org",
            "source_code_uri": "https://github.com/rails/rails"
        },
        {
            "name": "railties",
            "version": "8.0.2",
            "info": "Rails internals",
            "downloads": 300000000,
            "licenses": ["MIT"],
            "homepage_uri": null,
            "source_code_uri": "https://github.com/rails/rails"
        }
    ]"#;

    #[test]
    fn parse_rubygems_response() {
        let data: Vec<RubyGem> = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].name, "rails");
        assert_eq!(data[0].downloads, 500_000_000);
    }

    #[test]
    fn maps_to_package_info() {
        let data: Vec<RubyGem> = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .into_iter()
            .map(|g| PackageInfo {
                name: g.name,
                version: g.version,
                ecosystem: "ruby".to_string(),
                description: g.info.unwrap_or_default(),
                downloads: g.downloads,
                license: g.licenses.and_then(|l| l.into_iter().next()),
                repository: g.source_code_uri,
                homepage: g.homepage_uri,
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "ruby");
        assert_eq!(packages[0].name, "rails");
        assert_eq!(packages[0].license.as_deref(), Some("MIT"));
        assert!(packages[1].homepage.is_none());
    }
}

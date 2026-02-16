//! Go module registry client.
//!
//! `proxy.golang.org` is a module proxy, not a search API. Only direct module
//! path lookups are supported (queries containing `.` are treated as module
//! paths). Keyword searches return empty results. Download counts are not
//! available — results use `downloads: 0`.

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

/// Encode a Go module path per the module proxy protocol.
///
/// Uppercase letters are replaced with `!` followed by the lowercase letter.
/// See <https://go.dev/ref/mod#goproxy-protocol>.
fn encode_go_module_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len() + 8);
    for c in path.chars() {
        if c.is_ascii_uppercase() {
            encoded.push('!');
            encoded.push(c.to_ascii_lowercase());
        } else {
            encoded.push(c);
        }
    }
    encoded
}

#[derive(serde::Deserialize)]
struct GoProxyInfo {
    #[serde(rename = "Version")]
    version: String,
}

impl RegistryClient {
    /// Look up a Go module by path via `proxy.golang.org`.
    ///
    /// Only queries that look like module paths (contain `.`) are supported.
    /// Keyword searches return empty results because Go has no public search
    /// API that returns JSON.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_go(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        // proxy.golang.org is a module proxy, not a search API.
        // Only direct module path lookups (containing '.') are supported.
        if !query.contains('.') {
            return Ok(Vec::new());
        }

        self.lookup_go_module(query)
            .await
            .map(|pkg| pkg.into_iter().take(limit).collect())
    }

    async fn lookup_go_module(
        &self,
        module_path: &str,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let encoded = encode_go_module_path(module_path);
        let url = format!("https://proxy.golang.org/{encoded}/@latest");
        let resp = self.http.get(&url).send().await?;

        if resp.status() == 404 || resp.status() == 410 {
            return Ok(Vec::new());
        }
        let resp = check_response(resp).await?;

        let info: GoProxyInfo = resp.json().await?;
        Ok(vec![PackageInfo {
            name: module_path.to_string(),
            version: info.version,
            ecosystem: "go".to_string(),
            description: String::new(),
            downloads: 0,
            license: None,
            repository: None,
            homepage: Some(format!("https://pkg.go.dev/{module_path}")),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROXY_FIXTURE: &str = r#"{
        "Version": "v1.10.0",
        "Time": "2024-06-01T00:00:00Z"
    }"#;

    #[test]
    fn parse_go_proxy_info() {
        let info: GoProxyInfo = serde_json::from_str(PROXY_FIXTURE).unwrap();
        assert_eq!(info.version, "v1.10.0");
    }

    #[test]
    fn maps_to_package_info() {
        let info: GoProxyInfo = serde_json::from_str(PROXY_FIXTURE).unwrap();
        let pkg = PackageInfo {
            name: "github.com/gin-gonic/gin".to_string(),
            version: info.version,
            ecosystem: "go".to_string(),
            description: String::new(),
            downloads: 0,
            license: None,
            repository: None,
            homepage: Some("https://pkg.go.dev/github.com/gin-gonic/gin".to_string()),
        };

        assert_eq!(pkg.ecosystem, "go");
        assert_eq!(pkg.name, "github.com/gin-gonic/gin");
        assert_eq!(pkg.version, "v1.10.0");
        assert_eq!(pkg.downloads, 0);
    }
}

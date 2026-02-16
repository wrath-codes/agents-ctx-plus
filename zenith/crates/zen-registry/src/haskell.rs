//! Hackage (Haskell) registry client.
//!
//! Hackage has no JSON search API. This client performs a two-step direct
//! package lookup: first fetching the version list from `preferred.json`,
//! then fetching metadata for the latest version. Download counts are not
//! available — results use `downloads: 0`.

use crate::{PackageInfo, RegistryClient, error::RegistryError};

/// Preferred versions response — lists normal and deprecated versions.
#[derive(serde::Deserialize)]
struct HackagePreferred {
    #[serde(rename = "normal-version")]
    normal_version: Vec<String>,
}

/// Package metadata returned by `GET /package/{name}-{version}` with
/// `Accept: application/json`.
#[derive(serde::Deserialize)]
struct HackagePackageMeta {
    synopsis: Option<String>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
}

impl RegistryClient {
    /// Look up a Haskell package on Hackage by exact name.
    ///
    /// Uses a two-step lookup:
    /// 1. `GET /package/{name}/preferred.json` → latest stable version
    /// 2. `GET /package/{name}-{version}` (Accept: application/json) → metadata
    ///
    /// Returns a single-element `Vec` on success, or an empty `Vec` if not
    /// found (404).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_hackage(
        &self,
        query: &str,
        _limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let encoded = urlencoding::encode(query);

        // Step 1: get version list
        let pref_url = format!("https://hackage.haskell.org/package/{encoded}/preferred.json");
        let resp = self.http.get(&pref_url).send().await?;

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

        let preferred: HackagePreferred = resp.json().await.map_err(|e| {
            RegistryError::Parse(format!("hackage preferred.json parse error: {e}"))
        })?;

        let Some(latest) = preferred.normal_version.first() else {
            return Ok(Vec::new());
        };

        // Step 2: fetch metadata for latest version
        let meta_url =
            format!("https://hackage.haskell.org/package/{encoded}-{latest}");
        let meta_resp = self
            .http
            .get(&meta_url)
            .header("Accept", "application/json")
            .send()
            .await;

        let (description, license, homepage) = match meta_resp {
            Ok(r) if r.status().is_success() => {
                match r.json::<HackagePackageMeta>().await {
                    Ok(meta) => {
                        let desc = meta
                            .synopsis
                            .or(meta.description)
                            .unwrap_or_default();
                        (desc, meta.license, meta.homepage)
                    }
                    Err(_) => (String::new(), None, None),
                }
            }
            _ => (String::new(), None, None),
        };

        let (repository, homepage) = if homepage
            .as_deref()
            .is_some_and(|h| h.contains("github.com") || h.contains("gitlab.com"))
        {
            // GitHub/GitLab URL → use as repository, Hackage page as homepage
            (homepage, Some(format!("https://hackage.haskell.org/package/{encoded}")))
        } else {
            (None, homepage)
        };

        Ok(vec![PackageInfo {
            name: query.to_string(),
            version: latest.clone(),
            ecosystem: "haskell".to_string(),
            description,
            downloads: 0,
            license,
            repository,
            homepage,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFERRED_FIXTURE: &str = r#"{
        "normal-version": ["2.2.3.0", "2.2.2.0", "2.2.1.0"],
        "deprecated-version": ["0.10.0.0"]
    }"#;

    const META_FIXTURE: &str = r#"{
        "author": "Bryan O'Sullivan",
        "synopsis": "Fast JSON parsing and encoding",
        "description": "A JSON parsing and encoding library",
        "license": "BSD-3-Clause",
        "homepage": "https://github.com/haskell/aeson",
        "uploaded_at": "2024-01-01T00:00:00Z"
    }"#;

    #[test]
    fn parse_preferred_versions() {
        let pref: HackagePreferred = serde_json::from_str(PREFERRED_FIXTURE).unwrap();
        assert_eq!(pref.normal_version[0], "2.2.3.0");
        assert_eq!(pref.normal_version.len(), 3);
    }

    #[test]
    fn parse_package_meta() {
        let meta: HackagePackageMeta = serde_json::from_str(META_FIXTURE).unwrap();
        assert_eq!(meta.synopsis.as_deref(), Some("Fast JSON parsing and encoding"));
        assert_eq!(meta.license.as_deref(), Some("BSD-3-Clause"));
        assert_eq!(
            meta.homepage.as_deref(),
            Some("https://github.com/haskell/aeson")
        );
    }

    #[test]
    fn maps_to_package_info() {
        let meta: HackagePackageMeta = serde_json::from_str(META_FIXTURE).unwrap();
        let version = "2.2.3.0".to_string();
        let homepage = meta.homepage;

        let (repository, homepage) = if homepage
            .as_deref()
            .is_some_and(|h| h.contains("github.com") || h.contains("gitlab.com"))
        {
            (homepage, Some("https://hackage.haskell.org/package/aeson".to_string()))
        } else {
            (None, homepage)
        };

        let pkg = PackageInfo {
            name: "aeson".to_string(),
            version,
            ecosystem: "haskell".to_string(),
            description: meta.synopsis.unwrap_or_default(),
            downloads: 0,
            license: meta.license,
            repository,
            homepage,
        };

        assert_eq!(pkg.ecosystem, "haskell");
        assert_eq!(pkg.name, "aeson");
        assert_eq!(pkg.version, "2.2.3.0");
        assert_eq!(pkg.license.as_deref(), Some("BSD-3-Clause"));
        assert_eq!(
            pkg.repository.as_deref(),
            Some("https://github.com/haskell/aeson")
        );
        assert_eq!(
            pkg.homepage.as_deref(),
            Some("https://hackage.haskell.org/package/aeson")
        );
    }
}

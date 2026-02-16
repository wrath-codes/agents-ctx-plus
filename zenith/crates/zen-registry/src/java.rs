//! Maven Central (Java) registry client.
//!
//! Download counts are not available via the Maven Central search API —
//! results use `downloads: 0`.

use crate::{PackageInfo, RegistryClient, error::RegistryError};

#[derive(serde::Deserialize)]
struct MavenSearchResponse {
    response: MavenResponseBody,
}

#[derive(serde::Deserialize)]
struct MavenResponseBody {
    docs: Vec<MavenDoc>,
}

#[derive(serde::Deserialize)]
struct MavenDoc {
    /// Group ID (e.g., `com.google.guava`).
    g: String,
    /// Artifact ID (e.g., `guava`).
    a: String,
    /// Latest version.
    #[serde(rename = "latestVersion")]
    latest_version: Option<String>,
    /// Packaging type (e.g., `jar`).
    #[serde(default)]
    #[allow(dead_code)]
    p: Option<String>,
}

impl RegistryClient {
    /// Search Maven Central for Java packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_maven(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(200);
        let url = format!(
            "https://search.maven.org/solrsearch/select?q={}&rows={limit}&wt=json",
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

        let data: MavenSearchResponse = resp.json().await?;
        Ok(data
            .response
            .docs
            .into_iter()
            .map(|d| {
                let name = format!("{}:{}", d.g, d.a);
                let homepage = d.latest_version.as_ref().map(|v| {
                    format!(
                        "https://search.maven.org/artifact/{}/{}/{v}",
                        d.g, d.a,
                    )
                });
                PackageInfo {
                    name,
                    version: d.latest_version.unwrap_or_default(),
                    ecosystem: "java".to_string(),
                    description: String::new(),
                    downloads: 0,
                    license: None,
                    repository: None,
                    homepage,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "responseHeader": { "status": 0 },
        "response": {
            "numFound": 2,
            "docs": [
                {
                    "g": "com.google.guava",
                    "a": "guava",
                    "latestVersion": "33.4.0-jre",
                    "p": "jar"
                },
                {
                    "g": "com.google.guava",
                    "a": "guava-testlib",
                    "latestVersion": "33.4.0-jre",
                    "p": "jar"
                }
            ]
        }
    }"#;

    #[test]
    fn parse_maven_response() {
        let data: MavenSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.response.docs.len(), 2);
        assert_eq!(data.response.docs[0].g, "com.google.guava");
        assert_eq!(data.response.docs[0].a, "guava");
        assert_eq!(
            data.response.docs[0].latest_version.as_deref(),
            Some("33.4.0-jre")
        );
    }

    #[test]
    fn maps_to_package_info() {
        let data: MavenSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .response
            .docs
            .into_iter()
            .map(|d| {
                let homepage = d.latest_version.as_ref().map(|v| {
                    format!(
                        "https://search.maven.org/artifact/{}/{}/{v}",
                        d.g, d.a,
                    )
                });
                PackageInfo {
                    name: format!("{}:{}", d.g, d.a),
                    version: d.latest_version.unwrap_or_default(),
                    ecosystem: "java".to_string(),
                    description: String::new(),
                    downloads: 0,
                    license: None,
                    repository: None,
                    homepage,
                }
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "java");
        assert_eq!(packages[0].name, "com.google.guava:guava");
        assert_eq!(packages[0].version, "33.4.0-jre");
        assert_eq!(packages[0].downloads, 0);
        assert_eq!(packages[0].repository, None);
        assert_eq!(
            packages[0].homepage.as_deref(),
            Some("https://search.maven.org/artifact/com.google.guava/guava/33.4.0-jre")
        );
    }
}

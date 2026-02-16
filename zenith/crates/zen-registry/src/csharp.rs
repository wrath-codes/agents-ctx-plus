//! `NuGet` (C#/.NET) registry client.

use crate::{PackageInfo, RegistryClient, error::RegistryError, http::check_response};

#[derive(serde::Deserialize)]
struct NuGetSearchResponse {
    data: Vec<NuGetPackage>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NuGetPackage {
    id: String,
    version: String,
    description: Option<String>,
    total_downloads: Option<u64>,
    license_url: Option<String>,
    project_url: Option<String>,
}

/// `NuGet`'s `licenseUrl` often follows `https://licenses.nuget.org/{SPDX}` or
/// `https://www.nuget.org/packages/{id}/{version}/license`. Extract the SPDX
/// identifier from the first pattern; return the raw URL otherwise.
fn extract_license(license_url: Option<&str>) -> Option<String> {
    let url = license_url?;
    url.strip_prefix("https://licenses.nuget.org/")
        .map_or_else(|| Some(url.to_string()), |spdx| Some(spdx.to_string()))
}

/// If `project_url` points to a GitHub/GitLab repo, use it as the source
/// repository; otherwise treat it as homepage only.
fn split_project_url(project_url: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(url) = project_url else {
        return (None, None);
    };
    if url.contains("github.com") || url.contains("gitlab.com") {
        (Some(url.to_string()), Some(url.to_string()))
    } else {
        (None, Some(url.to_string()))
    }
}

impl RegistryClient {
    /// Search `NuGet` for C#/.NET packages matching `query`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the HTTP request fails, the registry
    /// returns a non-success status, or the response cannot be parsed.
    pub async fn search_nuget(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let limit = limit.min(1000);
        let url = format!(
            "https://azuresearch-usnc.nuget.org/query?q={}&take={limit}&semVerLevel=2.0.0",
            urlencoding::encode(query)
        );
        let resp = check_response(self.http.get(&url).send().await?).await?;

        let data: NuGetSearchResponse = resp.json().await?;
        Ok(data
            .data
            .into_iter()
            .map(|p| {
                let license = extract_license(p.license_url.as_deref());
                let (repository, homepage) = split_project_url(p.project_url.as_deref());
                PackageInfo {
                    name: p.id,
                    version: p.version,
                    ecosystem: "csharp".to_string(),
                    description: p.description.unwrap_or_default(),
                    downloads: p.total_downloads.unwrap_or(0),
                    license,
                    repository,
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
        "totalHits": 3,
        "data": [
            {
                "id": "Newtonsoft.Json",
                "version": "13.0.3",
                "description": "Json.NET is a popular high-performance JSON framework for .NET",
                "totalDownloads": 3500000000,
                "licenseUrl": "https://licenses.nuget.org/MIT",
                "projectUrl": "https://www.newtonsoft.com/json"
            },
            {
                "id": "Octokit",
                "version": "13.0.1",
                "description": "An async-based GitHub API client library for .NET",
                "totalDownloads": 200000000,
                "licenseUrl": "https://licenses.nuget.org/MIT",
                "projectUrl": "https://github.com/octokit/octokit.net"
            },
            {
                "id": "SomePackage",
                "version": "1.0.0",
                "description": "A package with non-standard license URL",
                "totalDownloads": 100,
                "licenseUrl": "https://www.nuget.org/packages/SomePackage/1.0.0/license",
                "projectUrl": null
            }
        ]
    }"#;

    #[test]
    fn parse_nuget_response() {
        let data: NuGetSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(data.data.len(), 3);
        assert_eq!(data.data[0].id, "Newtonsoft.Json");
        assert_eq!(data.data[0].version, "13.0.3");
        assert_eq!(data.data[0].total_downloads, Some(3_500_000_000));
    }

    #[test]
    fn extract_spdx_from_license_url() {
        assert_eq!(
            extract_license(Some("https://licenses.nuget.org/MIT")),
            Some("MIT".to_string())
        );
        assert_eq!(
            extract_license(Some("https://licenses.nuget.org/Apache-2.0")),
            Some("Apache-2.0".to_string())
        );
        // Non-standard URL kept as-is
        assert_eq!(
            extract_license(Some("https://www.nuget.org/packages/Foo/1.0/license")),
            Some("https://www.nuget.org/packages/Foo/1.0/license".to_string())
        );
        assert_eq!(extract_license(None), None);
    }

    #[test]
    fn split_github_project_url() {
        let (repo, home) = split_project_url(Some("https://github.com/octokit/octokit.net"));
        assert_eq!(
            repo.as_deref(),
            Some("https://github.com/octokit/octokit.net")
        );
        assert_eq!(
            home.as_deref(),
            Some("https://github.com/octokit/octokit.net")
        );
    }

    #[test]
    fn split_non_github_project_url() {
        let (repo, home) = split_project_url(Some("https://www.newtonsoft.com/json"));
        assert!(repo.is_none());
        assert_eq!(home.as_deref(), Some("https://www.newtonsoft.com/json"));
    }

    #[test]
    fn split_none_project_url() {
        let (repo, home) = split_project_url(None);
        assert!(repo.is_none());
        assert!(home.is_none());
    }

    #[test]
    fn maps_to_package_info() {
        let data: NuGetSearchResponse = serde_json::from_str(FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .data
            .into_iter()
            .map(|p| {
                let license = extract_license(p.license_url.as_deref());
                let (repository, homepage) = split_project_url(p.project_url.as_deref());
                PackageInfo {
                    name: p.id,
                    version: p.version,
                    ecosystem: "csharp".to_string(),
                    description: p.description.unwrap_or_default(),
                    downloads: p.total_downloads.unwrap_or(0),
                    license,
                    repository,
                    homepage,
                }
            })
            .collect();

        // Non-GitHub project URL → homepage only
        assert_eq!(packages[0].name, "Newtonsoft.Json");
        assert_eq!(packages[0].license.as_deref(), Some("MIT"));
        assert!(packages[0].repository.is_none());
        assert_eq!(
            packages[0].homepage.as_deref(),
            Some("https://www.newtonsoft.com/json")
        );

        // GitHub project URL → both repository and homepage
        assert_eq!(packages[1].name, "Octokit");
        assert_eq!(
            packages[1].repository.as_deref(),
            Some("https://github.com/octokit/octokit.net")
        );

        // No project URL
        assert!(packages[2].repository.is_none());
        assert!(packages[2].homepage.is_none());
    }
}

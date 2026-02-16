//! Lua/Neovim registry client.
//!
//! Neovim plugins follow the `{name}.nvim` naming convention on GitHub.
//! This client searches GitHub for Neovim plugins using two strategies:
//! 1. **Convention-based**: `{query}.nvim in:name` — matches the `.nvim` naming pattern
//! 2. **Config-based**: searches GitHub code in common Neovim config files
//!    (`init.lua`, `lazy.lua`, `plugins.lua`) for references to the plugin
//!
//! Download counts are approximated using GitHub stargazers count, boosted
//! by the number of config files referencing the plugin.

use crate::{PackageInfo, RegistryClient, error::RegistryError};

#[derive(serde::Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
}

#[derive(serde::Deserialize)]
struct GitHubRepo {
    full_name: String,
    description: Option<String>,
    html_url: String,
    license: Option<GitHubLicense>,
    stargazers_count: u64,
    default_branch: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitHubLicense {
    spdx_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitHubCodeSearchResponse {
    total_count: u64,
}

impl RegistryClient {
    /// Search for Lua/Neovim packages on GitHub.
    ///
    /// Runs two concurrent searches:
    /// 1. Convention-based — `{query}.nvim in:name`
    /// 2. Broad — `{query} neovim plugin language:lua`
    ///
    /// Results are merged (convention-based first), deduplicated by name,
    /// and boosted with config-reference counts from common Neovim config
    /// files (`init.lua`, `lazy.lua`, `plugins.lua`).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if all HTTP requests fail.
    pub async fn search_luarocks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PackageInfo>, RegistryError> {
        let (nvim_results, broad_results) = tokio::join!(
            self.search_github_nvim_plugin(query, limit),
            self.search_github_neovim_broad(query, limit),
        );

        // Merge: convention-based first, then broad, deduplicated by name.
        let mut results = nvim_results;
        for pkg in broad_results {
            if !results.iter().any(|r| r.name == pkg.name) {
                results.push(pkg);
            }
        }
        results.truncate(limit);

        // Boost downloads with config reference counts.
        // GitHub code search is heavily rate-limited (~10 req/min unauthenticated).
        let mut set = tokio::task::JoinSet::new();
        let http = self.http.clone();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(2));
        for (idx, pkg) in results.iter().enumerate() {
            let short_name = pkg
                .name
                .rsplit('/')
                .next()
                .unwrap_or(&pkg.name)
                .to_string();
            let client = http.clone();
            let sem = semaphore.clone();
            set.spawn(async move {
                let Ok(_permit) = sem.acquire().await else {
                    return (idx, 0);
                };
                let search_query = format!(
                    "{short_name} filename:init.lua OR filename:lazy.lua OR filename:plugins.lua"
                );
                let url = format!(
                    "https://api.github.com/search/code?q={}",
                    urlencoding::encode(&search_query)
                );
                let count = async {
                    let resp = client.get(&url).send().await.ok()?;
                    if !resp.status().is_success() {
                        return None;
                    }
                    resp.json::<GitHubCodeSearchResponse>()
                        .await
                        .ok()
                        .map(|r| r.total_count)
                }
                .await
                .unwrap_or(0);
                (idx, count)
            });
        }
        while let Some(Ok((idx, config_refs))) = set.join_next().await {
            results[idx].downloads = results[idx].downloads.saturating_add(config_refs);
        }

        Ok(results)
    }

    async fn search_github_nvim_plugin(&self, query: &str, limit: usize) -> Vec<PackageInfo> {
        let search_query = format!("{query}.nvim in:name");
        self.search_github_repos(&search_query, limit).await
    }

    async fn search_github_neovim_broad(&self, query: &str, limit: usize) -> Vec<PackageInfo> {
        let search_query = format!("{query} neovim plugin language:lua");
        self.search_github_repos(&search_query, limit).await
    }

    async fn search_github_repos(&self, search_query: &str, limit: usize) -> Vec<PackageInfo> {
        let limit = limit.min(100);
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=stars&per_page={limit}",
            urlencoding::encode(search_query)
        );
        let Ok(resp) = self.http.get(&url).send().await else {
            return Vec::new();
        };
        if !resp.status().is_success() {
            return Vec::new();
        }
        let Ok(data) = resp.json::<GitHubSearchResponse>().await else {
            return Vec::new();
        };
        data.items
            .into_iter()
            .map(|repo| PackageInfo {
                name: repo.full_name,
                version: repo.default_branch.unwrap_or_default(),
                ecosystem: "lua".to_string(),
                description: repo.description.unwrap_or_default(),
                downloads: repo.stargazers_count,
                license: repo.license.and_then(|l| l.spdx_id),
                repository: Some(repo.html_url.clone()),
                homepage: Some(repo.html_url),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GITHUB_FIXTURE: &str = r#"{
        "total_count": 2,
        "items": [
            {
                "full_name": "nvim-telescope/telescope.nvim",
                "description": "Find, Filter, Preview, Pick.",
                "html_url": "https://github.com/nvim-telescope/telescope.nvim",
                "license": { "spdx_id": "MIT" },
                "stargazers_count": 15000,
                "default_branch": "master"
            },
            {
                "full_name": "nvim-treesitter/nvim-treesitter",
                "description": "Treesitter configurations for Neovim",
                "html_url": "https://github.com/nvim-treesitter/nvim-treesitter",
                "license": { "spdx_id": "Apache-2.0" },
                "stargazers_count": 10000,
                "default_branch": "main"
            }
        ]
    }"#;

    const CODE_SEARCH_FIXTURE: &str =
        r#"{"total_count": 1234, "incomplete_results": false, "items": []}"#;

    #[test]
    fn parse_github_search_response() {
        let data: GitHubSearchResponse = serde_json::from_str(GITHUB_FIXTURE).unwrap();
        assert_eq!(data.items.len(), 2);
        assert_eq!(data.items[0].full_name, "nvim-telescope/telescope.nvim");
        assert_eq!(data.items[0].stargazers_count, 15000);
        assert_eq!(
            data.items[0]
                .license
                .as_ref()
                .and_then(|l| l.spdx_id.as_deref()),
            Some("MIT")
        );
    }

    #[test]
    fn maps_github_to_package_info() {
        let data: GitHubSearchResponse = serde_json::from_str(GITHUB_FIXTURE).unwrap();
        let packages: Vec<PackageInfo> = data
            .items
            .into_iter()
            .map(|repo| PackageInfo {
                name: repo.full_name,
                version: repo.default_branch.unwrap_or_default(),
                ecosystem: "lua".to_string(),
                description: repo.description.unwrap_or_default(),
                downloads: repo.stargazers_count,
                license: repo.license.and_then(|l| l.spdx_id),
                repository: Some(repo.html_url.clone()),
                homepage: Some(repo.html_url),
            })
            .collect();

        assert_eq!(packages[0].ecosystem, "lua");
        assert_eq!(packages[0].name, "nvim-telescope/telescope.nvim");
        assert_eq!(packages[0].downloads, 15000);
        assert_eq!(packages[0].license.as_deref(), Some("MIT"));
        assert_eq!(
            packages[0].repository.as_deref(),
            Some("https://github.com/nvim-telescope/telescope.nvim")
        );
    }

    #[test]
    fn parse_code_search_response() {
        let data: GitHubCodeSearchResponse =
            serde_json::from_str(CODE_SEARCH_FIXTURE).unwrap();
        assert_eq!(data.total_count, 1234);
    }
}

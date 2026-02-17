use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::Utc;
use serde::Serialize;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use toml::Value as TomlValue;
use zen_core::entities::ProjectDependency;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::OnboardArgs;
use crate::context::AppContext;
use crate::output::output;
use crate::pipeline::IndexingPipeline;

const REGISTRY_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Serialize)]
struct OnboardResponse {
    project: OnboardProject,
    dependencies: OnboardDeps,
}

#[derive(Debug, Serialize)]
struct OnboardProject {
    name: String,
    ecosystem: String,
    manifests_found: Vec<String>,
}

#[derive(Debug, Serialize)]
struct OnboardDeps {
    detected: usize,
    already_indexed: usize,
    newly_indexed: usize,
    failed: usize,
    failed_packages: Vec<String>,
}

/// Handle `znt onboard`.
pub async fn handle(
    args: &OnboardArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let root = args
        .root
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.project_root.clone());

    let manifests = detect_manifests(&root);
    let ecosystem = args
        .ecosystem
        .clone()
        .unwrap_or_else(|| detect_ecosystem(&manifests));
    let project_name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let dependencies = detect_dependencies(&root, &ecosystem, args.workspace)?;
    for dep in &dependencies {
        ctx.service.upsert_dependency(dep).await?;
    }

    let mut already_indexed = 0usize;
    let mut newly_indexed = 0usize;
    let mut failed = 0usize;
    let mut failed_packages = Vec::new();

    if !args.skip_indexing {
        for dep in &dependencies {
            match index_dependency(dep, ctx).await {
                Ok(IndexStatus::AlreadyIndexed) => {
                    already_indexed += 1;
                }
                Ok(IndexStatus::IndexedNow) => {
                    newly_indexed += 1;
                }
                Err(error) => {
                    tracing::warn!(package = %dep.name, %error, "onboard: dependency indexing failed");
                    failed += 1;
                    failed_packages.push(dep.name.clone());
                }
            }
        }
    }

    output(
        &OnboardResponse {
            project: OnboardProject {
                name: project_name,
                ecosystem,
                manifests_found: manifests,
            },
            dependencies: OnboardDeps {
                detected: dependencies.len(),
                already_indexed,
                newly_indexed,
                failed,
                failed_packages,
            },
        },
        flags.format,
    )
}

fn detect_manifests(root: &Path) -> Vec<String> {
    let candidates = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "requirements.txt",
        "mix.exs",
        "go.mod",
        "Gemfile",
        "composer.json",
    ];
    candidates
        .iter()
        .filter(|name| root.join(name).is_file())
        .map(|name| (*name).to_string())
        .collect()
}

fn detect_ecosystem(manifests: &[String]) -> String {
    if manifests.iter().any(|m| m == "Cargo.toml") {
        return "rust".to_string();
    }
    if manifests.iter().any(|m| m == "package.json") {
        return "npm".to_string();
    }
    if manifests
        .iter()
        .any(|m| m == "pyproject.toml" || m == "requirements.txt")
    {
        return "pypi".to_string();
    }
    if manifests.iter().any(|m| m == "go.mod") {
        return "go".to_string();
    }
    if manifests.iter().any(|m| m == "mix.exs") {
        return "hex".to_string();
    }
    "rust".to_string()
}

fn detect_dependencies(
    root: &Path,
    ecosystem: &str,
    workspace: bool,
) -> anyhow::Result<Vec<ProjectDependency>> {
    let deps = match ecosystem {
        "rust" => parse_cargo_dependencies(root, workspace)?,
        "npm" => parse_npm_dependencies(root)?,
        "pypi" => parse_python_dependencies(root)?,
        other => {
            bail!(
                "onboard: dependency detection is not implemented for ecosystem '{}'",
                other
            );
        }
    };

    Ok(deps
        .into_iter()
        .map(|(name, version, source)| ProjectDependency {
            ecosystem: ecosystem.to_string(),
            name,
            version,
            source,
            indexed: false,
            indexed_at: None,
        })
        .collect())
}

fn parse_cargo_dependencies(
    root: &Path,
    include_workspace: bool,
) -> anyhow::Result<Vec<(String, Option<String>, String)>> {
    let raw = fs::read_to_string(root.join("Cargo.toml"))?;
    let document: TomlValue = toml::from_str(&raw).context("failed to parse Cargo.toml")?;
    let mut out = Vec::new();

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = document.get(section).and_then(TomlValue::as_table) {
            for (name, value) in table {
                out.push((
                    name.clone(),
                    parse_cargo_version_value(value),
                    "Cargo.toml".to_string(),
                ));
            }
        }
    }

    if include_workspace
        && let Some(table) = document
            .get("workspace")
            .and_then(TomlValue::as_table)
            .and_then(|ws| ws.get("dependencies"))
            .and_then(TomlValue::as_table)
    {
        for (name, value) in table {
            out.push((
                name.clone(),
                parse_cargo_version_value(value),
                "Cargo.toml".to_string(),
            ));
        }
    }

    Ok(unique_deps(out))
}

fn parse_npm_dependencies(root: &Path) -> anyhow::Result<Vec<(String, Option<String>, String)>> {
    let raw = fs::read_to_string(root.join("package.json"))?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let mut out = Vec::new();

    for field in ["dependencies", "devDependencies"] {
        if let Some(obj) = value.get(field).and_then(serde_json::Value::as_object) {
            for (name, ver) in obj {
                out.push((
                    name.clone(),
                    ver.as_str().map(std::string::ToString::to_string),
                    "package.json".to_string(),
                ));
            }
        }
    }

    Ok(unique_deps(out))
}

fn parse_python_dependencies(root: &Path) -> anyhow::Result<Vec<(String, Option<String>, String)>> {
    let mut out = Vec::new();

    let requirements = root.join("requirements.txt");
    if requirements.is_file() {
        let raw = fs::read_to_string(&requirements)?;
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let (name, version) = parse_requirement_line(trimmed);
            if name.is_empty() {
                continue;
            }

            out.push((name, version, "requirements.txt".to_string()));
        }
    }

    let pyproject = root.join("pyproject.toml");
    if pyproject.is_file() {
        let raw = fs::read_to_string(&pyproject)?;
        for line in raw.lines() {
            let trimmed = line.trim();
            if !(trimmed.starts_with('"') && trimmed.ends_with('"')) {
                continue;
            }
            let dep = trimmed.trim_matches('"');
            let mut split = dep.split(['=', '<', '>', '!', '~']);
            let name = split.next().unwrap_or_default().trim();
            if !name.is_empty() && !name.eq_ignore_ascii_case("python") {
                out.push((name.to_string(), None, "pyproject.toml".to_string()));
            }
        }
    }

    Ok(unique_deps(out))
}

fn parse_cargo_version_value(value: &TomlValue) -> Option<String> {
    match value {
        TomlValue::String(version) => Some(version.clone()),
        TomlValue::Table(table) => table
            .get("version")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn parse_requirement_line(line: &str) -> (String, Option<String>) {
    let operators = ["==", "~=", "!=", ">=", "<=", ">", "<"];

    let base = line.split('#').next().unwrap_or_default().trim();
    let base = base.split(';').next().unwrap_or(base).trim();
    if base.is_empty() {
        return (String::new(), None);
    }

    let mut content = base;
    if let Some((before_extras, _)) = base.split_once('[') {
        content = before_extras.trim();
    }

    for op in operators {
        if let Some((name, version)) = content.split_once(op) {
            let parsed_name = name.trim().to_string();
            let parsed_version = version
                .split(',')
                .next()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string);
            return (parsed_name, parsed_version);
        }
    }

    (content.trim().to_string(), None)
}

fn unique_deps(
    deps: Vec<(String, Option<String>, String)>,
) -> Vec<(String, Option<String>, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (name, version, source) in deps {
        if seen.insert(name.clone()) {
            out.push((name, version, source));
        }
    }
    out
}

enum IndexStatus {
    AlreadyIndexed,
    IndexedNow,
}

async fn index_dependency(
    dep: &ProjectDependency,
    ctx: &mut AppContext,
) -> anyhow::Result<IndexStatus> {
    let candidates = ctx
        .registry
        .search(&dep.name, &dep.ecosystem, REGISTRY_SEARCH_LIMIT)
        .await?;
    let exact = candidates
        .into_iter()
        .find(|pkg| pkg.name.eq_ignore_ascii_case(&dep.name))
        .ok_or_else(|| anyhow::anyhow!("onboard: no exact registry match for '{}'", dep.name))?;
    let version = dep.version.clone().unwrap_or_else(|| exact.version.clone());

    if ctx
        .lake
        .is_package_indexed(&dep.ecosystem, &dep.name, &version)?
    {
        return Ok(IndexStatus::AlreadyIndexed);
    }

    let repo_url = exact
        .repository
        .clone()
        .ok_or_else(|| anyhow::anyhow!("onboard: package '{}' has no repository URL", dep.name))?;

    let temp = tempfile::TempDir::new().context("failed to create temp directory")?;
    let clone_path = temp.path().join("repo");

    let clone = timeout(
        Duration::from_secs(120),
        TokioCommand::new("git")
            .args(["clone", "--depth", "1", &repo_url])
            .arg(&clone_path)
            .output(),
    )
    .await
    .context("onboard: git clone timed out")?
    .context("onboard: failed to run git clone")?;
    if !clone.status.success() {
        anyhow::bail!(
            "onboard: git clone failed for {}: {}",
            dep.name,
            String::from_utf8_lossy(&clone.stderr)
        );
    }

    let _ = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &clone_path,
        &dep.ecosystem,
        &dep.name,
        &version,
        &mut ctx.embedder,
        true,
    )?;

    ctx.service
        .upsert_dependency(&ProjectDependency {
            ecosystem: dep.ecosystem.clone(),
            name: dep.name.clone(),
            version: Some(version),
            source: dep.source.clone(),
            indexed: true,
            indexed_at: Some(Utc::now()),
        })
        .await?;

    Ok(IndexStatus::IndexedNow)
}

#[cfg(test)]
mod tests {
    use super::{parse_requirement_line, unique_deps};

    #[test]
    fn parses_requirements_equals() {
        let parsed = parse_requirement_line("tokio==1.40.0");
        assert_eq!(parsed.0, "tokio");
        assert_eq!(parsed.1.as_deref(), Some("1.40.0"));
    }

    #[test]
    fn parses_requirements_range_operator() {
        let parsed = parse_requirement_line("requests>=2.31");
        assert_eq!(parsed.0, "requests");
        assert_eq!(parsed.1.as_deref(), Some("2.31"));
    }

    #[test]
    fn deduplicates_dependencies_by_name() {
        let deduped = unique_deps(vec![
            (
                "tokio".to_string(),
                Some("1".to_string()),
                "Cargo.toml".to_string(),
            ),
            (
                "tokio".to_string(),
                Some("2".to_string()),
                "Cargo.toml".to_string(),
            ),
            (
                "serde".to_string(),
                Some("1".to_string()),
                "Cargo.toml".to_string(),
            ),
        ]);
        assert_eq!(deduped.len(), 2);
        assert!(deduped.iter().any(|(n, _, _)| n == "tokio"));
        assert!(deduped.iter().any(|(n, _, _)| n == "serde"));
    }
}

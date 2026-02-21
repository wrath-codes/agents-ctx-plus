use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::Utc;
use futures::stream::{self, StreamExt};
use semver::Version;
use serde::Serialize;
use tokio::process::Command as TokioCommand;
use tokio::task;
use tokio::time::timeout;
use toml::Value as TomlValue;
use zen_core::entities::ProjectDependency;
use zen_core::enums::Visibility;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::OnboardArgs;
use crate::context::AppContext;
use crate::context::CacheLookup;
use crate::output::output;
use crate::pipeline::IndexingPipeline;
use crate::progress::Progress;

const REGISTRY_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Serialize)]
struct OnboardResponse {
    project: OnboardProject,
    dependencies: OnboardDeps,
    hooks: OnboardHooks,
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
    already_indexed_local: usize,
    already_indexed_cloud: usize,
    skipped_by_catalog: usize,
    newly_indexed: usize,
    indexed_now: usize,
    failed: usize,
    failed_packages: Vec<String>,
    mode: String,
    catalog_hits: Vec<CatalogHit>,
}

#[derive(Debug, Serialize)]
struct CatalogHit {
    ecosystem: String,
    package: String,
    version: String,
    source: String,
}

#[derive(Debug, Clone)]
struct ResolvedDependency {
    version: String,
    repository: Option<String>,
}

enum PrecheckOutcome {
    SkipCatalog {
        dep: ProjectDependency,
        resolved: ResolvedDependency,
        source: &'static str,
    },
    Index {
        dep: ProjectDependency,
        resolved: ResolvedDependency,
    },
    Failed {
        dep: ProjectDependency,
        error: String,
    },
}

#[derive(Debug, Serialize)]
struct OnboardHooks {
    installed: bool,
    prompted: bool,
    note: String,
    next_step: Option<String>,
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
    let dep_progress = Progress::bar(
        u64::try_from(dependencies.len()).unwrap_or(0),
        "onboard: indexing dependencies",
    );
    for dep in &dependencies {
        ctx.service.upsert_dependency(dep).await?;
    }

    let mut already_indexed = 0usize;
    let mut already_indexed_local = 0usize;
    let mut already_indexed_cloud = 0usize;
    let mut skipped_by_catalog = 0usize;
    let mut newly_indexed = 0usize;
    let mut failed = 0usize;
    let mut failed_packages = Vec::new();
    let mut catalog_hits = Vec::new();
    let cloud_enabled = cloud_search_ready(ctx);

    if !args.skip_indexing {
        let jobs = args.jobs.max(1);
        let jobs_index = args.jobs_index.max(1);
        if jobs_index > 1 {
            tracing::warn!(
                requested = jobs_index,
                "onboard: jobs-index > 1 requested, but index/write phase remains sequential to avoid write contention"
            );
        }
        let ctx_ref: &AppContext = ctx;
        let prechecked = stream::iter(dependencies.iter().cloned())
            .map(|dep| async move { precheck_dependency(dep, cloud_enabled, ctx_ref).await })
            .buffered(jobs)
            .collect::<Vec<_>>()
            .await;

        let mut to_index = Vec::new();

        for outcome in prechecked {
            match outcome {
                PrecheckOutcome::SkipCatalog {
                    dep,
                    resolved,
                    source,
                } => {
                    dep_progress.set_message(&format!("{}:{}", dep.ecosystem, dep.name));
                    already_indexed += 1;
                    already_indexed_cloud += 1;
                    skipped_by_catalog += 1;
                    catalog_hits.push(CatalogHit {
                        ecosystem: dep.ecosystem,
                        package: dep.name,
                        version: resolved.version,
                        source: source.to_string(),
                    });
                    dep_progress.inc(1);
                    continue;
                }
                PrecheckOutcome::Failed { dep, error } => {
                    dep_progress.set_message(&format!("{}:{}", dep.ecosystem, dep.name));
                    tracing::warn!(package = %dep.name, error = %error, "onboard: dependency precheck failed");
                    failed += 1;
                    failed_packages.push(dep.name);
                    dep_progress.inc(1);
                    continue;
                }
                PrecheckOutcome::Index { dep, resolved } => {
                    to_index.push((dep, resolved));
                }
            }
        }

        for (dep, resolved) in to_index {
            dep_progress.set_message(&format!("{}:{}", dep.ecosystem, dep.name));
            match index_dependency(&dep, &resolved, ctx).await {
                Ok(IndexStatus::AlreadyIndexed) => {
                    already_indexed += 1;
                    already_indexed_local += 1;
                    dep_progress.inc(1);
                }
                Ok(IndexStatus::IndexedNow) => {
                    newly_indexed += 1;
                    dep_progress.inc(1);
                }
                Err(error) => {
                    tracing::warn!(package = %dep.name, %error, "onboard: dependency indexing failed");
                    failed += 1;
                    failed_packages.push(dep.name.clone());
                    dep_progress.inc(1);
                }
            }
        }
    }

    dep_progress.finish_ok("onboard: dependency pass complete");

    let hooks = if args.install_hooks {
        let report = zen_hooks::install_hooks(&root, zen_hooks::HookInstallStrategy::Chain)?;
        OnboardHooks {
            installed: report.installed,
            prompted: false,
            note: "hooks installed via --install-hooks".to_string(),
            next_step: if report.installed {
                None
            } else {
                Some("run: znt hook status".to_string())
            },
        }
    } else {
        match zen_hooks::status_hooks(&root) {
            Ok(status) if status.installation.health == "ok" => OnboardHooks {
                installed: true,
                prompted: false,
                note: "hooks already installed".to_string(),
                next_step: None,
            },
            Ok(_status) => {
                let mut installed = false;
                let mut prompted = false;
                if io::stdin().is_terminal() && io::stdout().is_terminal() {
                    prompted = true;
                    if prompt_yes_no("Zenith hooks are not fully installed. Install now? [y/N] ") {
                        let report =
                            zen_hooks::install_hooks(&root, zen_hooks::HookInstallStrategy::Chain)?;
                        installed = report.installed;
                    }
                }

                OnboardHooks {
                    installed,
                    prompted,
                    note: if installed {
                        "hooks installed during onboarding".to_string()
                    } else {
                        "hooks not installed".to_string()
                    },
                    next_step: if installed {
                        None
                    } else {
                        Some("run: znt hook install --strategy chain".to_string())
                    },
                }
            }
            Err(_) => OnboardHooks {
                installed: false,
                prompted: false,
                note: "no git repository detected for hook installation".to_string(),
                next_step: Some(
                    "initialize git, then run: znt hook install --strategy chain".to_string(),
                ),
            },
        }
    };

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
                already_indexed_local,
                already_indexed_cloud,
                skipped_by_catalog,
                newly_indexed,
                indexed_now: newly_indexed,
                failed,
                failed_packages,
                mode: onboard_mode(cloud_enabled, already_indexed_local, skipped_by_catalog)
                    .to_string(),
                catalog_hits,
            },
            hooks,
        },
        flags.format,
    )
}

fn onboard_mode(
    cloud_enabled: bool,
    already_indexed_local: usize,
    skipped_by_catalog: usize,
) -> &'static str {
    if !cloud_enabled {
        return "local";
    }
    if skipped_by_catalog > 0 && already_indexed_local > 0 {
        return "hybrid";
    }
    if skipped_by_catalog > 0 {
        return "cloud";
    }
    "local"
}

fn cloud_search_ready(ctx: &AppContext) -> bool {
    if !ctx.config.turso.is_configured() {
        return false;
    }
    if ctx.config.r2.is_configured() {
        return true;
    }
    std::env::var("AWS_ACCESS_KEY_ID").is_ok()
        && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok()
        && std::env::var("AWS_ENDPOINT_URL").is_ok()
}

fn prompt_yes_no(prompt: &str) -> bool {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
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
                if let Some((resolved_name, version)) = parse_cargo_dependency_entry(name, value) {
                    out.push((resolved_name, version, "Cargo.toml".to_string()));
                }
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
            if let Some((resolved_name, version)) = parse_cargo_dependency_entry(name, value) {
                out.push((resolved_name, version, "Cargo.toml".to_string()));
            }
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

fn parse_cargo_dependency_entry(name: &str, value: &TomlValue) -> Option<(String, Option<String>)> {
    match value {
        TomlValue::String(version) => Some((name.to_string(), Some(version.clone()))),
        TomlValue::Table(table) => {
            if table
                .get("workspace")
                .and_then(TomlValue::as_bool)
                .unwrap_or(false)
            {
                return None;
            }

            if table.get("path").is_some() || table.get("git").is_some() {
                return None;
            }

            let resolved_name = table
                .get("package")
                .and_then(TomlValue::as_str)
                .unwrap_or(name)
                .to_string();

            Some((resolved_name, parse_cargo_version_value(value)))
        }
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
    resolved: &ResolvedDependency,
    ctx: &mut AppContext,
) -> anyhow::Result<IndexStatus> {
    let version = resolved.version.clone();

    if ctx
        .lake
        .is_package_indexed(&dep.ecosystem, &dep.name, &version)?
    {
        return Ok(IndexStatus::AlreadyIndexed);
    }

    let temp = tempfile::TempDir::new().context("failed to create temp directory")?;
    let source_path = if dep.ecosystem == "rust" {
        let unpack_path = temp.path().join("crate");
        match fetch_crates_io_source(&dep.name, &version, &unpack_path).await {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!(
                    package = %dep.name,
                    version = %version,
                    %error,
                    "onboard: crates.io source download failed; falling back to git clone"
                );
                let repo_url = resolved.repository.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("onboard: package '{}' has no repository URL", dep.name)
                })?;
                clone_repo(repo_url, temp.path()).await?
            }
        }
    } else {
        let repo_url = resolved.repository.as_deref().ok_or_else(|| {
            anyhow::anyhow!("onboard: package '{}' has no repository URL", dep.name)
        })?;
        clone_repo(repo_url, temp.path()).await?
    };

    let _ = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &source_path,
        &dep.ecosystem,
        &dep.name,
        &version,
        &mut ctx.embedder,
        true,
        dep.ecosystem == "rust",
    )?;

    ctx.service
        .upsert_dependency(&ProjectDependency {
            ecosystem: dep.ecosystem.clone(),
            name: dep.name.clone(),
            version: Some(version.clone()),
            source: dep.source.clone(),
            indexed: true,
            indexed_at: Some(Utc::now()),
        })
        .await?;

    if ctx.config.turso.is_configured() && ctx.config.r2.is_configured() {
        match ctx
            .lake
            .write_to_r2(
                &ctx.config.r2,
                &dep.ecosystem,
                &dep.name,
                &version,
                Visibility::Public,
            )
            .await
        {
            Ok(export) => {
                if let Some(symbols_path) = export.symbols_lance_path.as_deref()
                    && let Err(error) = ctx
                        .service
                        .register_catalog_data_file(
                            &dep.ecosystem,
                            &dep.name,
                            &version,
                            symbols_path,
                            Visibility::Public,
                            None,
                            ctx.identity.as_ref().map(|i| i.user_id.as_str()),
                        )
                        .await
                {
                    tracing::warn!(
                        package = %dep.name,
                        version = %version,
                        %error,
                        "onboard: failed to register R2 symbols dataset in Turso catalog"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    package = %dep.name,
                    version = %version,
                    %error,
                    "onboard: failed to export package to R2"
                );
            }
        }
    }

    Ok(IndexStatus::IndexedNow)
}

async fn resolve_dependency(
    dep: &ProjectDependency,
    ctx: &AppContext,
) -> anyhow::Result<ResolvedDependency> {
    let candidates = ctx
        .registry
        .search(&dep.name, &dep.ecosystem, REGISTRY_SEARCH_LIMIT)
        .await?;
    let exact = candidates
        .into_iter()
        .find(|pkg| pkg.name.eq_ignore_ascii_case(&dep.name))
        .ok_or_else(|| anyhow::anyhow!("onboard: no exact registry match for '{}'", dep.name))?;

    Ok(ResolvedDependency {
        version: resolved_dependency_version(dep, &exact.version),
        repository: exact.repository,
    })
}

fn resolved_dependency_version(dep: &ProjectDependency, registry_version: &str) -> String {
    match dep.ecosystem.as_str() {
        "rust" => dep
            .version
            .as_deref()
            .map(str::trim)
            .map(|v| v.trim_start_matches('=').trim())
            .filter(|v| Version::parse(v).is_ok())
            .unwrap_or(registry_version)
            .to_string(),
        _ => dep
            .version
            .clone()
            .unwrap_or_else(|| registry_version.to_string()),
    }
}

async fn precheck_dependency(
    dep: ProjectDependency,
    cloud_enabled: bool,
    ctx: &AppContext,
) -> PrecheckOutcome {
    let resolved = match resolve_dependency(&dep, ctx).await {
        Ok(resolved) => resolved,
        Err(error) => {
            return PrecheckOutcome::Failed {
                dep,
                error: error.to_string(),
            };
        }
    };

    if !cloud_enabled {
        return PrecheckOutcome::Index { dep, resolved };
    }

    let version = resolved.version.as_str();
    let mut stale_paths = Vec::new();

    if let Some(cache) = &ctx.catalog_cache {
        match cache
            .get_paths(&dep.ecosystem, &dep.name, Some(version), "public")
            .await
        {
            Ok(CacheLookup::Fresh(paths)) if !paths.is_empty() => {
                return PrecheckOutcome::SkipCatalog {
                    dep,
                    resolved,
                    source: "catalog_cache",
                };
            }
            Ok(CacheLookup::Stale(paths)) => stale_paths = paths,
            Ok(CacheLookup::Miss) => {}
            Err(error) => {
                tracing::warn!(
                    package = %dep.name,
                    ecosystem = %dep.ecosystem,
                    %error,
                    "onboard: cache lookup failed; continuing with remote catalog"
                );
            }
            _ => {}
        }
    }

    match ctx
        .service
        .catalog_check_before_index(&dep.ecosystem, &dep.name, version)
        .await
    {
        Ok(Some(paths)) if !paths.is_empty() => {
            if let Some(cache) = &ctx.catalog_cache
                && let Err(error) = cache
                    .put_paths(&dep.ecosystem, &dep.name, Some(version), "public", &paths)
                    .await
            {
                tracing::warn!(
                    package = %dep.name,
                    ecosystem = %dep.ecosystem,
                    %error,
                    "onboard: failed to write catalog cache"
                );
            }

            PrecheckOutcome::SkipCatalog {
                dep,
                resolved,
                source: "turso_catalog",
            }
        }
        Ok(_) => PrecheckOutcome::Index { dep, resolved },
        Err(error) => {
            tracing::warn!(
                package = %dep.name,
                ecosystem = %dep.ecosystem,
                %error,
                "onboard: catalog lookup failed; continuing with local indexing"
            );
            if !stale_paths.is_empty() {
                PrecheckOutcome::SkipCatalog {
                    dep,
                    resolved,
                    source: "catalog_cache_stale",
                }
            } else {
                PrecheckOutcome::Index { dep, resolved }
            }
        }
    }
}

async fn clone_repo(repo_url: &str, temp_root: &Path) -> anyhow::Result<PathBuf> {
    let clone_path = temp_root.join("repo");
    let clone = timeout(
        Duration::from_secs(120),
        TokioCommand::new("git")
            .args(["clone", "--depth", "1", repo_url])
            .arg(&clone_path)
            .output(),
    )
    .await
    .context("onboard: git clone timed out")?
    .context("onboard: failed to run git clone")?;
    if !clone.status.success() {
        anyhow::bail!(
            "onboard: git clone failed: {}",
            String::from_utf8_lossy(&clone.stderr)
        );
    }
    Ok(clone_path)
}

async fn fetch_crates_io_source(
    crate_name: &str,
    version: &str,
    unpack_dir: &Path,
) -> anyhow::Result<PathBuf> {
    if let Some(local_archive) = find_local_cargo_crate_archive(crate_name, version) {
        return unpack_crate_archive(local_archive, unpack_dir.to_path_buf()).await;
    }

    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}/download",
        crate_name, version
    );

    let response = timeout(
        Duration::from_secs(120),
        reqwest::Client::new()
            .get(&url)
            .header(reqwest::header::USER_AGENT, "zenith-cli/0.1")
            .send(),
    )
    .await
    .context("onboard: crates.io source download timed out")?
    .context("onboard: crates.io source request failed")?
    .error_for_status()
    .context("onboard: crates.io source download returned error status")?;

    let bytes = response
        .bytes()
        .await
        .context("onboard: failed to read crates.io source body")?
        .to_vec();

    unpack_crate_bytes(bytes, unpack_dir.to_path_buf()).await
}

fn find_local_cargo_crate_archive(crate_name: &str, version: &str) -> Option<PathBuf> {
    let cargo_home = std::env::var("CARGO_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".cargo")))?;
    let cache_root = cargo_home.join("registry").join("cache");
    let file_name = format!("{crate_name}-{version}.crate");

    let dirs = fs::read_dir(cache_root).ok()?;
    for entry in dirs.flatten() {
        let path = entry.path().join(&file_name);
        if path.is_file() {
            return Some(path);
        }
    }

    None
}

async fn unpack_crate_archive(
    archive_path: PathBuf,
    unpack_root: PathBuf,
) -> anyhow::Result<PathBuf> {
    task::spawn_blocking(move || -> anyhow::Result<PathBuf> {
        let bytes = fs::read(&archive_path).with_context(|| {
            format!(
                "onboard: failed to read crate archive {}",
                archive_path.display()
            )
        })?;
        unpack_crate_bytes_sync(bytes, unpack_root)
    })
    .await
    .context("onboard: join error while unpacking local crate archive")?
}

async fn unpack_crate_bytes(bytes: Vec<u8>, unpack_root: PathBuf) -> anyhow::Result<PathBuf> {
    task::spawn_blocking(move || unpack_crate_bytes_sync(bytes, unpack_root))
        .await
        .context("onboard: join error while unpacking crates.io source")?
}

fn unpack_crate_bytes_sync(bytes: Vec<u8>, unpack_root: PathBuf) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(&unpack_root).context("onboard: failed to create unpack dir")?;
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&unpack_root)
        .context("onboard: failed to unpack crates.io source archive")?;

    let mut child_dirs = Vec::new();
    for entry in fs::read_dir(&unpack_root).context("onboard: failed to inspect unpack dir")? {
        let entry = entry.context("onboard: failed to read unpack dir entry")?;
        if entry
            .file_type()
            .context("onboard: failed to read unpack dir entry type")?
            .is_dir()
        {
            child_dirs.push(entry.path());
        }
    }

    if child_dirs.len() == 1 {
        Ok(child_dirs.remove(0))
    } else {
        Ok(unpack_root)
    }
}

#[cfg(test)]
mod tests {
    use super::{onboard_mode, parse_cargo_dependencies, parse_requirement_line, unique_deps};
    use tempfile::TempDir;

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

    #[test]
    fn onboard_mode_matrix() {
        assert_eq!(onboard_mode(false, 0, 0), "local");
        assert_eq!(onboard_mode(true, 0, 0), "local");
        assert_eq!(onboard_mode(true, 0, 3), "cloud");
        assert_eq!(onboard_mode(true, 2, 3), "hybrid");
    }

    #[test]
    fn cargo_parser_skips_path_and_workspace_dependencies() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::write(
            temp.path().join("Cargo.toml"),
            r#"
                [dependencies]
                anyhow = "1"
                aether-config = { path = "../aether-config" }
                serde_json = { workspace = true }
            "#,
        )
        .expect("Cargo.toml should write");

        let deps = parse_cargo_dependencies(temp.path(), true).expect("dependencies should parse");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "anyhow");
    }

    #[test]
    fn cargo_parser_uses_package_name_for_renamed_dependencies() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::write(
            temp.path().join("Cargo.toml"),
            r#"
                [dependencies]
                serde1 = { package = "serde", version = "1" }
            "#,
        )
        .expect("Cargo.toml should write");

        let deps = parse_cargo_dependencies(temp.path(), false).expect("dependencies should parse");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "serde");
    }
}

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::Utc;
use serde::Serialize;
use tokio::task;
use zen_core::entities::ProjectDependency;
use zen_core::enums::{SessionStatus, Visibility};

use crate::cli::GlobalFlags;
use crate::cli::root_commands::InstallArgs;
use crate::context::AppContext;
use crate::context::CacheLookup;
use crate::output::output;
use crate::pipeline::IndexingPipeline;
use crate::progress::Progress;

const REGISTRY_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Serialize)]
struct InstallResponse {
    package: InstallPackage,
    indexing: InstallIndexing,
}

#[derive(Debug, Serialize)]
struct InstallPackage {
    ecosystem: String,
    package: String,
    version: String,
    repo_url: String,
}

#[derive(Debug, Serialize)]
struct InstallIndexing {
    files_parsed: i32,
    symbols_extracted: i32,
    doc_chunks_created: i32,
    source_files_cached: i32,
    skipped: bool,
}

/// Handle `znt install`.
pub async fn handle(
    args: &InstallArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let progress = Progress::bar(5, "install: resolving package metadata");

    let ecosystem = if let Some(eco) = &args.ecosystem {
        eco.clone()
    } else {
        ctx.service
            .get_meta("ecosystem")
            .await?
            .unwrap_or_else(|| "rust".to_string())
    };

    let resolved = resolve_registry_package(ctx, &ecosystem, &args.package).await?;
    progress.inc(1);
    progress.set_message("install: preparing repository checkout");
    let version = args
        .version
        .clone()
        .unwrap_or_else(|| resolved.version.clone());

    if ctx
        .lake
        .is_package_indexed(&ecosystem, &args.package, &version)?
        && !args.force
    {
        progress.finish_ok("install: already indexed (local)");
        return output(
            &InstallResponse {
                package: InstallPackage {
                    ecosystem,
                    package: args.package.clone(),
                    version,
                    repo_url: resolved
                        .repository
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                },
                indexing: InstallIndexing {
                    files_parsed: 0,
                    symbols_extracted: 0,
                    doc_chunks_created: 0,
                    source_files_cached: 0,
                    skipped: true,
                },
            },
            flags.format,
        );
    }

    let repo_url = resolved
        .repository
        .clone()
        .unwrap_or_else(|| format!("https://crates.io/crates/{}", args.package));

    // Crowdsource dedup: skip clone + indexing if this public package already exists in the catalog.
    if ctx.service.is_synced_replica() && !args.force {
        let mut stale_paths = Vec::new();
        if let Some(cache) = &ctx.catalog_cache {
            match cache
                .get_paths(&ecosystem, &args.package, Some(&version), "public")
                .await
            {
                Ok(CacheLookup::Fresh(paths)) if !paths.is_empty() => {
                    progress.finish_ok("install: already indexed (catalog cache)");
                    return output(
                        &InstallResponse {
                            package: InstallPackage {
                                ecosystem,
                                package: args.package.clone(),
                                version,
                                repo_url,
                            },
                            indexing: InstallIndexing {
                                files_parsed: 0,
                                symbols_extracted: 0,
                                doc_chunks_created: 0,
                                source_files_cached: 0,
                                skipped: true,
                            },
                        },
                        flags.format,
                    );
                }
                Ok(CacheLookup::Stale(paths)) => stale_paths = paths,
                Ok(CacheLookup::Miss) => {}
                Err(error) => tracing::warn!(%error, "install: cache lookup failed"),
                _ => {}
            }
        }

        let remote_paths = match ctx
            .service
            .catalog_check_before_index(&ecosystem, &args.package, &version)
            .await
        {
            Ok(paths) => paths,
            Err(error) => {
                tracing::warn!(%error, "install: catalog lookup failed");
                if stale_paths.is_empty() {
                    None
                } else {
                    Some(stale_paths)
                }
            }
        };

        if let Some(existing_paths) = remote_paths {
            tracing::info!(
                package = %args.package,
                version = %version,
                paths = ?existing_paths,
                "install: package already indexed in cloud catalog; skipping re-index"
            );

            if let Some(cache) = &ctx.catalog_cache
                && let Err(error) = cache
                    .put_paths(
                        &ecosystem,
                        &args.package,
                        Some(&version),
                        "public",
                        &existing_paths,
                    )
                    .await
            {
                tracing::warn!(%error, "install: failed to update local catalog cache");
            }

            let dep = ProjectDependency {
                ecosystem: ecosystem.clone(),
                name: args.package.clone(),
                version: Some(version.clone()),
                source: "registry".to_string(),
                indexed: true,
                indexed_at: Some(Utc::now()),
            };
            ctx.service.upsert_dependency(&dep).await?;

            progress.finish_ok("install: already indexed (catalog)");
            return output(
                &InstallResponse {
                    package: InstallPackage {
                        ecosystem,
                        package: args.package.clone(),
                        version,
                        repo_url: repo_url.clone(),
                    },
                    indexing: InstallIndexing {
                        files_parsed: 0,
                        symbols_extracted: 0,
                        doc_chunks_created: 0,
                        source_files_cached: 0,
                        skipped: true,
                    },
                },
                flags.format,
            );
        }
    }

    if args.force {
        if let Err(error) = ctx.lake.delete_package(&ecosystem, &args.package, &version) {
            tracing::warn!(
                ecosystem = %ecosystem,
                package = %args.package,
                version = %version,
                %error,
                "install: failed to delete existing lake package before force reinstall"
            );
        }
        if let Err(error) =
            ctx.source_store
                .delete_package_sources(&ecosystem, &args.package, &version)
        {
            tracing::warn!(
                ecosystem = %ecosystem,
                package = %args.package,
                version = %version,
                %error,
                "install: failed to delete existing source cache before force reinstall"
            );
        }
    }

    let temp = tempfile::TempDir::new().context("failed to create temp directory")?;
    let clone_path = temp.path().join("repo");
    progress.set_message("install: indexing repository sources");

    let source_path = if ecosystem == "rust" {
        let unpack_path = temp.path().join("crate");
        match fetch_crates_io_source(&args.package, &version, &unpack_path).await {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!(
                    package = %args.package,
                    version = %version,
                    %error,
                    "install: crates.io source download failed; falling back to git clone"
                );
                let repository = resolved.repository.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "install: package '{}' has no repository URL and crates.io source fetch failed",
                        args.package
                    )
                })?;
                run_git_clone(repository, &clone_path, false)?;
                let checkout_ref = args.version.clone().unwrap_or_else(|| version.clone());
                let checked_out = try_git_checkout_for_version(&clone_path, &checkout_ref)?;
                if !checked_out {
                    tracing::warn!(
                        package = %args.package,
                        version = %checkout_ref,
                        "install: unable to checkout resolved version ref; proceeding with repository default branch"
                    );
                }
                clone_path.clone()
            }
        }
    } else {
        let repository = resolved.repository.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "install: registry package '{}' has no repository URL",
                args.package
            )
        })?;
        run_git_clone(repository, &clone_path, false)?;
        let checkout_ref = args.version.clone().unwrap_or_else(|| version.clone());
        let checked_out = try_git_checkout_for_version(&clone_path, &checkout_ref)?;
        if !checked_out {
            tracing::warn!(
                package = %args.package,
                version = %checkout_ref,
                "install: unable to checkout resolved version ref; proceeding with repository default branch"
            );
        }
        clone_path.clone()
    };
    progress.inc(1);

    progress.set_message("install: indexing package sources");
    let index = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &source_path,
        &ecosystem,
        &args.package,
        &version,
        &mut ctx.embedder,
        !args.include_tests,
        ecosystem == "rust",
    )
    .context("indexing pipeline failed")?;
    progress.inc(1);
    progress.set_message("install: updating dependency metadata");

    let dep = ProjectDependency {
        ecosystem: ecosystem.clone(),
        name: args.package.clone(),
        version: Some(version.clone()),
        source: "registry".to_string(),
        indexed: true,
        indexed_at: Some(Utc::now()),
    };
    ctx.service.upsert_dependency(&dep).await?;
    progress.inc(1);

    progress.set_message("install: syncing cloud catalog");
    if ctx.config.turso.is_configured() && ctx.config.r2.is_configured() {
        progress.set_message("install: exporting indexed package to catalog");
        match ctx
            .lake
            .write_to_r2(
                &ctx.config.r2,
                &ecosystem,
                &args.package,
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
                            &ecosystem,
                            &args.package,
                            &version,
                            symbols_path,
                            Visibility::Public,
                            None,
                            ctx.identity.as_ref().map(|i| i.user_id.as_str()),
                        )
                        .await
                {
                    tracing::warn!(
                        package = %args.package,
                        version = %version,
                        %error,
                        "install: failed to register R2 symbols dataset in Turso catalog"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    package = %args.package,
                    version = %version,
                    %error,
                    "install: failed to export package to R2"
                );
            }
        }
    }

    if let Some(session) = ctx
        .service
        .list_sessions(Some(SessionStatus::Active), 1)
        .await?
        .first()
        .cloned()
        && let Err(error) = crate::workspace::agentfs::record_install_event(
            &ctx.project_root,
            &session.id,
            &ecosystem,
            &args.package,
            &version,
            true,
            None,
        )
        .await
    {
        tracing::warn!(
            session = %session.id,
            package = %args.package,
            %error,
            "install: failed to write workspace audit event"
        );
    }

    progress.inc(1);

    progress.finish_ok("install: completed");

    output(
        &InstallResponse {
            package: InstallPackage {
                ecosystem,
                package: args.package.clone(),
                version,
                repo_url,
            },
            indexing: InstallIndexing {
                files_parsed: index.file_count,
                symbols_extracted: index.symbol_count,
                doc_chunks_created: index.doc_chunk_count,
                source_files_cached: index.source_file_count,
                skipped: false,
            },
        },
        flags.format,
    )
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

    let response = tokio::time::timeout(
        Duration::from_secs(120),
        reqwest::Client::new()
            .get(&url)
            .header(reqwest::header::USER_AGENT, "zenith-cli/0.1")
            .send(),
    )
    .await
    .context("install: crates.io source download timed out")?
    .context("install: crates.io source request failed")?
    .error_for_status()
    .context("install: crates.io source download returned error status")?;

    let bytes = response
        .bytes()
        .await
        .context("install: failed to read crates.io source body")?
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
                "install: failed to read crate archive {}",
                archive_path.display()
            )
        })?;
        unpack_crate_bytes_sync(bytes, unpack_root)
    })
    .await
    .context("install: join error while unpacking local crate archive")?
}

async fn unpack_crate_bytes(bytes: Vec<u8>, unpack_root: PathBuf) -> anyhow::Result<PathBuf> {
    task::spawn_blocking(move || unpack_crate_bytes_sync(bytes, unpack_root))
        .await
        .context("install: join error while unpacking crates.io source")?
}

fn unpack_crate_bytes_sync(bytes: Vec<u8>, unpack_root: PathBuf) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(&unpack_root).context("install: failed to create unpack dir")?;
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&unpack_root)
        .context("install: failed to unpack crates.io source archive")?;

    let mut child_dirs = Vec::new();
    for entry in fs::read_dir(&unpack_root).context("install: failed to inspect unpack dir")? {
        let entry = entry.context("install: failed to read unpack dir entry")?;
        if entry
            .file_type()
            .context("install: failed to read unpack dir entry type")?
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

async fn resolve_registry_package(
    ctx: &AppContext,
    ecosystem: &str,
    package: &str,
) -> anyhow::Result<zen_registry::PackageInfo> {
    let candidates = ctx
        .registry
        .search(package, ecosystem, REGISTRY_SEARCH_LIMIT)
        .await?;
    let exact = candidates
        .into_iter()
        .find(|pkg| pkg.name.eq_ignore_ascii_case(package));
    exact.ok_or_else(|| {
        anyhow::anyhow!(
            "install: no exact registry match for package '{}' in ecosystem '{}'; use exact package name",
            package,
            ecosystem
        )
    })
}

fn try_git_checkout_for_version(
    repo_path: &std::path::Path,
    version: &str,
) -> anyhow::Result<bool> {
    if run_git_checkout(repo_path, version).is_ok() {
        return Ok(true);
    }

    let prefixed = format!("v{version}");
    if run_git_checkout(repo_path, &prefixed).is_ok() {
        return Ok(true);
    }

    Ok(false)
}

fn run_git_clone(
    repo_url: &str,
    clone_path: &std::path::Path,
    shallow: bool,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    if shallow {
        cmd.args(["--depth", "1"]);
    }
    let output = cmd
        .arg(repo_url)
        .arg(clone_path)
        .output()
        .context("failed to spawn git clone")?;
    if !output.status.success() {
        bail!(
            "install: git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn run_git_checkout(repo_path: &std::path::Path, revision: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["checkout", revision])
        .output()
        .context("failed to spawn git checkout")?;
    if !output.status.success() {
        bail!(
            "install: git checkout {} failed: {}",
            revision,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

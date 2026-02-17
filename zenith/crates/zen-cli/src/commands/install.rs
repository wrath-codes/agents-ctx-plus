use std::process::Command;

use anyhow::{Context, bail};
use chrono::Utc;
use serde::Serialize;
use zen_core::entities::ProjectDependency;
use zen_core::enums::SessionStatus;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::InstallArgs;
use crate::context::AppContext;
use crate::output::output;
use crate::pipeline::IndexingPipeline;

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
    let ecosystem = if let Some(eco) = &args.ecosystem {
        eco.clone()
    } else {
        ctx.service
            .get_meta("ecosystem")
            .await?
            .unwrap_or_else(|| "rust".to_string())
    };

    let resolved = resolve_registry_package(ctx, &ecosystem, &args.package).await?;
    let version = args
        .version
        .clone()
        .unwrap_or_else(|| resolved.version.clone());

    if ctx
        .lake
        .is_package_indexed(&ecosystem, &args.package, &version)?
        && !args.force
    {
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

    let repo_url = resolved.repository.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "install: registry package '{}' has no repository URL",
            args.package
        )
    })?;

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

    run_git_clone(&repo_url, &clone_path, false)?;
    let checkout_ref = args.version.clone().unwrap_or_else(|| version.clone());
    let checked_out = try_git_checkout_for_version(&clone_path, &checkout_ref)?;
    if !checked_out {
        tracing::warn!(
            package = %args.package,
            version = %checkout_ref,
            "install: unable to checkout resolved version ref; proceeding with repository default branch"
        );
    }

    let index = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &clone_path,
        &ecosystem,
        &args.package,
        &version,
        &mut ctx.embedder,
        !args.include_tests,
    )
    .context("indexing pipeline failed")?;

    let dep = ProjectDependency {
        ecosystem: ecosystem.clone(),
        name: args.package.clone(),
        version: Some(version.clone()),
        source: "registry".to_string(),
        indexed: true,
        indexed_at: Some(Utc::now()),
    };
    ctx.service.upsert_dependency(&dep).await?;

    if ctx.config.turso.is_configured() && ctx.config.r2.is_configured() {
        match ctx
            .lake
            .write_to_r2(&ctx.config.r2, &ecosystem, &args.package, &version)
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

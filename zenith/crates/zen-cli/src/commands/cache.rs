use anyhow::bail;
use duckdb::params;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::CacheCommands;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct CachePackageRow {
    ecosystem: String,
    package: String,
    version: String,
    file_count: i64,
    size_bytes: i64,
}

#[derive(Debug, Serialize)]
struct CacheListResponse {
    packages: Vec<CachePackageRow>,
    total_size_bytes: i64,
    total_packages: usize,
}

#[derive(Debug, Serialize)]
struct CacheStatsResponse {
    total_packages: usize,
    total_size_bytes: i64,
}

#[derive(Debug, Serialize)]
struct CacheCleanResponse {
    removed_packages: usize,
    removed_sources: usize,
    scope: String,
}

/// Handle `znt cache`.
pub async fn handle(
    action: &CacheCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        CacheCommands::List => {
            let mut packages = Vec::new();
            let mut total_size_bytes = 0i64;

            for (ecosystem, package, version) in ctx.lake.list_indexed_packages()? {
                let (file_count, size_bytes) =
                    source_stats_for(ctx, &ecosystem, &package, &version)?;
                total_size_bytes += size_bytes;
                packages.push(CachePackageRow {
                    ecosystem,
                    package,
                    version,
                    file_count,
                    size_bytes,
                });
            }

            output(
                &CacheListResponse {
                    total_packages: packages.len(),
                    total_size_bytes,
                    packages,
                },
                flags.format,
            )
        }
        CacheCommands::Stats => {
            let total_packages = ctx.lake.count_indexed_packages()?;
            let total_size_bytes = total_source_size(ctx)?;
            output(
                &CacheStatsResponse {
                    total_packages,
                    total_size_bytes,
                },
                flags.format,
            )
        }
        CacheCommands::Clean {
            package,
            ecosystem,
            version,
            all,
        } => {
            if *all {
                let removed_packages = ctx.lake.count_indexed_packages()?;
                let removed_sources = count_source_files(ctx)?;
                ctx.lake.clear()?;
                ctx.source_store.clear()?;
                return output(
                    &CacheCleanResponse {
                        removed_packages,
                        removed_sources,
                        scope: "all".to_string(),
                    },
                    flags.format,
                );
            }

            let package = package
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("provide --all or --package <name>"))?;
            let ecosystem = ecosystem.as_deref().unwrap_or("rust");

            let versions = if let Some(v) = version {
                vec![v.clone()]
            } else {
                let mut found = ctx
                    .lake
                    .list_indexed_packages()?
                    .into_iter()
                    .filter(|(eco, pkg, _)| eco == ecosystem && pkg == package)
                    .map(|(_, _, ver)| ver)
                    .collect::<Vec<_>>();
                found.sort();
                found
            };

            if versions.is_empty() {
                bail!("no indexed package found for {ecosystem}/{package}");
            }

            let mut removed_sources = 0usize;
            for v in &versions {
                let (file_count, _) = source_stats_for(ctx, ecosystem, package, v)?;
                removed_sources = removed_sources.saturating_add(usize::try_from(file_count)?);
                ctx.lake.delete_package(ecosystem, package, v)?;
                ctx.source_store
                    .delete_package_sources(ecosystem, package, v)?;
            }

            output(
                &CacheCleanResponse {
                    removed_packages: versions.len(),
                    removed_sources,
                    scope: format!("{ecosystem}/{package}"),
                },
                flags.format,
            )
        }
    }
}

fn source_stats_for(
    ctx: &AppContext,
    ecosystem: &str,
    package: &str,
    version: &str,
) -> anyhow::Result<(i64, i64)> {
    let conn = ctx.source_store.conn();
    let mut stmt = conn.prepare(
        "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0)
         FROM source_files WHERE ecosystem = ? AND package = ? AND version = ?",
    )?;
    let result = stmt.query_row(params![ecosystem, package, version], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    Ok(result)
}

fn total_source_size(ctx: &AppContext) -> anyhow::Result<i64> {
    let conn = ctx.source_store.conn();
    let mut stmt = conn.prepare("SELECT COALESCE(SUM(size_bytes), 0) FROM source_files")?;
    stmt.query_row([], |row| row.get(0)).map_err(Into::into)
}

fn count_source_files(ctx: &AppContext) -> anyhow::Result<usize> {
    let conn = ctx.source_store.conn();
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM source_files")?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;
    usize::try_from(count).map_err(|_| anyhow::anyhow!("source file count overflow"))
}

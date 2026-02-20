use std::path::Path;

use anyhow::{Context, bail};
use semver::Version;
use serde::Serialize;
use zen_search::{
    RecursiveBudget, RecursiveQuery, RecursiveQueryEngine, SearchEngine, SearchFilters, SearchMode,
    SearchResult, VectorSearchResult, VectorSource,
};

use crate::cli::GlobalFlags;
use crate::cli::root_commands::SearchArgs;
use crate::commands::shared::limit::effective_limit;
use crate::context::AppContext;
use crate::context::CacheLookup;
use crate::output::output;

#[derive(Debug, Serialize)]
struct SearchResponse {
    query: String,
    mode: String,
    fetched_results: usize,
    returned: usize,
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
struct RecursiveResponse {
    query: String,
    mode: String,
    returned: usize,
    results: Vec<zen_search::SymbolRefHit>,
    ref_graph: Option<RecursiveRefGraph>,
    budget_used: zen_search::BudgetUsed,
}

#[derive(Debug, Serialize)]
struct RecursiveRefGraph {
    categories: std::collections::HashMap<String, usize>,
    summary_json: Option<String>,
    summary_json_pretty: Option<String>,
}

/// Handle `znt search`.
pub async fn handle(
    args: &SearchArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let mode = parse_mode(args.mode.as_deref())?;
    let limit = effective_limit(None, flags.limit, 20);

    if matches!(mode, SearchMode::Recursive) {
        return handle_recursive(args, ctx, flags, limit).await;
    }

    if let Some(cloud_results) = try_cloud_vector_search(args, ctx, mode, limit).await? {
        let fetched_results = cloud_results.len();
        output(
            &SearchResponse {
                query: args.query.clone(),
                mode: "vector".to_string(),
                fetched_results,
                returned: cloud_results.len(),
                results: cloud_results,
            },
            flags.format,
        )?;
        return Ok(());
    }

    let mut engine = SearchEngine::new(
        &ctx.service,
        &ctx.lake,
        &ctx.source_store,
        &mut ctx.embedder,
    );

    let filters = SearchFilters {
        package: args.package.clone(),
        ecosystem: args.ecosystem.clone(),
        version: args.version.clone(),
        kind: args.kind.clone(),
        entity_types: Vec::new(),
        limit: Some(limit),
        min_score: None,
    };

    let mut results = engine.search(&args.query, mode, filters).await?;
    let fetched_results = results.len();
    results.truncate(usize::try_from(limit)?);

    output(
        &SearchResponse {
            query: args.query.clone(),
            mode: mode_name(mode).to_string(),
            fetched_results,
            returned: results.len(),
            results,
        },
        flags.format,
    )
}

async fn handle_recursive(
    args: &SearchArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
    limit: u32,
) -> anyhow::Result<()> {
    let budget = RecursiveBudget {
        max_depth: usize::try_from(args.max_depth.unwrap_or(2))?,
        max_chunks: usize::try_from(args.context_budget.or(args.max_chunks).unwrap_or(200))?,
        max_bytes_per_chunk: usize::try_from(args.max_bytes_per_chunk.unwrap_or(6_000))?,
        max_total_bytes: usize::try_from(args.max_total_bytes.unwrap_or(750_000))?,
    };

    let mut query = RecursiveQuery::from_text(&args.query);
    query.generate_summary = true;

    let result = if let Some((eco, pkg, version)) = resolve_triplet(args, &ctx.lake)? {
        let engine = RecursiveQueryEngine::from_source_store(
            &ctx.source_store,
            &eco,
            &pkg,
            &version,
            budget,
        )
        .context("failed to initialize recursive search engine from package source store")?;
        engine.execute(&query)?
    } else {
        let engine = RecursiveQueryEngine::from_directory(Path::new(&ctx.project_root), budget)
            .context("failed to initialize recursive search engine from project directory")?;
        engine.execute(&query)?
    };

    let mut hits = result.hits;
    hits.truncate(usize::try_from(limit)?);

    let (summary_json, summary_json_pretty) = pretty_summary(result.summary_json);
    let ref_graph = if args.show_ref_graph {
        Some(RecursiveRefGraph {
            categories: result.category_counts,
            summary_json,
            summary_json_pretty,
        })
    } else {
        None
    };

    output(
        &RecursiveResponse {
            query: args.query.clone(),
            mode: "recursive".to_string(),
            returned: hits.len(),
            results: hits,
            ref_graph,
            budget_used: result.budget_used,
        },
        flags.format,
    )
}

fn parse_mode(raw: Option<&str>) -> anyhow::Result<SearchMode> {
    match raw.unwrap_or("hybrid") {
        "vector" => Ok(SearchMode::Vector),
        "fts" => Ok(SearchMode::Fts),
        "hybrid" => Ok(SearchMode::Hybrid { alpha: 0.5 }),
        "recursive" => Ok(SearchMode::Recursive),
        "graph" => Ok(SearchMode::Graph),
        other => {
            bail!(
                "search: invalid --mode '{other}'; expected one of: vector, fts, hybrid, recursive, graph"
            )
        }
    }
}

const fn mode_name(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Vector => "vector",
        SearchMode::Fts => "fts",
        SearchMode::Hybrid { .. } => "hybrid",
        SearchMode::Recursive => "recursive",
        SearchMode::Graph => "graph",
    }
}

fn resolve_triplet(
    args: &SearchArgs,
    lake: &zen_lake::ZenLake,
) -> anyhow::Result<Option<(String, String, String)>> {
    match (&args.ecosystem, &args.package, &args.version) {
        (Some(eco), Some(pkg), Some(version)) => {
            Ok(Some((eco.clone(), pkg.clone(), version.clone())))
        }
        (Some(eco), Some(pkg), None) => {
            let mut versions = lake
                .list_indexed_packages()?
                .into_iter()
                .filter(|(e, p, _)| e == eco && p == pkg)
                .map(|(_, _, v)| v)
                .collect::<Vec<_>>();
            versions.sort_by(semver_or_lexicographic_cmp);
            Ok(versions
                .pop()
                .map(|version| (eco.clone(), pkg.clone(), version)))
        }
        _ => Ok(None),
    }
}

fn semver_or_lexicographic_cmp(a: &String, b: &String) -> std::cmp::Ordering {
    match (Version::parse(a), Version::parse(b)) {
        (Ok(av), Ok(bv)) => av.cmp(&bv),
        _ => a.cmp(b),
    }
}

fn pretty_summary(summary_json: Option<String>) -> (Option<String>, Option<String>) {
    match summary_json {
        Some(raw) => {
            let pretty = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| serde_json::to_string_pretty(&v).ok());
            (Some(raw), pretty)
        }
        None => (None, None),
    }
}

async fn try_cloud_vector_search(
    args: &SearchArgs,
    ctx: &mut AppContext,
    mode: SearchMode,
    limit: u32,
) -> anyhow::Result<Option<Vec<SearchResult>>> {
    if !matches!(mode, SearchMode::Vector) || !ctx.config.turso.is_configured() {
        return Ok(None);
    }

    let (ecosystem, package) = match (&args.ecosystem, &args.package) {
        (Some(eco), Some(pkg)) => (eco, pkg),
        _ => return Ok(None),
    };

    let query_embedding = ctx.embedder.embed_single(&args.query)?;
    let scope = cache_scope_key(ctx.identity.as_ref());

    let mut stale_paths = Vec::new();
    if let Some(cache) = &ctx.catalog_cache {
        match cache
            .get_paths(ecosystem, package, args.version.as_deref(), &scope)
            .await
        {
            Ok(CacheLookup::Fresh(paths)) if !paths.is_empty() => {
                let paths = canonical_lance_locators(paths);
                if paths.is_empty() {
                    tracing::debug!(
                        ecosystem = %ecosystem,
                        package = %package,
                        "search: cache had no canonical table-uri paths; falling back to remote lookup"
                    );
                } else {
                    match map_cloud_paths_to_search_results(
                        &ctx.lake,
                        ctx.config.r2.is_configured().then_some(&ctx.config.r2),
                        ecosystem,
                        package,
                        &paths,
                        &query_embedding,
                        limit,
                    )
                    .await
                    {
                        Ok(mapped) => {
                            if mapped.is_empty() {
                                tracing::warn!(
                                    ecosystem = %ecosystem,
                                    package = %package,
                                    "search: cached lance paths returned no results; refreshing from remote catalog"
                                );
                                stale_paths = paths;
                            } else {
                                return Ok(Some(mapped));
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                %error,
                                ecosystem = %ecosystem,
                                package = %package,
                                "search: cached lance paths failed; falling back to remote catalog"
                            );
                            stale_paths = paths;
                        }
                    }
                }
            }
            Ok(CacheLookup::Stale(paths)) => stale_paths = canonical_lance_locators(paths),
            Ok(CacheLookup::Miss) => {}
            Err(error) => tracing::warn!(%error, "search: cache lookup failed"),
            _ => {}
        }
    }

    let paths = match ctx
        .service
        .catalog_paths_for_package_scoped(ecosystem, package, args.version.as_deref())
        .await
    {
        Ok(paths) => paths,
        Err(error) => {
            if stale_paths.is_empty() {
                return Err(anyhow::anyhow!(
                    "search: cloud catalog lookup failed and no cache available: {error}"
                ));
            }

            tracing::warn!(%error, "search: cloud lookup failed; using stale catalog cache");
            stale_paths
        }
    };

    let paths = canonical_lance_locators(paths);

    if paths.is_empty() {
        return Ok(Some(Vec::new()));
    }

    if let Some(cache) = &ctx.catalog_cache
        && let Err(error) = cache
            .put_paths(ecosystem, package, args.version.as_deref(), &scope, &paths)
            .await
    {
        tracing::warn!(%error, "search: failed to update local catalog cache");
    }

    let mapped = map_cloud_paths_to_search_results(
        &ctx.lake,
        ctx.config.r2.is_configured().then_some(&ctx.config.r2),
        ecosystem,
        package,
        &paths,
        &query_embedding,
        limit,
    )
    .await?;
    Ok(Some(mapped))
}

async fn map_cloud_paths_to_search_results(
    lake: &zen_lake::ZenLake,
    r2_config: Option<&zen_config::R2Config>,
    ecosystem: &str,
    package: &str,
    paths: &[String],
    query_embedding: &[f32],
    limit: u32,
) -> anyhow::Result<Vec<SearchResult>> {
    let cloud = lake
        .search_lance_paths_with_r2(paths, query_embedding, limit, r2_config)
        .await
        .map_err(|error| anyhow::anyhow!("search: native Lance query failed: {error}"))?;

    if cloud.is_empty() {
        return Ok(Vec::new());
    }

    let mapped = cloud
        .into_iter()
        .map(|item| {
            SearchResult::Vector(VectorSearchResult {
                id: item.id,
                ecosystem: ecosystem.to_string(),
                package: package.to_string(),
                version: item.version,
                kind: item.kind,
                name: item.name,
                signature: item.signature,
                doc_comment: item.doc_comment,
                file_path: item.file_path.unwrap_or_default(),
                line_start: None,
                line_end: None,
                score: -item.distance,
                source_type: VectorSource::ApiSymbol,
            })
        })
        .collect::<Vec<_>>();

    Ok(mapped)
}

fn cache_scope_key(identity: Option<&zen_core::identity::AuthIdentity>) -> String {
    match identity {
        Some(identity) => {
            if let Some(org_id) = &identity.org_id {
                return format!("org:{org_id}:user:{}", identity.user_id);
            }
            format!("user:{}", identity.user_id)
        }
        None => "anonymous".to_string(),
    }
}

fn canonical_lance_locators(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|path| path.contains(".lance") && !path.contains('#'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_mode, pretty_summary};

    #[test]
    fn parse_mode_defaults_to_hybrid() {
        let mode = parse_mode(None).expect("mode should parse");
        assert!(matches!(mode, zen_search::SearchMode::Hybrid { .. }));
    }

    #[test]
    fn parse_mode_rejects_invalid_value() {
        let err = parse_mode(Some("nope")).expect_err("invalid mode should fail");
        assert!(err.to_string().contains("search: invalid --mode"));
    }

    #[test]
    fn pretty_summary_builds_pretty_variant() {
        let (raw, pretty) = pretty_summary(Some("{\"a\":1}".to_string()));
        assert_eq!(raw.as_deref(), Some("{\"a\":1}"));
        assert!(pretty.as_deref().is_some_and(|v| v.contains("\n")));
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::{Context, bail};
use semver::Version;
use serde::Serialize;
use zen_search::{GrepEngine, GrepOptions};

use crate::cli::GlobalFlags;
use crate::cli::root_commands::GrepArgs;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct CountEntry {
    path: String,
    count: u64,
}

#[derive(Debug, Serialize)]
struct CountResponse {
    counts: Vec<CountEntry>,
    stats: zen_search::GrepStats,
}

#[derive(Debug, Serialize)]
struct FilesResponse {
    files: Vec<String>,
    stats: zen_search::GrepStats,
}

/// Handle `znt grep`.
pub async fn handle(
    args: &GrepArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let opts = GrepOptions {
        case_insensitive: args.ignore_case,
        smart_case: args.smart_case,
        fixed_strings: args.fixed_strings,
        word_regexp: args.word_regexp,
        multiline: false,
        context_before: args.context.unwrap_or(2),
        context_after: args.context.unwrap_or(2),
        include_glob: args.include.clone(),
        exclude_glob: args.exclude.clone(),
        max_count: args.max_count,
        skip_tests: args.skip_tests,
        no_symbols: args.no_symbols,
    };

    let mode_count = usize::from(args.all_packages)
        + usize::from(!args.packages.is_empty())
        + usize::from(!args.paths.is_empty());
    if mode_count == 0 {
        bail!("grep: choose one target mode: local paths, --package/-P, or --all-packages");
    }
    if mode_count > 1 {
        bail!(
            "grep: target modes are mutually exclusive; choose only one of local paths, --package/-P, or --all-packages"
        );
    }
    if args.count && args.files_with_matches {
        bail!("grep: --count and --files-with-matches cannot be used together");
    }

    let mut result = if args.all_packages || !args.packages.is_empty() {
        let packages = resolve_package_targets(args, ctx)?;
        GrepEngine::grep_package(
            &ctx.source_store,
            &ctx.lake,
            &args.pattern,
            &packages,
            &opts,
        )?
    } else {
        let roots = args
            .paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<PathBuf>>();
        GrepEngine::grep_local(&args.pattern, &roots, &opts)?
    };

    if let Some(limit) = flags.limit {
        result.matches.truncate(usize::try_from(limit)?);
    }

    if args.count {
        let mut by_file: BTreeMap<String, u64> = BTreeMap::new();
        for m in result.matches {
            *by_file.entry(m.path).or_default() += 1;
        }
        let counts = by_file
            .into_iter()
            .map(|(path, count)| CountEntry { path, count })
            .collect::<Vec<_>>();
        return output(
            &CountResponse {
                counts,
                stats: result.stats,
            },
            flags.format,
        );
    }

    if args.files_with_matches {
        let files = result
            .matches
            .into_iter()
            .map(|m| m.path)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        return output(
            &FilesResponse {
                files,
                stats: result.stats,
            },
            flags.format,
        );
    }

    output(&result, flags.format)
}

fn resolve_package_targets(
    args: &GrepArgs,
    ctx: &AppContext,
) -> anyhow::Result<Vec<(String, String, String)>> {
    let indexed = ctx
        .lake
        .list_indexed_packages()
        .context("failed to list indexed packages")?;

    if args.all_packages {
        if indexed.is_empty() {
            bail!("grep: no indexed packages found; run 'znt install <package>' first");
        }
        return Ok(indexed);
    }

    let ecosystem = args.ecosystem.clone().unwrap_or_else(|| "rust".to_string());

    let mut targets = Vec::new();
    for package in &args.packages {
        if let Some(version) = &args.version {
            if indexed
                .iter()
                .any(|(e, p, v)| e == &ecosystem && p == package && v == version)
            {
                targets.push((ecosystem.clone(), package.clone(), version.clone()));
                continue;
            }
            bail!(
                "grep: indexed package not found for --package {} --ecosystem {} --version {}",
                package,
                ecosystem,
                version
            );
        }

        let mut versions = indexed
            .iter()
            .filter(|(e, p, _)| e == &ecosystem && p == package)
            .map(|(_, _, v)| v.clone())
            .collect::<Vec<_>>();
        versions.sort_by(semver_or_lexicographic_cmp);
        let version = versions.pop().ok_or_else(|| {
            anyhow::anyhow!(
                "grep: indexed package not found for --package {} --ecosystem {}; run 'znt install {} --ecosystem {}' first",
                package,
                ecosystem,
                package,
                ecosystem
            )
        })?;
        targets.push((ecosystem.clone(), package.clone(), version));
    }

    Ok(targets)
}

fn semver_or_lexicographic_cmp(a: &String, b: &String) -> std::cmp::Ordering {
    match (Version::parse(a), Version::parse(b)) {
        (Ok(av), Ok(bv)) => av.cmp(&bv),
        _ => a.cmp(b),
    }
}

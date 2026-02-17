use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct RegistryResponse {
    results: Vec<zen_registry::PackageInfo>,
    query: String,
    ecosystem: String,
}

pub async fn run(
    query: &str,
    ecosystem: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = usize::try_from(effective_limit(limit, flags.limit, 10))?;

    let (results, ecosystem_label) = if let Some(ecosystem) = ecosystem {
        (
            ctx.registry.search(query, ecosystem, limit).await?,
            ecosystem.to_string(),
        )
    } else {
        // `search_all` intentionally returns `Vec<PackageInfo>` (not `Result`) and logs per-registry failures.
        (
            ctx.registry.search_all(query, limit).await,
            String::from("all"),
        )
    };

    output(
        &RegistryResponse {
            results,
            query: query.to_string(),
            ecosystem: ecosystem_label,
        },
        flags.format,
    )
}

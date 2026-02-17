use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(query: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let entries = fetch(query, ctx, flags).await?;
    output(&entries, flags.format)
}

pub async fn fetch(
    query: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<Vec<zen_core::entities::AuditEntry>> {
    let limit = effective_limit(None, flags.limit, 50);
    ctx.service
        .search_audit(query, limit)
        .await
        .map_err(Into::into)
}

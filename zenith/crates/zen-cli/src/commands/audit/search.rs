use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(query: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let limit = effective_limit(None, flags.limit, 50);
    let entries = ctx.service.search_audit(query, limit).await?;
    output(&entries, flags.format)
}

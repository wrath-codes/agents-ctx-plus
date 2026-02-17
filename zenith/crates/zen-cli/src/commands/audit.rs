#[path = "audit/query.rs"]
mod query;
#[path = "audit/search.rs"]
mod search;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::AuditArgs;
use crate::context::AppContext;

/// Handle `znt audit`.
pub async fn handle(
    args: &AuditArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    if let Some(search_query) = args.search.as_deref() {
        search::run(search_query, ctx, flags).await
    } else {
        query::run(args, ctx, flags).await
    }
}

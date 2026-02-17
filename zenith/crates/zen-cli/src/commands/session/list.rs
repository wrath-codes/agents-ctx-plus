use zen_core::enums::SessionStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let status = status
        .map(|value| parse_enum::<SessionStatus>(value, "status"))
        .transpose()?;
    let limit = effective_limit(limit, flags.limit, 20);
    let sessions = ctx.service.list_sessions(status, limit).await?;
    output(&sessions, flags.format)
}

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

use super::types::SessionStartResponse;

pub async fn run(ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let (session, orphaned) = ctx.service.start_session().await?;
    output(&SessionStartResponse { session, orphaned }, flags.format)
}

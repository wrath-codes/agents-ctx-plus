use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let compat = ctx.service.get_compat_by_id(id).await?;
    output(&compat, flags.format)
}

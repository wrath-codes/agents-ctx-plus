use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let insight = ctx.service.get_insight(id).await?;
    output(&insight, flags.format)
}

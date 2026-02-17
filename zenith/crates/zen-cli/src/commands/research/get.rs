use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let research = ctx.service.get_research(id).await?;
    output(&research, flags.format)
}

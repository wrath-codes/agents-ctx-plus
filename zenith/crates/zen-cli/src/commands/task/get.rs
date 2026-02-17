use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let task = ctx.service.get_task(id).await?;
    output(&task, flags.format)
}

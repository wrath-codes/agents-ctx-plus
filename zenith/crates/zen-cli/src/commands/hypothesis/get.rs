use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let hypothesis = ctx.service.get_hypothesis(id).await?;
    output(&hypothesis, flags.format)
}

use serde::Serialize;
use zen_core::entities::Finding;

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct FindingDetailResponse {
    finding: Finding,
    tags: Vec<String>,
}

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let finding = ctx.service.get_finding(id).await?;
    let tags = ctx.service.get_finding_tags(id).await?;
    output(&FindingDetailResponse { finding, tags }, flags.format)
}

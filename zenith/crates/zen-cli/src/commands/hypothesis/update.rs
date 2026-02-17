use zen_core::enums::HypothesisStatus;
use zen_db::updates::hypothesis::HypothesisUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    id: &str,
    content: Option<&str>,
    reason: Option<String>,
    status: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    if content.is_none() && reason.is_none() && status.is_none() {
        anyhow::bail!("At least one of --content, --reason, or --status must be provided");
    }

    let mut builder = HypothesisUpdateBuilder::new();
    if let Some(content) = content {
        builder = builder.content(content);
    }
    if let Some(reason) = reason {
        builder = builder.reason(Some(reason));
    }
    if let Some(status) = status {
        builder = builder.status(parse_enum::<HypothesisStatus>(status, "status")?);
    }

    let hypothesis = ctx
        .service
        .update_hypothesis(&session_id, id, builder.build())
        .await?;
    output(&hypothesis, flags.format)
}

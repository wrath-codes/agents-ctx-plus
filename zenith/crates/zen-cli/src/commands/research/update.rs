use zen_core::enums::ResearchStatus;
use zen_db::updates::research::ResearchUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    id: &str,
    title: Option<&str>,
    description: Option<String>,
    status: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    if title.is_none() && description.is_none() && status.is_none() {
        anyhow::bail!("At least one of --title, --description, or --status must be provided");
    }

    let mut builder = ResearchUpdateBuilder::new();
    if let Some(title) = title {
        builder = builder.title(title);
    }
    if let Some(description) = description {
        builder = builder.description(Some(description));
    }
    if let Some(status) = status {
        builder = builder.status(parse_enum::<ResearchStatus>(status, "status")?);
    }

    let updated = ctx
        .service
        .update_research(&session_id, id, builder.build())
        .await?;
    output(&updated, flags.format)
}

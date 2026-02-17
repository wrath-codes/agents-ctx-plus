use zen_core::enums::TaskStatus;
use zen_db::updates::task::TaskUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub struct Params {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub research: Option<String>,
    pub issue: Option<String>,
}

pub async fn run(params: Params, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    if params.title.is_none()
        && params.description.is_none()
        && params.status.is_none()
        && params.research.is_none()
        && params.issue.is_none()
    {
        anyhow::bail!(
            "At least one of --title, --description, --status, --research, or --issue must be provided"
        );
    }

    let mut builder = TaskUpdateBuilder::new();
    if let Some(title) = params.title.as_deref() {
        builder = builder.title(title);
    }
    if let Some(description) = params.description {
        builder = builder.description(Some(description));
    }
    if let Some(status) = params.status.as_deref() {
        builder = builder.status(parse_enum::<TaskStatus>(status, "status")?);
    }
    if let Some(research) = params.research {
        builder = builder.research_id(Some(research));
    }
    if let Some(issue) = params.issue {
        builder = builder.issue_id(Some(issue));
    }

    let task = ctx
        .service
        .update_task(&session_id, &params.id, builder.build())
        .await?;
    output(&task, flags.format)
}

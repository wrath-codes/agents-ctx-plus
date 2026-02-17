use zen_core::enums::IssueType;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    title: &str,
    issue_type: Option<&str>,
    priority: Option<u8>,
    description: Option<&str>,
    parent: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let issue_type = issue_type
        .map(|value| parse_enum::<IssueType>(value, "type"))
        .transpose()?
        .unwrap_or(IssueType::Feature);
    let priority = priority.unwrap_or(3);

    let issue = ctx
        .service
        .create_issue(
            &session_id,
            title,
            issue_type,
            priority,
            description,
            parent,
        )
        .await?;

    output(&issue, flags.format)
}

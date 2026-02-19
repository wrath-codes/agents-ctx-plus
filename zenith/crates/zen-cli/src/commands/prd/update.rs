use zen_core::enums::IssueType;
use zen_db::updates::issue::IssueUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    id: &str,
    content: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let issue = ctx.service.get_issue(id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!(
            "Issue '{id}' is not an epic (type: {}). PRD commands only work with epics.",
            issue.issue_type
        );
    }

    let update = IssueUpdateBuilder::new()
        .description(Some(content.to_string()))
        .build();
    let updated = ctx.service.update_issue(&session_id, id, update).await?;

    output(&updated, flags.format)
}

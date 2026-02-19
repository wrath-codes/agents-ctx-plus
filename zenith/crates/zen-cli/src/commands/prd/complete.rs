use zen_core::enums::{IssueStatus, IssueType};

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    let issue = ctx.service.get_issue(id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!(
            "Issue '{id}' is not an epic. Use 'znt issue update --status done' for non-epic issues."
        );
    }

    if issue.status == IssueStatus::Done {
        return output(&issue, flags.format);
    }

    if issue.status == IssueStatus::Abandoned {
        anyhow::bail!(
            "PRD '{id}' is abandoned and cannot be completed. Use 'znt issue update --status in_progress' to reopen first."
        );
    }

    if needs_in_progress_hop(issue.status) {
        ctx.service
            .transition_issue(&session_id, id, IssueStatus::InProgress)
            .await?;
    }

    let completed = ctx
        .service
        .transition_issue(&session_id, id, IssueStatus::Done)
        .await?;

    output(&completed, flags.format)
}

fn needs_in_progress_hop(status: IssueStatus) -> bool {
    status == IssueStatus::Open || status == IssueStatus::Blocked
}

#[cfg(test)]
mod tests {
    use zen_core::enums::IssueStatus;

    use super::needs_in_progress_hop;

    #[test]
    fn requires_in_progress_hop_for_open_and_blocked() {
        assert!(needs_in_progress_hop(IssueStatus::Open));
        assert!(needs_in_progress_hop(IssueStatus::Blocked));
        assert!(!needs_in_progress_hop(IssueStatus::InProgress));
        assert!(!needs_in_progress_hop(IssueStatus::Done));
        assert!(!needs_in_progress_hop(IssueStatus::Abandoned));
    }
}

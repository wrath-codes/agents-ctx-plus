use serde::Serialize;
use zen_core::entities::Issue;
use zen_core::enums::{IssueStatus, IssueType, TaskStatus};

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdListItem {
    #[serde(flatten)]
    issue: Issue,
    tasks_total: usize,
    tasks_done: usize,
}

pub async fn run(
    status: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit);

    let mut issues: Vec<Issue> = if let Some(query) = search {
        ctx.service.search_issues(query, fetch_limit).await?
    } else {
        ctx.service.list_issues(fetch_limit).await?
    };

    issues.retain(|issue| issue.issue_type == IssueType::Epic);

    if let Some(status) = status {
        let status = parse_enum::<IssueStatus>(status, "status")?;
        issues.retain(|issue| issue.status == status);
    }

    issues.truncate(usize::try_from(limit)?);

    let mut items = Vec::with_capacity(issues.len());
    for issue in issues {
        let tasks = ctx.service.get_tasks_for_issue(&issue.id).await?;
        let tasks_done = tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Done)
            .count();
        items.push(PrdListItem {
            issue,
            tasks_total: tasks.len(),
            tasks_done,
        });
    }

    output(&items, flags.format)
}

fn compute_fetch_limit(limit: u32) -> u32 {
    limit.saturating_mul(5).min(500)
}

#[cfg(test)]
mod tests {
    use super::compute_fetch_limit;

    #[test]
    fn always_boosts_fetch_limit_for_epic_filtering() {
        assert_eq!(compute_fetch_limit(20), 100);
        assert_eq!(compute_fetch_limit(120), 500);
    }
}

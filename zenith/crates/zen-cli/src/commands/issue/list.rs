use zen_core::entities::Issue;
use zen_core::enums::{IssueStatus, IssueType};

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    issue_type: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status, issue_type);

    let mut issues: Vec<Issue> = if let Some(query) = search {
        ctx.service.search_issues(query, fetch_limit).await?
    } else {
        ctx.service.list_issues(fetch_limit).await?
    };

    if let Some(status) = status {
        let status = parse_enum::<IssueStatus>(status, "status")?;
        issues.retain(|issue| issue.status == status);
    }
    if let Some(issue_type) = issue_type {
        let issue_type = parse_enum::<IssueType>(issue_type, "type")?;
        issues.retain(|issue| issue.issue_type == issue_type);
    }
    issues.truncate(usize::try_from(limit)?);

    output(&issues, flags.format)
}

fn compute_fetch_limit(limit: u32, status: Option<&str>, issue_type: Option<&str>) -> u32 {
    if status.is_some() || issue_type.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

use serde::Serialize;
use zen_core::entities::{Issue, Task};

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct IssueDetailResponse {
    issue: Issue,
    children: Vec<Issue>,
    tasks: Vec<Task>,
}

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let issue = ctx.service.get_issue(id).await?;
    let children = ctx.service.get_child_issues(id).await?;
    let tasks = ctx.service.get_tasks_for_issue(id).await?;

    output(
        &IssueDetailResponse {
            issue,
            children,
            tasks,
        },
        flags.format,
    )
}

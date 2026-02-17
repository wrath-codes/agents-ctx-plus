use zen_core::entities::Task;
use zen_core::enums::TaskStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    issue: Option<&str>,
    research: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status, issue, research);

    let mut tasks: Vec<Task> = if let Some(query) = search {
        ctx.service.search_tasks(query, fetch_limit).await?
    } else {
        ctx.service.list_tasks(fetch_limit).await?
    };

    if let Some(status) = status {
        let status = parse_enum::<TaskStatus>(status, "status")?;
        tasks.retain(|task| task.status == status);
    }
    if let Some(issue_id) = issue {
        tasks.retain(|task| task.issue_id.as_deref() == Some(issue_id));
    }
    if let Some(research_id) = research {
        tasks.retain(|task| task.research_id.as_deref() == Some(research_id));
    }
    tasks.truncate(usize::try_from(limit)?);

    output(&tasks, flags.format)
}

fn compute_fetch_limit(
    limit: u32,
    status: Option<&str>,
    issue: Option<&str>,
    research: Option<&str>,
) -> u32 {
    if status.is_some() || issue.is_some() || research.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

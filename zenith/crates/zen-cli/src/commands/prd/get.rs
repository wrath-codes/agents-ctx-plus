use serde::Serialize;
use zen_core::entities::{Finding, Hypothesis, Issue, Task};
use zen_core::enums::{EntityType, HypothesisStatus, IssueType, TaskStatus};

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdDetailResponse {
    prd: Issue,
    tasks: TaskProgress,
    findings: Vec<Finding>,
    open_questions: Vec<Hypothesis>,
}

#[derive(Debug, Serialize)]
struct TaskProgress {
    total: usize,
    done: usize,
    in_progress: usize,
    open: usize,
    blocked: usize,
    items: Vec<Task>,
}

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let prd = ctx.service.get_issue(id).await?;
    if prd.issue_type != IssueType::Epic {
        anyhow::bail!(
            "Issue '{id}' is not an epic (type: {}). Use 'znt issue get' for non-epic issues.",
            prd.issue_type
        );
    }

    let tasks = ctx.service.get_tasks_for_issue(id).await?;
    let progress = TaskProgress::from_tasks(tasks);

    let links = ctx.service.get_links_from(EntityType::Issue, id).await?;
    let mut findings = Vec::new();
    let mut open_questions = Vec::new();

    for link in links {
        match link.target_type {
            EntityType::Finding => match ctx.service.get_finding(&link.target_id).await {
                Ok(finding) => findings.push(finding),
                Err(error) => {
                    tracing::warn!(
                        target_id = %link.target_id,
                        "failed to fetch linked finding: {error}"
                    );
                }
            },
            EntityType::Hypothesis => match ctx.service.get_hypothesis(&link.target_id).await {
                Ok(hypothesis) if hypothesis.status == HypothesisStatus::Unverified => {
                    open_questions.push(hypothesis);
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(
                        target_id = %link.target_id,
                        "failed to fetch linked hypothesis: {error}"
                    );
                }
            },
            _ => {}
        }
    }

    output(
        &PrdDetailResponse {
            prd,
            tasks: progress,
            findings,
            open_questions,
        },
        flags.format,
    )
}

impl TaskProgress {
    fn from_tasks(items: Vec<Task>) -> Self {
        let done = items
            .iter()
            .filter(|task| task.status == TaskStatus::Done)
            .count();
        let in_progress = items
            .iter()
            .filter(|task| task.status == TaskStatus::InProgress)
            .count();
        let open = items
            .iter()
            .filter(|task| task.status == TaskStatus::Open)
            .count();
        let blocked = items
            .iter()
            .filter(|task| task.status == TaskStatus::Blocked)
            .count();

        Self {
            total: items.len(),
            done,
            in_progress,
            open,
            blocked,
            items,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Task;
    use zen_core::enums::TaskStatus;

    use super::TaskProgress;

    fn mk_task(id: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            issue_id: None,
            research_id: None,
            session_id: Some(String::from("ses-1")),
            title: format!("task-{id}"),
            description: None,
            status,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn aggregates_task_status_counts() {
        let tasks = vec![
            mk_task("1", TaskStatus::Open),
            mk_task("2", TaskStatus::InProgress),
            mk_task("3", TaskStatus::Done),
            mk_task("4", TaskStatus::Blocked),
            mk_task("5", TaskStatus::Done),
        ];

        let progress = TaskProgress::from_tasks(tasks);
        assert_eq!(progress.total, 5);
        assert_eq!(progress.open, 1);
        assert_eq!(progress.in_progress, 1);
        assert_eq!(progress.done, 2);
        assert_eq!(progress.blocked, 1);
    }
}

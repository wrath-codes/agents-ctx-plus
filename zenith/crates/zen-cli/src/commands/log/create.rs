use crate::cli::GlobalFlags;
use crate::cli::root_commands::LogArgs;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;
use zen_core::entities::Task;
use zen_core::enums::TaskStatus;

use super::parse_location::parse_location;

pub async fn run(args: &LogArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let task_id = resolve_task_id(args, &session_id, ctx).await?;
    let parsed = parse_location(&args.location)?;

    let log = ctx
        .service
        .create_impl_log(
            &session_id,
            &task_id,
            &parsed.file_path,
            parsed.start_line,
            parsed.end_line,
            args.description.as_deref(),
        )
        .await?;

    output(&log, flags.format)
}

async fn resolve_task_id(
    args: &LogArgs,
    session_id: &str,
    ctx: &AppContext,
) -> anyhow::Result<String> {
    if let Some(task_id) = args.task.as_deref() {
        return Ok(task_id.to_string());
    }

    let tasks = ctx.service.list_tasks(100).await?;
    select_default_task_id(&tasks, session_id)
}

fn select_default_task_id(tasks: &[Task], session_id: &str) -> anyhow::Result<String> {
    let mut candidates = tasks
        .iter()
        .filter(|task| task.session_id.as_deref() == Some(session_id))
        .filter(|task| matches!(task.status, TaskStatus::InProgress | TaskStatus::Open))
        .map(|task| task.id.clone());

    let first = candidates.next();
    let second = candidates.next();

    match (first, second) {
        (Some(task_id), None) => Ok(task_id),
        (Some(_), Some(_)) => Err(anyhow::anyhow!(
            "multiple active/open tasks found in this session; pass --task explicitly"
        )),
        _ => Err(anyhow::anyhow!(
            "--task is required unless exactly one open/in_progress task exists in the active session"
        )),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Task;
    use zen_core::enums::TaskStatus;

    use super::select_default_task_id;

    #[test]
    fn rejects_missing_task() {
        let tasks = Vec::<Task>::new();
        assert!(select_default_task_id(&tasks, "ses-1").is_err());
    }

    #[test]
    fn accepts_present_task() {
        let tasks = vec![mk_task("tsk-1", "ses-1", TaskStatus::InProgress)];
        assert_eq!(
            select_default_task_id(&tasks, "ses-1").expect("task should exist"),
            String::from("tsk-1")
        );
    }

    #[test]
    fn rejects_multiple_candidates() {
        let tasks = vec![
            mk_task("tsk-1", "ses-1", TaskStatus::InProgress),
            mk_task("tsk-2", "ses-1", TaskStatus::Open),
        ];
        assert!(select_default_task_id(&tasks, "ses-1").is_err());
    }

    fn mk_task(id: &str, session: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            research_id: None,
            issue_id: None,
            session_id: Some(session.to_string()),
            title: String::from("task"),
            description: None,
            status,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

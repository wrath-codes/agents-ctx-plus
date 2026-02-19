use serde::Serialize;
use zen_core::entities::Task;
use zen_core::enums::IssueType;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdTasksResponse {
    tasks: Vec<Task>,
    message: &'static str,
}

pub async fn run(
    epic_id: &str,
    tasks_json: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    let issue = ctx.service.get_issue(epic_id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!(
            "Issue '{epic_id}' is not an epic. PRD tasks can only be generated for epics."
        );
    }

    let titles = parse_task_titles(tasks_json)?;

    let mut tasks: Vec<Task> = Vec::with_capacity(titles.len());
    for title in &titles {
        match ctx
            .service
            .create_task(&session_id, title, None, Some(epic_id), None)
            .await
        {
            Ok(task) => tasks.push(task),
            Err(error) => {
                let created_ids = join_task_ids(&tasks);
                let rollback_error = rollback_tasks(ctx, &session_id, &tasks).await.err();
                if let Some(rollback_error) = rollback_error {
                    anyhow::bail!(
                        "Failed to create task '{title}': {error}. Created task IDs before rollback: [{}]. Rollback failed: {rollback_error}",
                        created_ids
                    );
                }

                anyhow::bail!(
                    "Failed to create task '{title}': {error}. Rolled back created task IDs: [{}]",
                    created_ids
                );
            }
        }
    }

    output(
        &PrdTasksResponse {
            tasks,
            message: "High-level tasks generated. Ask the user to confirm before generating sub-tasks.",
        },
        flags.format,
    )
}

pub(super) fn parse_task_titles(tasks_json: &str) -> anyhow::Result<Vec<String>> {
    let raw_titles: Vec<String> = serde_json::from_str(tasks_json).map_err(|error| {
        anyhow::anyhow!("Invalid --tasks JSON: {error}. Expected: '[\"title1\", \"title2\"]'")
    })?;

    if raw_titles.is_empty() {
        anyhow::bail!("--tasks array is empty. Provide at least one task title.");
    }

    let mut titles = Vec::with_capacity(raw_titles.len());
    for (index, title) in raw_titles.into_iter().enumerate() {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            anyhow::bail!(
                "--tasks entry at index {index} is empty. Provide non-empty task titles."
            );
        }
        titles.push(trimmed.to_string());
    }

    Ok(titles)
}

fn join_task_ids(tasks: &[Task]) -> String {
    tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

async fn rollback_tasks(ctx: &AppContext, session_id: &str, tasks: &[Task]) -> anyhow::Result<()> {
    let mut rollback_failures = Vec::new();
    for task in tasks.iter().rev() {
        if let Err(error) = ctx.service.delete_task(session_id, &task.id).await {
            rollback_failures.push(format!("{} ({error})", task.id));
        }
    }

    if rollback_failures.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "Unable to rollback all created tasks. Failed deletes: {}",
        rollback_failures.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::parse_task_titles;

    #[test]
    fn parses_non_empty_json_array() {
        let titles =
            parse_task_titles("[\"Build schema\",\"Add tests\"]").expect("json array should parse");
        assert_eq!(titles, vec!["Build schema", "Add tests"]);
    }

    #[test]
    fn trims_whitespace_in_titles() {
        let titles = parse_task_titles("[\"  Build schema  \",\"Add tests\"]")
            .expect("json array should parse");
        assert_eq!(titles, vec!["Build schema", "Add tests"]);
    }

    #[test]
    fn rejects_invalid_json() {
        let error = parse_task_titles("not-json").expect_err("invalid json should fail");
        assert!(error.to_string().contains("Invalid --tasks JSON"));
    }

    #[test]
    fn rejects_empty_array() {
        let error = parse_task_titles("[]").expect_err("empty array should fail");
        assert!(error.to_string().contains("--tasks array is empty"));
    }

    #[test]
    fn rejects_blank_title_entry() {
        let error = parse_task_titles("[\"ok\",\"   \"]").expect_err("blank title should fail");
        assert!(error.to_string().contains("entry at index 1 is empty"));
    }
}

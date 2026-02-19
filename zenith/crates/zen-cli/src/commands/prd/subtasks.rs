use serde::Serialize;
use zen_core::entities::Task;
use zen_core::enums::{EntityType, IssueType, Relation};

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdSubtasksResponse {
    subtasks: Vec<Task>,
    parent_task_id: String,
}

pub async fn run(
    parent_task_id: &str,
    epic_id: &str,
    tasks_json: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    let parent = ctx.service.get_task(parent_task_id).await?;
    let epic = ctx.service.get_issue(epic_id).await?;
    if epic.issue_type != IssueType::Epic {
        anyhow::bail!(
            "Issue '{epic_id}' is not an epic. Sub-tasks can only be created under epic issues."
        );
    }
    if parent.issue_id.as_deref() != Some(epic_id) {
        anyhow::bail!("Parent task '{parent_task_id}' is not linked to epic '{epic_id}'.");
    }

    let titles = super::tasks::parse_task_titles(tasks_json)?;

    let mut subtasks: Vec<Task> = Vec::with_capacity(titles.len());
    for title in &titles {
        match ctx
            .service
            .create_task(&session_id, title, None, Some(epic_id), None)
            .await
        {
            Ok(task) => subtasks.push(task),
            Err(error) => {
                let created_ids = join_task_ids(&subtasks);
                let empty_links: Vec<String> = Vec::new();
                let rollback_error = rollback_subtasks(ctx, &session_id, &subtasks, &empty_links)
                    .await
                    .err();
                if let Some(rollback_error) = rollback_error {
                    anyhow::bail!(
                        "Failed to create sub-task '{title}': {error}. Created sub-task IDs before rollback: [{}]. Rollback failed: {rollback_error}",
                        created_ids
                    );
                }

                anyhow::bail!(
                    "Failed to create sub-task '{title}': {error}. Rolled back created sub-task IDs: [{}]",
                    created_ids
                );
            }
        }
    }

    let mut created_link_ids = Vec::with_capacity(subtasks.len());
    for task in &subtasks {
        match ctx
            .service
            .create_link(
                &session_id,
                EntityType::Task,
                &task.id,
                EntityType::Task,
                parent_task_id,
                Relation::DependsOn,
            )
            .await
        {
            Ok(link) => created_link_ids.push(link.id),
            Err(error) => {
                let created_ids = join_task_ids(&subtasks);
                let rollback_error =
                    rollback_subtasks(ctx, &session_id, &subtasks, &created_link_ids)
                        .await
                        .err();
                if let Some(rollback_error) = rollback_error {
                    anyhow::bail!(
                        "Failed to create depends_on link for sub-task '{}' -> parent '{}': {error}. Created sub-task IDs before rollback: [{}]. Rollback failed: {rollback_error}",
                        task.id,
                        parent_task_id,
                        created_ids
                    );
                }

                anyhow::bail!(
                    "Failed to create depends_on link for sub-task '{}' -> parent '{}': {error}. Rolled back created sub-task IDs: [{}]",
                    task.id,
                    parent_task_id,
                    created_ids
                );
            }
        }
    }

    output(
        &PrdSubtasksResponse {
            subtasks,
            parent_task_id: parent_task_id.to_string(),
        },
        flags.format,
    )
}

fn join_task_ids(tasks: &[Task]) -> String {
    tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

async fn rollback_subtasks(
    ctx: &AppContext,
    session_id: &str,
    tasks: &[Task],
    link_ids: &[String],
) -> anyhow::Result<()> {
    let mut rollback_failures = Vec::new();

    for link_id in link_ids.iter().rev() {
        if let Err(error) = ctx.service.delete_link(session_id, link_id).await {
            rollback_failures.push(format!("link {link_id} ({error})"));
        }
    }

    for task in tasks.iter().rev() {
        if let Err(error) = ctx.service.delete_task(session_id, &task.id).await {
            rollback_failures.push(format!("task {} ({error})", task.id));
        }
    }

    if rollback_failures.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "Unable to rollback all created subtasks. Failed deletes: {}",
        rollback_failures.join(", ")
    )
}

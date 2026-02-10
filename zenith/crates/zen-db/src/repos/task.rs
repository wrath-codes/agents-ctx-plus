//! Task repository — CRUD + FTS + status transitions.

use chrono::Utc;

use zen_core::audit_detail::StatusChangedDetail;
use zen_core::entities::{AuditEntry, Task};
use zen_core::enums::{AuditAction, EntityType, TaskStatus, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_TASK};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::task::TaskUpdate;

const SELECT_COLS: &str =
    "id, research_id, issue_id, session_id, title, description, status, created_at, updated_at";

fn row_to_task(row: &libsql::Row) -> Result<Task, DatabaseError> {
    Ok(Task {
        id: row.get(0)?,
        research_id: get_opt_string(row, 1)?,
        issue_id: get_opt_string(row, 2)?,
        session_id: get_opt_string(row, 3)?,
        title: row.get(4)?,
        description: get_opt_string(row, 5)?,
        status: parse_enum(&row.get::<String>(6)?)?,
        created_at: parse_datetime(&row.get::<String>(7)?)?,
        updated_at: parse_datetime(&row.get::<String>(8)?)?,
    })
}

impl ZenService {
    pub async fn create_task(
        &self,
        session_id: &str,
        title: &str,
        description: Option<&str>,
        issue_id: Option<&str>,
        research_id: Option<&str>,
    ) -> Result<Task, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_TASK).await?;

        self.db().conn().execute(
            &format!(
                "INSERT INTO tasks ({SELECT_COLS})
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
            ),
            libsql::params![
                id.as_str(),
                research_id,
                issue_id,
                session_id,
                title,
                description,
                TaskStatus::Open.as_str(),
                now.to_rfc3339(),
                now.to_rfc3339()
            ],
        ).await?;

        let task = Task {
            id: id.clone(),
            research_id: research_id.map(String::from),
            issue_id: issue_id.map(String::from),
            session_id: Some(session_id.to_string()),
            title: title.to_string(),
            description: description.map(String::from),
            status: TaskStatus::Open,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Task,
            entity_id: id.clone(),
            action: AuditAction::Created,
            detail: None,
            created_at: now,
        }).await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Create,
            entity: EntityType::Task,
            id: id.clone(),
            data: serde_json::to_value(&task).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(task)
    }

    pub async fn get_task(&self, id: &str) -> Result<Task, DatabaseError> {
        let mut rows = self.db().conn().query(
            &format!("SELECT {SELECT_COLS} FROM tasks WHERE id = ?1"),
            [id],
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_task(&row)
    }

    pub async fn update_task(
        &self,
        session_id: &str,
        task_id: &str,
        update: TaskUpdate,
    ) -> Result<Task, DatabaseError> {
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1usize;

        if let Some(ref title) = update.title {
            sets.push(format!("title = ?{idx}"));
            params.push(title.clone().into());
            idx += 1;
        }
        if let Some(ref description) = update.description {
            sets.push(format!("description = ?{idx}"));
            params.push(description.clone().map_or(libsql::Value::Null, Into::into));
            idx += 1;
        }
        if let Some(ref status) = update.status {
            sets.push(format!("status = ?{idx}"));
            params.push(status.as_str().into());
            idx += 1;
        }
        if let Some(ref issue_id) = update.issue_id {
            sets.push(format!("issue_id = ?{idx}"));
            params.push(issue_id.clone().map_or(libsql::Value::Null, Into::into));
            idx += 1;
        }
        if let Some(ref research_id) = update.research_id {
            sets.push(format!("research_id = ?{idx}"));
            params.push(research_id.clone().map_or(libsql::Value::Null, Into::into));
            idx += 1;
        }

        if sets.is_empty() {
            return self.get_task(task_id).await;
        }

        let now = Utc::now();
        sets.push(format!("updated_at = ?{idx}"));
        params.push(now.to_rfc3339().into());
        idx += 1;

        params.push(task_id.into());
        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{idx}",
            sets.join(", ")
        );
        self.db().conn().execute(&sql, libsql::params_from_iter(params)).await?;

        let updated = self.get_task(task_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Task,
            entity_id: task_id.to_string(),
            action: AuditAction::Updated,
            detail: Some(serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?),
            created_at: now,
        }).await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Update,
            entity: EntityType::Task,
            id: task_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    pub async fn delete_task(
        &self,
        session_id: &str,
        task_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        self.db().conn().execute(
            "DELETE FROM tasks WHERE id = ?1",
            [task_id],
        ).await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::Task,
            id: task_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    pub async fn list_tasks(&self, limit: u32) -> Result<Vec<Task>, DatabaseError> {
        let mut rows = self.db().conn().query(
            &format!(
                "SELECT {SELECT_COLS} FROM tasks ORDER BY status, created_at DESC LIMIT {limit}"
            ),
            (),
        ).await?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next().await? {
            tasks.push(row_to_task(&row)?);
        }
        Ok(tasks)
    }

    pub async fn search_tasks(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Task>, DatabaseError> {
        let mut rows = self.db().conn().query(
            &format!(
                "SELECT t.id, t.research_id, t.issue_id, t.session_id, t.title, t.description, \
                 t.status, t.created_at, t.updated_at \
                 FROM tasks_fts \
                 JOIN tasks t ON t.rowid = tasks_fts.rowid \
                 WHERE tasks_fts MATCH ?1 \
                 ORDER BY rank LIMIT ?2"
            ),
            libsql::params![query, limit],
        ).await?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next().await? {
            tasks.push(row_to_task(&row)?);
        }
        Ok(tasks)
    }

    pub async fn transition_task(
        &self,
        session_id: &str,
        task_id: &str,
        new_status: TaskStatus,
    ) -> Result<Task, DatabaseError> {
        let current = self.get_task(task_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition task {} from {} to {}",
                task_id, current.status, new_status
            )));
        }

        let now = Utc::now();
        self.db().conn().execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            libsql::params![new_status.as_str(), now.to_rfc3339(), task_id],
        ).await?;

        let updated = Task {
            status: new_status,
            updated_at: now,
            ..current.clone()
        };

        let detail = StatusChangedDetail {
            from: current.status.as_str().to_string(),
            to: new_status.as_str().to_string(),
            reason: None,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Task,
            entity_id: task_id.to_string(),
            action: AuditAction::StatusChanged,
            detail: Some(serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?),
            created_at: now,
        }).await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Transition,
            entity: EntityType::Task,
            id: task_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    pub async fn get_tasks_for_issue(
        &self,
        issue_id: &str,
    ) -> Result<Vec<Task>, DatabaseError> {
        let mut rows = self.db().conn().query(
            &format!(
                "SELECT {SELECT_COLS} FROM tasks WHERE issue_id = ?1 ORDER BY status, created_at"
            ),
            [issue_id],
        ).await?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next().await? {
            tasks.push(row_to_task(&row)?);
        }
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::task::TaskUpdateBuilder;
    use zen_core::enums::IssueType;

    #[tokio::test]
    async fn create_task_roundtrip() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "Implement auth", Some("Add JWT middleware"), None, None)
            .await
            .unwrap();

        assert!(task.id.starts_with("tsk-"));
        assert_eq!(task.title, "Implement auth");
        assert_eq!(task.description.as_deref(), Some("Add JWT middleware"));
        assert_eq!(task.status, TaskStatus::Open);

        let fetched = svc.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.title, "Implement auth");
    }

    #[tokio::test]
    async fn update_task_partial() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "Original task", None, None, None)
            .await
            .unwrap();

        let update = TaskUpdateBuilder::new().title("Updated task").build();
        let updated = svc.update_task(&ses, &task.id, update).await.unwrap();
        assert_eq!(updated.title, "Updated task");
    }

    #[tokio::test]
    async fn delete_task() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "To delete", None, None, None)
            .await
            .unwrap();

        svc.delete_task(&ses, &task.id).await.unwrap();
        let result = svc.get_task(&task.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_tasks() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        svc.create_task(&ses, "Task A", None, None, None).await.unwrap();
        svc.create_task(&ses, "Task B", None, None, None).await.unwrap();

        let tasks = svc.list_tasks(10).await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn search_task_fts() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        svc.create_task(&ses, "Authentication middleware", None, None, None)
            .await
            .unwrap();
        svc.create_task(&ses, "Database schema", None, None, None)
            .await
            .unwrap();

        let results = svc.search_tasks("authentication", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Authentication middleware");
    }

    #[tokio::test]
    async fn transition_task_valid() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "Transition me", None, None, None)
            .await
            .unwrap();

        let updated = svc
            .transition_task(&ses, &task.id, TaskStatus::InProgress)
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn transition_task_invalid() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "Bad transition", None, None, None)
            .await
            .unwrap();

        let result = svc
            .transition_task(&ses, &task.id, TaskStatus::Done)
            .await;
        assert!(matches!(result, Err(DatabaseError::InvalidState(_))));
    }

    #[tokio::test]
    async fn task_issue_linkage() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Parent issue", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        let task = svc
            .create_task(&ses, "Linked task", None, Some(&issue.id), None)
            .await
            .unwrap();

        let tasks = svc.get_tasks_for_issue(&issue.id).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task.id);
    }

    #[tokio::test]
    async fn task_audit_on_create() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let task = svc
            .create_task(&ses, "Audited task", None, None, None)
            .await
            .unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_id: Some(task.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_type, EntityType::Task);
    }
}

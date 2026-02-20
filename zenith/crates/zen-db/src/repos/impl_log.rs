//! Implementation log repository — CRUD for task implementation entries.

use chrono::Utc;

use zen_core::entities::{AuditEntry, ImplLog};
use zen_core::enums::{AuditAction, EntityType, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_IMPL_LOG};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime};
use crate::service::ZenService;

const SELECT_COLS: &str =
    "id, task_id, session_id, file_path, start_line, end_line, description, created_at";

fn row_to_impl_log(row: &libsql::Row) -> Result<ImplLog, DatabaseError> {
    Ok(ImplLog {
        id: row.get(0)?,
        task_id: row.get(1)?,
        session_id: get_opt_string(row, 2)?,
        file_path: row.get(3)?,
        start_line: row.get::<Option<i64>>(4)?,
        end_line: row.get::<Option<i64>>(5)?,
        description: get_opt_string(row, 6)?,
        created_at: parse_datetime(&row.get::<String>(7)?)?,
    })
}

impl ZenService {
    pub async fn create_impl_log(
        &self,
        session_id: &str,
        task_id: &str,
        file_path: &str,
        start_line: Option<i64>,
        end_line: Option<i64>,
        description: Option<&str>,
    ) -> Result<ImplLog, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_IMPL_LOG).await?;

        self.db()
            .execute_with(
                "INSERT INTO implementation_log (id, task_id, session_id, file_path, start_line, end_line, description, created_at, org_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                || libsql::params![
                    id.as_str(),
                    task_id,
                    session_id,
                    file_path,
                    start_line,
                    end_line,
                    description,
                    now.to_rfc3339(),
                    self.org_id()
                ],
            )
            .await?;

        let log = ImplLog {
            id: id.clone(),
            task_id: task_id.to_string(),
            session_id: Some(session_id.to_string()),
            file_path: file_path.to_string(),
            start_line,
            end_line,
            description: description.map(String::from),
            created_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::ImplLog,
            entity_id: id.clone(),
            action: AuditAction::Created,
            detail: None,
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Create,
            entity: EntityType::ImplLog,
            id: id.clone(),
            data: serde_json::to_value(&log).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(log)
    }

    pub async fn get_impl_log(&self, id: &str) -> Result<ImplLog, DatabaseError> {
        let mut rows = self
            .db()
            .query(
                &format!("SELECT {SELECT_COLS} FROM implementation_log WHERE id = ?1"),
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_impl_log(&row)
    }

    pub async fn delete_impl_log(
        &self,
        session_id: &str,
        impl_log_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM implementation_log WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![impl_log_id.into()];
        del_params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(del_params.clone()))
            .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::ImplLog,
            id: impl_log_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    pub async fn list_impl_logs(&self, limit: u32) -> Result<Vec<ImplLog>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM implementation_log WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(org_params.clone()))
            .await?;

        let mut logs = Vec::new();
        while let Some(row) = rows.next().await? {
            logs.push(row_to_impl_log(&row)?);
        }
        Ok(logs)
    }

    pub async fn get_logs_for_task(&self, task_id: &str) -> Result<Vec<ImplLog>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM implementation_log WHERE task_id = ?1 {org_filter} ORDER BY created_at"
        );
        let mut params: Vec<libsql::Value> = vec![task_id.into()];
        params.extend(org_params);
        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let mut logs = Vec::new();
        while let Some(row) = rows.next().await? {
            logs.push(row_to_impl_log(&row)?);
        }
        Ok(logs)
    }

    pub async fn get_logs_by_file(&self, file_path: &str) -> Result<Vec<ImplLog>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM implementation_log WHERE file_path LIKE ?1 || '%' {org_filter} ORDER BY created_at"
        );
        let mut params: Vec<libsql::Value> = vec![file_path.into()];
        params.extend(org_params);
        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let mut logs = Vec::new();
        while let Some(row) = rows.next().await? {
            logs.push(row_to_impl_log(&row)?);
        }
        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};

    async fn create_test_task(svc: &ZenService, ses: &str) -> String {
        let task = svc
            .create_task(ses, "Test task", None, None, None)
            .await
            .unwrap();
        task.id
    }

    #[tokio::test]
    async fn create_impl_log_roundtrip() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        let log = svc
            .create_impl_log(
                &ses,
                &task_id,
                "src/main.rs",
                Some(10),
                Some(25),
                Some("Added handler"),
            )
            .await
            .unwrap();

        assert!(log.id.starts_with("imp-"));
        assert_eq!(log.task_id, task_id);
        assert_eq!(log.file_path, "src/main.rs");
        assert_eq!(log.start_line, Some(10));
        assert_eq!(log.end_line, Some(25));
        assert_eq!(log.description.as_deref(), Some("Added handler"));

        let fetched = svc.get_impl_log(&log.id).await.unwrap();
        assert_eq!(fetched.file_path, "src/main.rs");
    }

    #[tokio::test]
    async fn delete_impl_log() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        let log = svc
            .create_impl_log(&ses, &task_id, "src/lib.rs", None, None, None)
            .await
            .unwrap();

        svc.delete_impl_log(&ses, &log.id).await.unwrap();
        let result = svc.get_impl_log(&log.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_impl_logs() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        svc.create_impl_log(&ses, &task_id, "src/a.rs", None, None, None)
            .await
            .unwrap();
        svc.create_impl_log(&ses, &task_id, "src/b.rs", None, None, None)
            .await
            .unwrap();

        let logs = svc.list_impl_logs(10).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn get_logs_for_task() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        svc.create_impl_log(&ses, &task_id, "src/handler.rs", Some(1), Some(50), None)
            .await
            .unwrap();
        svc.create_impl_log(&ses, &task_id, "src/model.rs", None, None, None)
            .await
            .unwrap();

        let logs = svc.get_logs_for_task(&task_id).await.unwrap();
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|l| l.task_id == task_id));
    }

    #[tokio::test]
    async fn get_logs_by_file() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        svc.create_impl_log(&ses, &task_id, "src/main.rs", None, None, None)
            .await
            .unwrap();
        svc.create_impl_log(&ses, &task_id, "tests/test.rs", None, None, None)
            .await
            .unwrap();

        let logs = svc.get_logs_by_file("src/").await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].file_path, "src/main.rs");
    }

    #[tokio::test]
    async fn impl_log_audit_on_create() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;
        let task_id = create_test_task(&svc, &ses).await;

        let log = svc
            .create_impl_log(&ses, &task_id, "src/lib.rs", None, None, None)
            .await
            .unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_id: Some(log.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_type, EntityType::ImplLog);
    }
}

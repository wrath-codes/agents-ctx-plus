use zen_core::entities::{Hypothesis, Task};
use zen_core::responses::WhatsNextResponse;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::repos::audit::AuditFilter;
use crate::service::ZenService;

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

fn row_to_hypothesis(row: &libsql::Row) -> Result<Hypothesis, DatabaseError> {
    Ok(Hypothesis {
        id: row.get::<String>(0)?,
        research_id: get_opt_string(row, 1)?,
        finding_id: get_opt_string(row, 2)?,
        session_id: get_opt_string(row, 3)?,
        content: row.get::<String>(4)?,
        status: parse_enum(&row.get::<String>(5)?)?,
        reason: get_opt_string(row, 6)?,
        created_at: parse_datetime(&row.get::<String>(7)?)?,
        updated_at: parse_datetime(&row.get::<String>(8)?)?,
    })
}

impl ZenService {
    pub async fn whats_next(&self) -> Result<WhatsNextResponse, DatabaseError> {
        let sessions = self.list_sessions(None, 1).await?;
        let last_session = sessions.into_iter().next();

        let (task_org_filter, task_org_params) = self.org_id_filter(1);
        let task_sql = format!(
            "SELECT id, research_id, issue_id, session_id, title, description, status, created_at, updated_at \
             FROM tasks WHERE status IN ('open', 'in_progress') {task_org_filter} \
             ORDER BY status, created_at"
        );
        let mut task_rows = self
            .db()
            .query_with(&task_sql, || libsql::params_from_iter(task_org_params.clone()))
            .await?;

        let mut open_tasks = Vec::new();
        while let Some(row) = task_rows.next().await? {
            open_tasks.push(row_to_task(&row)?);
        }

        let (hyp_org_filter, hyp_org_params) = self.org_id_filter(1);
        let hyp_sql = format!(
            "SELECT id, research_id, finding_id, session_id, content, status, reason, created_at, updated_at \
             FROM hypotheses WHERE status IN ('unverified', 'analyzing') {hyp_org_filter} \
             ORDER BY created_at DESC"
        );
        let mut hyp_rows = self
            .db()
            .query_with(&hyp_sql, || libsql::params_from_iter(hyp_org_params.clone()))
            .await?;

        let mut pending_hypotheses = Vec::new();
        while let Some(row) = hyp_rows.next().await? {
            pending_hypotheses.push(row_to_hypothesis(&row)?);
        }

        let recent_audit = self
            .query_audit(&AuditFilter {
                limit: Some(20),
                ..Default::default()
            })
            .await?;

        Ok(WhatsNextResponse {
            last_session,
            open_tasks,
            pending_hypotheses,
            recent_audit,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::helpers::{start_test_session, test_service};
    use zen_core::enums::{HypothesisStatus, TaskStatus};

    #[tokio::test]
    async fn whats_next_empty() {
        let svc = test_service().await;
        let resp = svc.whats_next().await.unwrap();

        assert!(resp.last_session.is_none());
        assert!(resp.open_tasks.is_empty());
        assert!(resp.pending_hypotheses.is_empty());
        assert!(resp.recent_audit.is_empty());
    }

    #[tokio::test]
    async fn whats_next_with_data() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        svc.create_task(&ses, "Open task", None, None, None)
            .await
            .unwrap();
        svc.create_task(&ses, "Another open task", None, None, None)
            .await
            .unwrap();

        svc.create_hypothesis(&ses, "Unverified hypothesis", None, None)
            .await
            .unwrap();

        let resp = svc.whats_next().await.unwrap();

        assert_eq!(resp.open_tasks.len(), 2);
        assert!(resp.open_tasks.iter().all(|t| t.status == TaskStatus::Open));
        assert_eq!(resp.pending_hypotheses.len(), 1);
        assert_eq!(
            resp.pending_hypotheses[0].status,
            HypothesisStatus::Unverified
        );
        assert!(!resp.recent_audit.is_empty());
    }

    #[tokio::test]
    async fn whats_next_last_session() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let resp = svc.whats_next().await.unwrap();

        assert!(resp.last_session.is_some());
        assert_eq!(resp.last_session.unwrap().id, ses);
    }
}

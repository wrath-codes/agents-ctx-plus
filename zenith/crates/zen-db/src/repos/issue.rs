//! Issue repository — CRUD + FTS + status transitions.

use chrono::Utc;

use zen_core::audit_detail::StatusChangedDetail;
use zen_core::entities::{AuditEntry, Issue};
use zen_core::enums::{AuditAction, EntityType, IssueStatus, IssueType, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_ISSUE};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::issue::IssueUpdate;

const SELECT_COLS: &str =
    "id, type, parent_id, title, description, status, priority, session_id, created_at, updated_at";

fn row_to_issue(row: &libsql::Row) -> Result<Issue, DatabaseError> {
    Ok(Issue {
        id: row.get(0)?,
        issue_type: parse_enum(&row.get::<String>(1)?)?,
        parent_id: get_opt_string(row, 2)?,
        title: row.get(3)?,
        description: get_opt_string(row, 4)?,
        status: parse_enum(&row.get::<String>(5)?)?,
        priority: row.get::<i64>(6)? as u8,
        session_id: get_opt_string(row, 7)?,
        created_at: parse_datetime(&row.get::<String>(8)?)?,
        updated_at: parse_datetime(&row.get::<String>(9)?)?,
    })
}

impl ZenService {
    pub async fn create_issue(
        &self,
        session_id: &str,
        title: &str,
        issue_type: IssueType,
        priority: u8,
        description: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<Issue, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_ISSUE).await?;

        self.db()
            .conn()
            .execute(
                "INSERT INTO issues (id, type, parent_id, title, description, status, priority, session_id, created_at, updated_at, org_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                libsql::params![
                    id.as_str(),
                    issue_type.as_str(),
                    parent_id,
                    title,
                    description,
                    IssueStatus::Open.as_str(),
                    priority as i64,
                    session_id,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                    self.org_id()
                ],
            )
            .await?;

        let issue = Issue {
            id: id.clone(),
            issue_type,
            parent_id: parent_id.map(String::from),
            title: title.to_string(),
            description: description.map(String::from),
            status: IssueStatus::Open,
            priority,
            session_id: Some(session_id.to_string()),
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Issue,
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
            entity: EntityType::Issue,
            id: id.clone(),
            data: serde_json::to_value(&issue).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(issue)
    }

    pub async fn get_issue(&self, id: &str) -> Result<Issue, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                &format!("SELECT {SELECT_COLS} FROM issues WHERE id = ?1"),
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_issue(&row)
    }

    pub async fn update_issue(
        &self,
        session_id: &str,
        issue_id: &str,
        update: IssueUpdate,
    ) -> Result<Issue, DatabaseError> {
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
        if let Some(ref priority) = update.priority {
            sets.push(format!("priority = ?{idx}"));
            params.push((*priority as i64).into());
            idx += 1;
        }
        if let Some(ref issue_type) = update.issue_type {
            sets.push(format!("type = ?{idx}"));
            params.push(issue_type.as_str().into());
            idx += 1;
        }
        if let Some(ref parent_id) = update.parent_id {
            sets.push(format!("parent_id = ?{idx}"));
            params.push(parent_id.clone().map_or(libsql::Value::Null, Into::into));
            idx += 1;
        }

        if sets.is_empty() {
            return self.get_issue(issue_id).await;
        }

        let now = Utc::now();
        sets.push(format!("updated_at = ?{idx}"));
        params.push(now.to_rfc3339().into());
        idx += 1;

        params.push(issue_id.into());
        let id_idx = idx;
        idx += 1;
        let (org_filter, org_params) = self.org_id_filter(idx as u32);
        params.extend(org_params);
        let sql = format!("UPDATE issues SET {} WHERE id = ?{id_idx} {org_filter}", sets.join(", "));
        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(params))
            .await?;

        let updated = self.get_issue(issue_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Issue,
            entity_id: issue_id.to_string(),
            action: AuditAction::Updated,
            detail: Some(
                serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
            ),
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Update,
            entity: EntityType::Issue,
            id: issue_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    pub async fn delete_issue(
        &self,
        session_id: &str,
        issue_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM issues WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![issue_id.into()];
        del_params.extend(org_params);
        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(del_params))
            .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::Issue,
            id: issue_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    pub async fn list_issues(&self, limit: u32) -> Result<Vec<Issue>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM issues WHERE 1=1 {org_filter} ORDER BY priority, created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(org_params)).await?;

        let mut issues = Vec::new();
        while let Some(row) = rows.next().await? {
            issues.push(row_to_issue(&row)?);
        }
        Ok(issues)
    }

    pub async fn search_issues(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Issue>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT i.id, i.type, i.parent_id, i.title, i.description, i.status, \
             i.priority, i.session_id, i.created_at, i.updated_at \
             FROM issues_fts \
             JOIN issues i ON i.rowid = issues_fts.rowid \
             WHERE issues_fts MATCH ?1 {org_filter} \
             ORDER BY rank LIMIT ?2"
        );
        let mut params: Vec<libsql::Value> = vec![query.into(), (limit as i64).into()];
        params.extend(org_params);
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(params)).await?;

        let mut issues = Vec::new();
        while let Some(row) = rows.next().await? {
            issues.push(row_to_issue(&row)?);
        }
        Ok(issues)
    }

    pub async fn transition_issue(
        &self,
        session_id: &str,
        issue_id: &str,
        new_status: IssueStatus,
    ) -> Result<Issue, DatabaseError> {
        let current = self.get_issue(issue_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition issue {} from {} to {}",
                issue_id, current.status, new_status
            )));
        }

        let now = Utc::now();
        let (org_filter, org_params) = self.org_id_filter(4);
        let sql = format!("UPDATE issues SET status = ?1, updated_at = ?2 WHERE id = ?3 {org_filter}");
        let mut params: Vec<libsql::Value> = vec![new_status.as_str().into(), now.to_rfc3339().into(), issue_id.into()];
        params.extend(org_params);
        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(params))
            .await?;

        let updated = Issue {
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
            entity_type: EntityType::Issue,
            entity_id: issue_id.to_string(),
            action: AuditAction::StatusChanged,
            detail: Some(
                serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
            ),
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Transition,
            entity: EntityType::Issue,
            id: issue_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    pub async fn get_child_issues(&self, parent_id: &str) -> Result<Vec<Issue>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM issues WHERE parent_id = ?1 {org_filter} ORDER BY priority, created_at"
        );
        let mut params: Vec<libsql::Value> = vec![parent_id.into()];
        params.extend(org_params);
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(params)).await?;

        let mut issues = Vec::new();
        while let Some(row) = rows.next().await? {
            issues.push(row_to_issue(&row)?);
        }
        Ok(issues)
    }

    pub async fn get_parent_issue(&self, issue_id: &str) -> Result<Option<Issue>, DatabaseError> {
        let issue = self.get_issue(issue_id).await?;
        match issue.parent_id {
            Some(ref pid) => Ok(Some(self.get_issue(pid).await?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::issue::IssueUpdateBuilder;

    #[tokio::test]
    async fn create_issue_roundtrip() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(
                &ses,
                "Fix login bug",
                IssueType::Bug,
                2,
                Some("Login fails"),
                None,
            )
            .await
            .unwrap();

        assert!(issue.id.starts_with("iss-"));
        assert_eq!(issue.title, "Fix login bug");
        assert_eq!(issue.issue_type, IssueType::Bug);
        assert_eq!(issue.priority, 2);
        assert_eq!(issue.description.as_deref(), Some("Login fails"));
        assert_eq!(issue.status, IssueStatus::Open);

        let fetched = svc.get_issue(&issue.id).await.unwrap();
        assert_eq!(fetched.title, "Fix login bug");
    }

    #[tokio::test]
    async fn update_issue_partial() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Original", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        let update = IssueUpdateBuilder::new().title("Updated Title").build();
        let updated = svc.update_issue(&ses, &issue.id, update).await.unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.priority, 3);
    }

    #[tokio::test]
    async fn delete_issue() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "To delete", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        svc.delete_issue(&ses, &issue.id).await.unwrap();
        let result = svc.get_issue(&issue.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_issues() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        svc.create_issue(&ses, "Issue A", IssueType::Bug, 1, None, None)
            .await
            .unwrap();
        svc.create_issue(&ses, "Issue B", IssueType::Feature, 2, None, None)
            .await
            .unwrap();

        let issues = svc.list_issues(10).await.unwrap();
        assert_eq!(issues.len(), 2);
    }

    #[tokio::test]
    async fn search_issue_fts() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        svc.create_issue(
            &ses,
            "Authentication refactor",
            IssueType::Feature,
            3,
            None,
            None,
        )
        .await
        .unwrap();
        svc.create_issue(&ses, "Database migration", IssueType::Spike, 3, None, None)
            .await
            .unwrap();

        let results = svc.search_issues("authentication", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Authentication refactor");
    }

    #[tokio::test]
    async fn transition_issue_valid() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Transition me", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        let updated = svc
            .transition_issue(&ses, &issue.id, IssueStatus::InProgress)
            .await
            .unwrap();
        assert_eq!(updated.status, IssueStatus::InProgress);
    }

    #[tokio::test]
    async fn transition_issue_invalid() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Bad transition", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        let result = svc
            .transition_issue(&ses, &issue.id, IssueStatus::Done)
            .await;
        assert!(matches!(result, Err(DatabaseError::InvalidState(_))));
    }

    #[tokio::test]
    async fn issue_parent_child() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let parent = svc
            .create_issue(&ses, "Parent epic", IssueType::Epic, 1, None, None)
            .await
            .unwrap();

        let child = svc
            .create_issue(
                &ses,
                "Child task",
                IssueType::Bug,
                2,
                None,
                Some(&parent.id),
            )
            .await
            .unwrap();

        let children = svc.get_child_issues(&parent.id).await.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, child.id);

        let found_parent = svc.get_parent_issue(&child.id).await.unwrap();
        assert!(found_parent.is_some());
        assert_eq!(found_parent.unwrap().id, parent.id);
    }

    #[tokio::test]
    async fn issue_type_column_mapping() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Feature request", IssueType::Feature, 3, None, None)
            .await
            .unwrap();

        let fetched = svc.get_issue(&issue.id).await.unwrap();
        assert_eq!(fetched.issue_type, IssueType::Feature);
    }

    #[tokio::test]
    async fn issue_audit_on_create() {
        let svc = test_service().await;
        let ses = start_test_session(&svc).await;

        let issue = svc
            .create_issue(&ses, "Audited issue", IssueType::Bug, 3, None, None)
            .await
            .unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_id: Some(issue.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_type, EntityType::Issue);
    }
}

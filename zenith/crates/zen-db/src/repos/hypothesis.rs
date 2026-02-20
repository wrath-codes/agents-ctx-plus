//! Hypothesis repository — CRUD + FTS + status transitions.

use chrono::Utc;

use zen_core::audit_detail::StatusChangedDetail;
use zen_core::entities::{AuditEntry, Hypothesis};
use zen_core::enums::{AuditAction, EntityType, HypothesisStatus, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_HYPOTHESIS};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::hypothesis::HypothesisUpdate;

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

const SELECT_COLS: &str =
    "id, research_id, finding_id, session_id, content, status, reason, created_at, updated_at";

impl ZenService {
    /// Create a new hypothesis.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the INSERT fails.
    pub async fn create_hypothesis(
        &self,
        session_id: &str,
        content: &str,
        research_id: Option<&str>,
        finding_id: Option<&str>,
    ) -> Result<Hypothesis, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_HYPOTHESIS).await?;

        self.db().execute_with(
            "INSERT INTO hypotheses (id, research_id, finding_id, session_id, content, status, created_at, updated_at, org_id)
             VALUES (?1, ?2, ?3, ?4, ?5, 'unverified', ?6, ?6, ?7)",
            || libsql::params![
                id.as_str(),
                research_id,
                finding_id,
                session_id,
                content,
                now.to_rfc3339(),
                self.org_id()
            ],
        ).await?;

        let hypothesis = Hypothesis {
            id: id.clone(),
            research_id: research_id.map(String::from),
            finding_id: finding_id.map(String::from),
            session_id: Some(session_id.to_string()),
            content: content.to_string(),
            status: HypothesisStatus::Unverified,
            reason: None,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Hypothesis,
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
            entity: EntityType::Hypothesis,
            id: id.clone(),
            data: serde_json::to_value(&hypothesis).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(hypothesis)
    }

    /// Get a hypothesis by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NoResult` if the hypothesis does not exist.
    pub async fn get_hypothesis(&self, id: &str) -> Result<Hypothesis, DatabaseError> {
        let sql = format!("SELECT {SELECT_COLS} FROM hypotheses WHERE id = ?1");
        let mut rows = self.db().query(&sql, [id]).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_hypothesis(&row)
    }

    /// Update a hypothesis with dynamic SET clauses.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the UPDATE fails or the entity is not found.
    pub async fn update_hypothesis(
        &self,
        session_id: &str,
        hyp_id: &str,
        update: HypothesisUpdate,
    ) -> Result<Hypothesis, DatabaseError> {
        let now = Utc::now();
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref content) = update.content {
            params.push(libsql::Value::Text(content.clone()));
            sets.push(format!("content = ?{idx}"));
            idx += 1;
        }
        if let Some(ref status) = update.status {
            params.push(libsql::Value::Text(status.as_str().to_string()));
            sets.push(format!("status = ?{idx}"));
            idx += 1;
        }
        if let Some(ref reason) = update.reason {
            match reason {
                Some(r) => params.push(libsql::Value::Text(r.clone())),
                None => params.push(libsql::Value::Null),
            }
            sets.push(format!("reason = ?{idx}"));
            idx += 1;
        }

        params.push(libsql::Value::Text(now.to_rfc3339()));
        sets.push(format!("updated_at = ?{idx}"));
        idx += 1;

        params.push(libsql::Value::Text(hyp_id.to_string()));
        let id_idx = idx;
        idx += 1;
        let (org_filter, org_params) = self.org_id_filter(idx as u32);
        params.extend(org_params);
        let sql = format!(
            "UPDATE hypotheses SET {} WHERE id = ?{id_idx} {org_filter}",
            sets.join(", ")
        );

        self.db()
            .execute_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let updated = self.get_hypothesis(hyp_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Hypothesis,
            entity_id: hyp_id.to_string(),
            action: AuditAction::Updated,
            detail: None,
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Update,
            entity: EntityType::Hypothesis,
            id: hyp_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    /// Delete a hypothesis.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the DELETE fails.
    pub async fn delete_hypothesis(
        &self,
        session_id: &str,
        hyp_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM hypotheses WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![hyp_id.into()];
        del_params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(del_params.clone()))
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Hypothesis,
            entity_id: hyp_id.to_string(),
            action: AuditAction::StatusChanged,
            detail: Some(serde_json::json!({ "action": "deleted" })),
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::Hypothesis,
            id: hyp_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    /// List hypotheses ordered by creation date descending.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn list_hypotheses(&self, limit: u32) -> Result<Vec<Hypothesis>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM hypotheses WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().query_with(&sql, || libsql::params_from_iter(org_params.clone())).await?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await? {
            items.push(row_to_hypothesis(&row)?);
        }
        Ok(items)
    }

    /// FTS5 search across hypotheses.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn search_hypotheses(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Hypothesis>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT h.id, h.research_id, h.finding_id, h.session_id, h.content, h.status, h.reason, h.created_at, h.updated_at \
             FROM hypotheses_fts fts \
             JOIN hypotheses h ON h.rowid = fts.rowid \
             WHERE hypotheses_fts MATCH ?1 {org_filter} \
             ORDER BY rank LIMIT ?2"
        );

        let mut params: Vec<libsql::Value> = vec![query.into(), (limit as i64).into()];
        params.extend(org_params);
        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await? {
            items.push(row_to_hypothesis(&row)?);
        }
        Ok(items)
    }

    /// Transition a hypothesis to a new status.
    ///
    /// Validates the transition via `can_transition_to()`. Updates the `reason`
    /// column if provided.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::InvalidState` if the transition is not allowed.
    pub async fn transition_hypothesis(
        &self,
        session_id: &str,
        hyp_id: &str,
        new_status: HypothesisStatus,
        reason: Option<&str>,
    ) -> Result<Hypothesis, DatabaseError> {
        let current = self.get_hypothesis(hyp_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition hypothesis {} from {} to {}",
                hyp_id, current.status, new_status
            )));
        }

        let now = Utc::now();

        if reason.is_some() {
            let (org_filter, org_params) = self.org_id_filter(5);
            let sql = format!("UPDATE hypotheses SET status = ?1, reason = ?2, updated_at = ?3 WHERE id = ?4 {org_filter}");
            let mut params: Vec<libsql::Value> = vec![new_status.as_str().into(), reason.into(), now.to_rfc3339().into(), hyp_id.into()];
            params.extend(org_params);
            self.db()
                .execute_with(&sql, || libsql::params_from_iter(params.clone()))
                .await?;
        } else {
            let (org_filter, org_params) = self.org_id_filter(4);
            let sql = format!("UPDATE hypotheses SET status = ?1, updated_at = ?2 WHERE id = ?3 {org_filter}");
            let mut params: Vec<libsql::Value> = vec![new_status.as_str().into(), now.to_rfc3339().into(), hyp_id.into()];
            params.extend(org_params);
            self.db()
                .execute_with(&sql, || libsql::params_from_iter(params.clone()))
                .await?;
        }

        let updated = Hypothesis {
            status: new_status,
            reason: reason.map(String::from).or(current.reason.clone()),
            updated_at: now,
            ..current.clone()
        };

        let detail = StatusChangedDetail {
            from: current.status.to_string(),
            to: new_status.to_string(),
            reason: reason.map(String::from),
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Hypothesis,
            entity_id: hyp_id.to_string(),
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
            entity: EntityType::Hypothesis,
            id: hyp_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};

    #[tokio::test]
    async fn create_hypothesis_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "tokio works well with axum", None, None)
            .await
            .unwrap();

        assert!(hyp.id.starts_with("hyp-"));
        assert_eq!(hyp.session_id.as_deref(), Some(sid.as_str()));
        assert_eq!(hyp.content, "tokio works well with axum");
        assert_eq!(hyp.status, HypothesisStatus::Unverified);
        assert_eq!(hyp.reason, None);
        assert_eq!(hyp.research_id, None);
        assert_eq!(hyp.finding_id, None);

        let fetched = svc.get_hypothesis(&hyp.id).await.unwrap();
        assert_eq!(fetched.id, hyp.id);
        assert_eq!(fetched.content, hyp.content);
        assert_eq!(fetched.status, hyp.status);
    }

    #[tokio::test]
    async fn update_hypothesis_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Original content", None, None)
            .await
            .unwrap();

        let update = HypothesisUpdate {
            content: Some("Updated content".to_string()),
            ..Default::default()
        };
        let updated = svc.update_hypothesis(&sid, &hyp.id, update).await.unwrap();

        assert_eq!(updated.content, "Updated content");
        assert_eq!(updated.status, HypothesisStatus::Unverified);
    }

    #[tokio::test]
    async fn delete_hypothesis() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "To delete", None, None)
            .await
            .unwrap();
        svc.delete_hypothesis(&sid, &hyp.id).await.unwrap();

        let result = svc.get_hypothesis(&hyp.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_hypotheses() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_hypothesis(&sid, "Hyp 1", None, None)
            .await
            .unwrap();
        svc.create_hypothesis(&sid, "Hyp 2", None, None)
            .await
            .unwrap();
        svc.create_hypothesis(&sid, "Hyp 3", None, None)
            .await
            .unwrap();

        let items = svc.list_hypotheses(10).await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn search_hypotheses_fts() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_hypothesis(&sid, "tokio works well with axum", None, None)
            .await
            .unwrap();
        svc.create_hypothesis(&sid, "database connections are fast", None, None)
            .await
            .unwrap();

        let results = svc.search_hypotheses("tokio", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "tokio works well with axum");
    }

    #[tokio::test]
    async fn transition_hypothesis_valid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Needs analysis", None, None)
            .await
            .unwrap();
        let updated = svc
            .transition_hypothesis(&sid, &hyp.id, HypothesisStatus::Analyzing, None)
            .await
            .unwrap();

        assert_eq!(updated.status, HypothesisStatus::Analyzing);
    }

    #[tokio::test]
    async fn transition_hypothesis_invalid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Needs analysis", None, None)
            .await
            .unwrap();
        let result = svc
            .transition_hypothesis(&sid, &hyp.id, HypothesisStatus::Confirmed, None)
            .await;

        assert!(matches!(result, Err(DatabaseError::InvalidState(_))));
    }

    #[tokio::test]
    async fn transition_hypothesis_with_reason() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Hypothesis to confirm", None, None)
            .await
            .unwrap();

        svc.transition_hypothesis(&sid, &hyp.id, HypothesisStatus::Analyzing, None)
            .await
            .unwrap();

        let confirmed = svc
            .transition_hypothesis(
                &sid,
                &hyp.id,
                HypothesisStatus::Confirmed,
                Some("Benchmarks showed 3x improvement"),
            )
            .await
            .unwrap();

        assert_eq!(confirmed.status, HypothesisStatus::Confirmed);
        assert_eq!(
            confirmed.reason.as_deref(),
            Some("Benchmarks showed 3x improvement")
        );

        let fetched = svc.get_hypothesis(&hyp.id).await.unwrap();
        assert_eq!(
            fetched.reason.as_deref(),
            Some("Benchmarks showed 3x improvement")
        );
    }

    #[tokio::test]
    async fn hypothesis_status_change_audit_detail() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Audit detail test", None, None)
            .await
            .unwrap();

        svc.transition_hypothesis(&sid, &hyp.id, HypothesisStatus::Analyzing, None)
            .await
            .unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_type: Some(EntityType::Hypothesis),
                entity_id: Some(hyp.id.clone()),
                action: Some(AuditAction::StatusChanged),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        let detail: StatusChangedDetail =
            serde_json::from_value(entries[0].detail.clone().unwrap()).unwrap();
        assert_eq!(detail.from, "unverified");
        assert_eq!(detail.to, "analyzing");
    }

    #[tokio::test]
    async fn hypothesis_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "Audited hypothesis", None, None)
            .await
            .unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_type: Some(EntityType::Hypothesis),
                entity_id: Some(hyp.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_id, hyp.id);
        assert_eq!(entries[0].action, AuditAction::Created);
    }
}

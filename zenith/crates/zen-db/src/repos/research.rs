//! Research item repository — CRUD + FTS + status transitions.

use chrono::Utc;

use zen_core::audit_detail::StatusChangedDetail;
use zen_core::entities::{AuditEntry, ResearchItem};
use zen_core::enums::{AuditAction, EntityType, ResearchStatus, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_RESEARCH};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::research::ResearchUpdate;

fn row_to_research(row: &libsql::Row) -> Result<ResearchItem, DatabaseError> {
    Ok(ResearchItem {
        id: row.get::<String>(0)?,
        session_id: get_opt_string(row, 1)?,
        title: row.get::<String>(2)?,
        description: get_opt_string(row, 3)?,
        status: parse_enum(&row.get::<String>(4)?)?,
        created_at: parse_datetime(&row.get::<String>(5)?)?,
        updated_at: parse_datetime(&row.get::<String>(6)?)?,
    })
}

impl ZenService {
    /// Create a new research item.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the INSERT fails.
    pub async fn create_research(
        &self,
        session_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<ResearchItem, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_RESEARCH).await?;

        self.db().execute_with(
            "INSERT INTO research_items (id, session_id, title, description, status, created_at, updated_at, org_id)
             VALUES (?1, ?2, ?3, ?4, 'open', ?5, ?5, ?6)",
            || libsql::params![
                id.as_str(),
                session_id,
                title,
                description,
                now.to_rfc3339(),
                self.org_id()
            ],
        ).await?;

        let research = ResearchItem {
            id: id.clone(),
            session_id: Some(session_id.to_string()),
            title: title.to_string(),
            description: description.map(String::from),
            status: ResearchStatus::Open,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Research,
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
            entity: EntityType::Research,
            id: id.clone(),
            data: serde_json::to_value(&research).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(research)
    }

    /// Get a research item by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NoResult` if the research item does not exist.
    pub async fn get_research(&self, id: &str) -> Result<ResearchItem, DatabaseError> {
        let mut rows = self
            .db()
            .query(
                "SELECT id, session_id, title, description, status, created_at, updated_at
             FROM research_items WHERE id = ?1",
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_research(&row)
    }

    /// Update a research item with dynamic SET clauses.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the UPDATE fails or the entity is not found.
    pub async fn update_research(
        &self,
        session_id: &str,
        research_id: &str,
        update: ResearchUpdate,
    ) -> Result<ResearchItem, DatabaseError> {
        let now = Utc::now();
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref title) = update.title {
            params.push(libsql::Value::Text(title.clone()));
            sets.push(format!("title = ?{idx}"));
            idx += 1;
        }
        if let Some(ref description) = update.description {
            match description {
                Some(d) => params.push(libsql::Value::Text(d.clone())),
                None => params.push(libsql::Value::Null),
            }
            sets.push(format!("description = ?{idx}"));
            idx += 1;
        }
        if let Some(ref status) = update.status {
            params.push(libsql::Value::Text(status.as_str().to_string()));
            sets.push(format!("status = ?{idx}"));
            idx += 1;
        }

        params.push(libsql::Value::Text(now.to_rfc3339()));
        sets.push(format!("updated_at = ?{idx}"));
        idx += 1;

        params.push(libsql::Value::Text(research_id.to_string()));
        let id_idx = idx;
        idx += 1;
        let (org_filter, org_params) = self.org_id_filter(idx as u32);
        params.extend(org_params);
        let sql = format!(
            "UPDATE research_items SET {} WHERE id = ?{id_idx} {org_filter}",
            sets.join(", ")
        );

        self.db()
            .execute_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let updated = self.get_research(research_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Research,
            entity_id: research_id.to_string(),
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
            entity: EntityType::Research,
            id: research_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    /// Delete a research item.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the DELETE fails.
    pub async fn delete_research(
        &self,
        session_id: &str,
        research_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM research_items WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![research_id.into()];
        del_params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(del_params.clone()))
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Research,
            entity_id: research_id.to_string(),
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
            entity: EntityType::Research,
            id: research_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    /// List research items ordered by creation date descending.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn list_research(&self, limit: u32) -> Result<Vec<ResearchItem>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT id, session_id, title, description, status, created_at, updated_at
             FROM research_items WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(org_params.clone()))
            .await?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await? {
            items.push(row_to_research(&row)?);
        }
        Ok(items)
    }

    /// FTS5 search across research items.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn search_research(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ResearchItem>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT r.id, r.session_id, r.title, r.description, r.status, r.created_at, r.updated_at
             FROM research_fts fts
             JOIN research_items r ON r.rowid = fts.rowid
             WHERE research_fts MATCH ?1 {org_filter}
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
            items.push(row_to_research(&row)?);
        }
        Ok(items)
    }

    /// Transition a research item to a new status.
    ///
    /// Validates the transition via `can_transition_to()`.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::InvalidState` if the transition is not allowed.
    pub async fn transition_research(
        &self,
        session_id: &str,
        research_id: &str,
        new_status: ResearchStatus,
    ) -> Result<ResearchItem, DatabaseError> {
        let current = self.get_research(research_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition research {} from {} to {}",
                research_id, current.status, new_status
            )));
        }

        let now = Utc::now();
        let (org_filter, org_params) = self.org_id_filter(4);
        let sql = format!(
            "UPDATE research_items SET status = ?1, updated_at = ?2 WHERE id = ?3 {org_filter}"
        );
        let mut params: Vec<libsql::Value> = vec![
            new_status.as_str().into(),
            now.to_rfc3339().into(),
            research_id.into(),
        ];
        params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let updated = ResearchItem {
            status: new_status,
            updated_at: now,
            ..current.clone()
        };

        let detail = StatusChangedDetail {
            from: current.status.to_string(),
            to: new_status.to_string(),
            reason: None,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Research,
            entity_id: research_id.to_string(),
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
            entity: EntityType::Research,
            id: research_id.to_string(),
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
    async fn create_research_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc
            .create_research(
                &sid,
                "HTTP client comparison",
                Some("Compare reqwest vs hyper"),
            )
            .await
            .unwrap();

        assert!(res.id.starts_with("res-"));
        assert_eq!(res.session_id.as_deref(), Some(sid.as_str()));
        assert_eq!(res.title, "HTTP client comparison");
        assert_eq!(res.description.as_deref(), Some("Compare reqwest vs hyper"));
        assert_eq!(res.status, ResearchStatus::Open);

        let fetched = svc.get_research(&res.id).await.unwrap();
        assert_eq!(fetched.id, res.id);
        assert_eq!(fetched.title, res.title);
        assert_eq!(fetched.description, res.description);
        assert_eq!(fetched.status, res.status);
    }

    #[tokio::test]
    async fn update_research_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc
            .create_research(&sid, "Original title", None)
            .await
            .unwrap();

        let update = ResearchUpdate {
            title: Some("Updated title".to_string()),
            ..Default::default()
        };
        let updated = svc.update_research(&sid, &res.id, update).await.unwrap();

        assert_eq!(updated.title, "Updated title");
        assert_eq!(updated.description, None);
    }

    #[tokio::test]
    async fn update_research_set_description_null() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc
            .create_research(&sid, "Title", Some("Has description"))
            .await
            .unwrap();

        let update = ResearchUpdate {
            description: Some(None),
            ..Default::default()
        };
        let updated = svc.update_research(&sid, &res.id, update).await.unwrap();

        assert_eq!(updated.description, None);
    }

    #[tokio::test]
    async fn delete_research() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc.create_research(&sid, "To delete", None).await.unwrap();
        svc.delete_research(&sid, &res.id).await.unwrap();

        let result = svc.get_research(&res.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_research() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_research(&sid, "Research 1", None).await.unwrap();
        svc.create_research(&sid, "Research 2", None).await.unwrap();
        svc.create_research(&sid, "Research 3", None).await.unwrap();

        let items = svc.list_research(10).await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn search_research_fts() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_research(&sid, "HTTP client comparison", Some("reqwest vs hyper"))
            .await
            .unwrap();
        svc.create_research(&sid, "Database benchmarks", None)
            .await
            .unwrap();

        let results = svc.search_research("client", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "HTTP client comparison");
    }

    #[tokio::test]
    async fn transition_research_valid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc.create_research(&sid, "Topic", None).await.unwrap();
        let updated = svc
            .transition_research(&sid, &res.id, ResearchStatus::InProgress)
            .await
            .unwrap();

        assert_eq!(updated.status, ResearchStatus::InProgress);
    }

    #[tokio::test]
    async fn transition_research_invalid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc.create_research(&sid, "Topic", None).await.unwrap();
        let result = svc
            .transition_research(&sid, &res.id, ResearchStatus::Resolved)
            .await;

        assert!(matches!(result, Err(DatabaseError::InvalidState(_))));
    }

    #[tokio::test]
    async fn research_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let res = svc.create_research(&sid, "Audited", None).await.unwrap();

        let entries = svc
            .query_audit(&AuditFilter {
                entity_type: Some(EntityType::Research),
                entity_id: Some(res.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_id, res.id);
        assert_eq!(entries[0].action, AuditAction::Created);
    }
}

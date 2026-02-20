//! Insight repository — CRUD + FTS.

use chrono::Utc;

use zen_core::entities::{AuditEntry, Insight};
use zen_core::enums::{AuditAction, Confidence, EntityType, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_INSIGHT};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::insight::InsightUpdate;

fn row_to_insight(row: &libsql::Row) -> Result<Insight, DatabaseError> {
    Ok(Insight {
        id: row.get::<String>(0)?,
        research_id: get_opt_string(row, 1)?,
        session_id: get_opt_string(row, 2)?,
        content: row.get::<String>(3)?,
        confidence: parse_enum(&row.get::<String>(4)?)?,
        created_at: parse_datetime(&row.get::<String>(5)?)?,
        updated_at: parse_datetime(&row.get::<String>(6)?)?,
    })
}

impl ZenService {
    pub async fn create_insight(
        &self,
        session_id: &str,
        content: &str,
        confidence: Confidence,
        research_id: Option<&str>,
    ) -> Result<Insight, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_INSIGHT).await?;

        self.db().conn().execute(
            "INSERT INTO insights (id, research_id, session_id, content, confidence, created_at, updated_at, org_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            libsql::params![
                id.as_str(),
                research_id,
                session_id,
                content,
                confidence.as_str(),
                now.to_rfc3339(),
                now.to_rfc3339(),
                self.org_id()
            ],
        ).await?;

        let insight = Insight {
            id: id.clone(),
            research_id: research_id.map(String::from),
            session_id: Some(session_id.to_string()),
            content: content.to_string(),
            confidence,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Insight,
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
            entity: EntityType::Insight,
            id: id.clone(),
            data: serde_json::to_value(&insight).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(insight)
    }

    pub async fn get_insight(&self, id: &str) -> Result<Insight, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT id, research_id, session_id, content, confidence, created_at, updated_at
             FROM insights WHERE id = ?1",
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_insight(&row)
    }

    pub async fn update_insight(
        &self,
        session_id: &str,
        insight_id: &str,
        update: InsightUpdate,
    ) -> Result<Insight, DatabaseError> {
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref content) = update.content {
            sets.push(format!("content = ?{idx}"));
            params.push(content.as_str().into());
            idx += 1;
        }
        if let Some(ref confidence) = update.confidence {
            sets.push(format!("confidence = ?{idx}"));
            params.push(confidence.as_str().into());
            idx += 1;
        }

        if sets.is_empty() {
            return self.get_insight(insight_id).await;
        }

        sets.push(format!("updated_at = ?{idx}"));
        let now = Utc::now();
        params.push(now.to_rfc3339().into());
        idx += 1;

        params.push(insight_id.into());
        let id_idx = idx;
        idx += 1;
        let (org_filter, org_params) = self.org_id_filter(idx as u32);
        params.extend(org_params);
        let sql = format!("UPDATE insights SET {} WHERE id = ?{id_idx} {org_filter}", sets.join(", "));

        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(params))
            .await?;

        let insight = self.get_insight(insight_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Insight,
            entity_id: insight_id.to_string(),
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
            entity: EntityType::Insight,
            id: insight_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(insight)
    }

    pub async fn delete_insight(
        &self,
        session_id: &str,
        insight_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM insights WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![insight_id.into()];
        del_params.extend(org_params);
        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(del_params))
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Insight,
            entity_id: insight_id.to_string(),
            action: AuditAction::Deleted,
            detail: None,
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::Insight,
            id: insight_id.to_string(),
            data: serde_json::json!({}),
        })?;

        Ok(())
    }

    pub async fn list_insights(&self, limit: u32) -> Result<Vec<Insight>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT id, research_id, session_id, content, confidence, created_at, updated_at
             FROM insights WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(org_params)).await?;

        let mut insights = Vec::new();
        while let Some(row) = rows.next().await? {
            insights.push(row_to_insight(&row)?);
        }
        Ok(insights)
    }

    pub async fn search_insights(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Insight>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT i.id, i.research_id, i.session_id, i.content, i.confidence, i.created_at, i.updated_at
             FROM insights_fts
             JOIN insights i ON i.rowid = insights_fts.rowid
             WHERE insights_fts MATCH ?1 {org_filter}
             ORDER BY rank LIMIT ?2"
        );
        let mut params: Vec<libsql::Value> = vec![query.into(), (limit as i64).into()];
        params.extend(org_params);
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(params)).await?;

        let mut insights = Vec::new();
        while let Some(row) = rows.next().await? {
            insights.push(row_to_insight(&row)?);
        }
        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::insight::InsightUpdateBuilder;

    #[tokio::test]
    async fn create_insight_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let insight = svc
            .create_insight(
                &sid,
                "Combining findings suggests a pattern",
                Confidence::High,
                None,
            )
            .await
            .unwrap();

        assert!(insight.id.starts_with("ins-"));
        assert_eq!(insight.content, "Combining findings suggests a pattern");
        assert_eq!(insight.confidence, Confidence::High);
        assert_eq!(insight.session_id.as_deref(), Some(sid.as_str()));
        assert!(insight.research_id.is_none());

        let fetched = svc.get_insight(&insight.id).await.unwrap();
        assert_eq!(fetched.id, insight.id);
        assert_eq!(fetched.content, insight.content);
        assert_eq!(fetched.confidence, insight.confidence);
    }

    #[tokio::test]
    async fn update_insight_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let insight = svc
            .create_insight(&sid, "original insight", Confidence::Low, None)
            .await
            .unwrap();

        let update = InsightUpdateBuilder::new()
            .content("updated insight")
            .build();
        let updated = svc.update_insight(&sid, &insight.id, update).await.unwrap();

        assert_eq!(updated.content, "updated insight");
        assert_eq!(updated.confidence, Confidence::Low);
    }

    #[tokio::test]
    async fn delete_insight() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let insight = svc
            .create_insight(&sid, "to delete", Confidence::Medium, None)
            .await
            .unwrap();

        svc.delete_insight(&sid, &insight.id).await.unwrap();

        let result = svc.get_insight(&insight.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_insights() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        for i in 0..3 {
            svc.create_insight(&sid, &format!("insight {i}"), Confidence::Medium, None)
                .await
                .unwrap();
        }

        let insights = svc.list_insights(10).await.unwrap();
        assert_eq!(insights.len(), 3);
    }

    #[tokio::test]
    async fn search_insight_fts() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_insight(
            &sid,
            "work-stealing scheduler improves throughput",
            Confidence::High,
            None,
        )
        .await
        .unwrap();
        svc.create_insight(
            &sid,
            "serde is fast at deserialization",
            Confidence::Medium,
            None,
        )
        .await
        .unwrap();

        let results = svc.search_insights("throughput", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("throughput"));
    }

    #[tokio::test]
    async fn insight_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let insight = svc
            .create_insight(&sid, "audited insight", Confidence::Medium, None)
            .await
            .unwrap();

        let audits = svc
            .query_audit(&AuditFilter {
                entity_id: Some(insight.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].entity_type, EntityType::Insight);
    }
}

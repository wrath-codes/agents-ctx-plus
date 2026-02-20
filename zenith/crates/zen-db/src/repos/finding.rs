//! Finding repository — CRUD + FTS + tag management.

use chrono::Utc;

use zen_core::audit_detail::TaggedDetail;
use zen_core::entities::{AuditEntry, Finding};
use zen_core::enums::{AuditAction, Confidence, EntityType, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_FINDING};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::finding::FindingUpdate;

fn row_to_finding(row: &libsql::Row) -> Result<Finding, DatabaseError> {
    Ok(Finding {
        id: row.get::<String>(0)?,
        research_id: get_opt_string(row, 1)?,
        session_id: get_opt_string(row, 2)?,
        content: row.get::<String>(3)?,
        source: get_opt_string(row, 4)?,
        confidence: parse_enum(&row.get::<String>(5)?)?,
        created_at: parse_datetime(&row.get::<String>(6)?)?,
        updated_at: parse_datetime(&row.get::<String>(7)?)?,
    })
}

impl ZenService {
    pub async fn create_finding(
        &self,
        session_id: &str,
        content: &str,
        source: Option<&str>,
        confidence: Confidence,
        research_id: Option<&str>,
    ) -> Result<Finding, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_FINDING).await?;

        self.db().execute_with(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at, org_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            || libsql::params![
                id.as_str(),
                research_id,
                session_id,
                content,
                source,
                confidence.as_str(),
                now.to_rfc3339(),
                now.to_rfc3339(),
                self.org_id()
            ],
        ).await?;

        let finding = Finding {
            id: id.clone(),
            research_id: research_id.map(String::from),
            session_id: Some(session_id.to_string()),
            content: content.to_string(),
            source: source.map(String::from),
            confidence,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
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
            entity: EntityType::Finding,
            id: id.clone(),
            data: serde_json::to_value(&finding).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(finding)
    }

    pub async fn get_finding(&self, id: &str) -> Result<Finding, DatabaseError> {
        let mut rows = self.db().query(
            "SELECT id, research_id, session_id, content, source, confidence, created_at, updated_at
             FROM findings WHERE id = ?1",
            [id],
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_finding(&row)
    }

    pub async fn update_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        update: FindingUpdate,
    ) -> Result<Finding, DatabaseError> {
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref content) = update.content {
            sets.push(format!("content = ?{idx}"));
            params.push(content.as_str().into());
            idx += 1;
        }
        if let Some(ref source) = update.source {
            sets.push(format!("source = ?{idx}"));
            params.push(source.as_deref().into());
            idx += 1;
        }
        if let Some(ref confidence) = update.confidence {
            sets.push(format!("confidence = ?{idx}"));
            params.push(confidence.as_str().into());
            idx += 1;
        }

        if sets.is_empty() {
            return self.get_finding(finding_id).await;
        }

        sets.push(format!("updated_at = ?{idx}"));
        let now = Utc::now();
        params.push(now.to_rfc3339().into());
        idx += 1;

        params.push(finding_id.into());
        let id_idx = idx;
        idx += 1;
        let (org_filter, org_params) = self.org_id_filter(idx as u32);
        params.extend(org_params);
        let sql = format!("UPDATE findings SET {} WHERE id = ?{id_idx} {org_filter}", sets.join(", "));

        self.db()
            .execute_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let finding = self.get_finding(finding_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: finding_id.to_string(),
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
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(finding)
    }

    pub async fn delete_finding(
        &self,
        session_id: &str,
        finding_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        self.db()
            .execute(
                "DELETE FROM finding_tags WHERE finding_id = ?1",
                [finding_id],
            )
            .await?;

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM findings WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![finding_id.into()];
        del_params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(del_params.clone()))
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: finding_id.to_string(),
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
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::json!({}),
        })?;

        Ok(())
    }

    pub async fn list_findings(&self, limit: u32) -> Result<Vec<Finding>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT id, research_id, session_id, content, source, confidence, created_at, updated_at
             FROM findings WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().query_with(&sql, || libsql::params_from_iter(org_params.clone())).await?;

        let mut findings = Vec::new();
        while let Some(row) = rows.next().await? {
            findings.push(row_to_finding(&row)?);
        }
        Ok(findings)
    }

    pub async fn search_findings(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Finding>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT f.id, f.research_id, f.session_id, f.content, f.source, f.confidence, f.created_at, f.updated_at
             FROM findings_fts
             JOIN findings f ON f.rowid = findings_fts.rowid
             WHERE findings_fts MATCH ?1 {org_filter}
             ORDER BY rank LIMIT ?2"
        );
        let mut params: Vec<libsql::Value> = vec![query.into(), (limit as i64).into()];
        params.extend(org_params);
        let mut rows = self.db().query_with(&sql, || libsql::params_from_iter(params.clone())).await?;

        let mut findings = Vec::new();
        while let Some(row) = rows.next().await? {
            findings.push(row_to_finding(&row)?);
        }
        Ok(findings)
    }

    pub async fn tag_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        tag: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        self.db()
            .execute_with(
                "INSERT OR IGNORE INTO finding_tags (finding_id, tag) VALUES (?1, ?2)",
                || libsql::params![finding_id, tag],
            )
            .await?;

        let detail = TaggedDetail {
            tag: tag.to_string(),
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: finding_id.to_string(),
            action: AuditAction::Tagged,
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
            op: TrailOp::Tag,
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(())
    }

    pub async fn untag_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        tag: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        self.db()
            .execute_with(
                "DELETE FROM finding_tags WHERE finding_id = ?1 AND tag = ?2",
                || libsql::params![finding_id, tag],
            )
            .await?;

        let detail = TaggedDetail {
            tag: tag.to_string(),
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: finding_id.to_string(),
            action: AuditAction::Untagged,
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
            op: TrailOp::Untag,
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(())
    }

    pub async fn get_finding_tags(&self, finding_id: &str) -> Result<Vec<String>, DatabaseError> {
        let mut rows = self
            .db()
            .query(
                "SELECT tag FROM finding_tags WHERE finding_id = ?1 ORDER BY tag",
                [finding_id],
            )
            .await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(row.get::<String>(0)?);
        }
        Ok(tags)
    }

    pub async fn list_finding_ids_by_tag(&self, tag: &str) -> Result<Vec<String>, DatabaseError> {
        let mut rows = self
            .db()
            .query(
                "SELECT finding_id FROM finding_tags WHERE tag = ?1 ORDER BY finding_id",
                [tag],
            )
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(row.get::<String>(0)?);
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::finding::FindingUpdateBuilder;

    #[tokio::test]
    async fn create_finding_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(
                &sid,
                "Tokio uses work-stealing scheduler",
                Some("docs.rs"),
                Confidence::High,
                None,
            )
            .await
            .unwrap();

        assert!(finding.id.starts_with("fnd-"));
        assert_eq!(finding.content, "Tokio uses work-stealing scheduler");
        assert_eq!(finding.source.as_deref(), Some("docs.rs"));
        assert_eq!(finding.confidence, Confidence::High);
        assert_eq!(finding.session_id.as_deref(), Some(sid.as_str()));
        assert!(finding.research_id.is_none());

        let fetched = svc.get_finding(&finding.id).await.unwrap();
        assert_eq!(fetched.id, finding.id);
        assert_eq!(fetched.content, finding.content);
        assert_eq!(fetched.source, finding.source);
        assert_eq!(fetched.confidence, finding.confidence);
    }

    #[tokio::test]
    async fn update_finding_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "original content", Some("src"), Confidence::Low, None)
            .await
            .unwrap();

        let update = FindingUpdateBuilder::new()
            .content("updated content")
            .build();
        let updated = svc.update_finding(&sid, &finding.id, update).await.unwrap();

        assert_eq!(updated.content, "updated content");
        assert_eq!(updated.source.as_deref(), Some("src"));
        assert_eq!(updated.confidence, Confidence::Low);
    }

    #[tokio::test]
    async fn update_finding_set_source_null() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(
                &sid,
                "content",
                Some("will-be-removed"),
                Confidence::Medium,
                None,
            )
            .await
            .unwrap();

        let update = FindingUpdateBuilder::new().source(None).build();
        let updated = svc.update_finding(&sid, &finding.id, update).await.unwrap();

        assert!(updated.source.is_none());
    }

    #[tokio::test]
    async fn delete_finding() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "to delete", None, Confidence::Low, None)
            .await
            .unwrap();

        svc.delete_finding(&sid, &finding.id).await.unwrap();

        let result = svc.get_finding(&finding.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_findings() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        for i in 0..3 {
            svc.create_finding(
                &sid,
                &format!("finding {i}"),
                None,
                Confidence::Medium,
                None,
            )
            .await
            .unwrap();
        }

        let findings = svc.list_findings(10).await.unwrap();
        assert_eq!(findings.len(), 3);
    }

    #[tokio::test]
    async fn search_finding_fts() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_finding(&sid, "tokio async runtime", None, Confidence::High, None)
            .await
            .unwrap();
        svc.create_finding(&sid, "serde serialization", None, Confidence::Medium, None)
            .await
            .unwrap();

        let results = svc.search_findings("runtime", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("runtime"));
    }

    #[tokio::test]
    async fn tag_finding_and_get_tags() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "tagged finding", None, Confidence::Medium, None)
            .await
            .unwrap();

        svc.tag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();

        let tags = svc.get_finding_tags(&finding.id).await.unwrap();
        assert_eq!(tags, vec!["verified"]);
    }

    #[tokio::test]
    async fn list_finding_ids_by_tag_returns_matching_ids() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding_a = svc
            .create_finding(&sid, "first tagged finding", None, Confidence::Medium, None)
            .await
            .unwrap();
        let finding_b = svc
            .create_finding(
                &sid,
                "second tagged finding",
                None,
                Confidence::Medium,
                None,
            )
            .await
            .unwrap();
        let finding_c = svc
            .create_finding(&sid, "untagged finding", None, Confidence::Medium, None)
            .await
            .unwrap();

        svc.tag_finding(&sid, &finding_a.id, "verified")
            .await
            .unwrap();
        svc.tag_finding(&sid, &finding_b.id, "verified")
            .await
            .unwrap();
        svc.tag_finding(&sid, &finding_c.id, "other").await.unwrap();

        let mut ids = svc.list_finding_ids_by_tag("verified").await.unwrap();
        ids.sort();

        let mut expected = vec![finding_a.id, finding_b.id];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[tokio::test]
    async fn tag_finding_idempotent() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "idempotent tag", None, Confidence::Medium, None)
            .await
            .unwrap();

        svc.tag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();
        svc.tag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();

        let tags = svc.get_finding_tags(&finding.id).await.unwrap();
        assert_eq!(tags, vec!["verified"]);
    }

    #[tokio::test]
    async fn untag_finding() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "will untag", None, Confidence::Medium, None)
            .await
            .unwrap();

        svc.tag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();
        svc.untag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();

        let tags = svc.get_finding_tags(&finding.id).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn finding_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "audited finding", None, Confidence::Medium, None)
            .await
            .unwrap();

        let audits = svc
            .query_audit(&AuditFilter {
                entity_id: Some(finding.id.clone()),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].entity_type, EntityType::Finding);
    }
}

//! Entity link repository — CRUD for cross-entity relationships.

use chrono::Utc;

use zen_core::audit_detail::LinkedDetail;
use zen_core::entities::{AuditEntry, EntityLink};
use zen_core::enums::{AuditAction, EntityType, Relation, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_LINK};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{parse_datetime, parse_enum};
use crate::service::ZenService;

fn row_to_link(row: &libsql::Row) -> Result<EntityLink, DatabaseError> {
    Ok(EntityLink {
        id: row.get::<String>(0)?,
        source_type: parse_enum(&row.get::<String>(1)?)?,
        source_id: row.get::<String>(2)?,
        target_type: parse_enum(&row.get::<String>(3)?)?,
        target_id: row.get::<String>(4)?,
        relation: parse_enum(&row.get::<String>(5)?)?,
        created_at: parse_datetime(&row.get::<String>(6)?)?,
    })
}

impl ZenService {
    pub async fn create_link(
        &self,
        session_id: &str,
        source_type: EntityType,
        source_id: &str,
        target_type: EntityType,
        target_id: &str,
        relation: Relation,
    ) -> Result<EntityLink, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_LINK).await?;

        self.db().conn().execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            libsql::params![
                id.as_str(),
                source_type.as_str(),
                source_id,
                target_type.as_str(),
                target_id,
                relation.as_str(),
                now.to_rfc3339()
            ],
        ).await?;

        let link = EntityLink {
            id: id.clone(),
            source_type,
            source_id: source_id.to_string(),
            target_type,
            target_id: target_id.to_string(),
            relation,
            created_at: now,
        };

        let detail = LinkedDetail {
            source_type: source_type.as_str().to_string(),
            source_id: source_id.to_string(),
            target_type: target_type.as_str().to_string(),
            target_id: target_id.to_string(),
            relation: relation.as_str().to_string(),
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::EntityLink,
            entity_id: id.clone(),
            action: AuditAction::Linked,
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
            op: TrailOp::Link,
            entity: EntityType::EntityLink,
            id: id.clone(),
            data: serde_json::to_value(&link).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(link)
    }

    pub async fn get_link(&self, id: &str) -> Result<EntityLink, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT id, source_type, source_id, target_type, target_id, relation, created_at
             FROM entity_links WHERE id = ?1",
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_link(&row)
    }

    pub async fn delete_link(&self, session_id: &str, link_id: &str) -> Result<(), DatabaseError> {
        let link = self.get_link(link_id).await?;
        let now = Utc::now();

        self.db()
            .conn()
            .execute("DELETE FROM entity_links WHERE id = ?1", [link_id])
            .await?;

        let detail = LinkedDetail {
            source_type: link.source_type.as_str().to_string(),
            source_id: link.source_id.clone(),
            target_type: link.target_type.as_str().to_string(),
            target_id: link.target_id.clone(),
            relation: link.relation.as_str().to_string(),
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::EntityLink,
            entity_id: link_id.to_string(),
            action: AuditAction::Unlinked,
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
            op: TrailOp::Unlink,
            entity: EntityType::EntityLink,
            id: link_id.to_string(),
            data: serde_json::to_value(&link).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(())
    }

    pub async fn get_links_from(
        &self,
        source_type: EntityType,
        source_id: &str,
    ) -> Result<Vec<EntityLink>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT id, source_type, source_id, target_type, target_id, relation, created_at
             FROM entity_links WHERE source_type = ?1 AND source_id = ?2",
                libsql::params![source_type.as_str(), source_id],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(row) = rows.next().await? {
            links.push(row_to_link(&row)?);
        }
        Ok(links)
    }

    pub async fn get_links_to(
        &self,
        target_type: EntityType,
        target_id: &str,
    ) -> Result<Vec<EntityLink>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT id, source_type, source_id, target_type, target_id, relation, created_at
             FROM entity_links WHERE target_type = ?1 AND target_id = ?2",
                libsql::params![target_type.as_str(), target_id],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(row) = rows.next().await? {
            links.push(row_to_link(&row)?);
        }
        Ok(links)
    }

    pub async fn get_linked_ids(
        &self,
        source_type: EntityType,
        source_id: &str,
        target_type: EntityType,
    ) -> Result<Vec<String>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT target_id FROM entity_links
             WHERE source_type = ?1 AND source_id = ?2 AND target_type = ?3",
                libsql::params![source_type.as_str(), source_id, target_type.as_str()],
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

    #[tokio::test]
    async fn create_link_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let link = svc
            .create_link(
                &sid,
                EntityType::Finding,
                "fnd-00000001",
                EntityType::Hypothesis,
                "hyp-00000001",
                Relation::Validates,
            )
            .await
            .unwrap();

        assert!(link.id.starts_with("lnk-"));
        assert_eq!(link.source_type, EntityType::Finding);
        assert_eq!(link.source_id, "fnd-00000001");
        assert_eq!(link.target_type, EntityType::Hypothesis);
        assert_eq!(link.target_id, "hyp-00000001");
        assert_eq!(link.relation, Relation::Validates);

        let fetched = svc.get_link(&link.id).await.unwrap();
        assert_eq!(fetched.id, link.id);
        assert_eq!(fetched.source_type, link.source_type);
        assert_eq!(fetched.target_id, link.target_id);
    }

    #[tokio::test]
    async fn delete_link() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let link = svc
            .create_link(
                &sid,
                EntityType::Study,
                "stu-1",
                EntityType::Hypothesis,
                "hyp-1",
                Relation::RelatesTo,
            )
            .await
            .unwrap();

        svc.delete_link(&sid, &link.id).await.unwrap();

        let result = svc.get_link(&link.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn get_links_from() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_link(
            &sid,
            EntityType::Study,
            "stu-1",
            EntityType::Hypothesis,
            "hyp-1",
            Relation::RelatesTo,
        )
        .await
        .unwrap();
        svc.create_link(
            &sid,
            EntityType::Study,
            "stu-1",
            EntityType::Finding,
            "fnd-1",
            Relation::RelatesTo,
        )
        .await
        .unwrap();

        let links = svc
            .get_links_from(EntityType::Study, "stu-1")
            .await
            .unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn get_links_to() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_link(
            &sid,
            EntityType::Study,
            "stu-1",
            EntityType::Hypothesis,
            "hyp-1",
            Relation::RelatesTo,
        )
        .await
        .unwrap();
        svc.create_link(
            &sid,
            EntityType::Finding,
            "fnd-1",
            EntityType::Hypothesis,
            "hyp-1",
            Relation::Validates,
        )
        .await
        .unwrap();

        let links = svc
            .get_links_to(EntityType::Hypothesis, "hyp-1")
            .await
            .unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn get_linked_ids() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_link(
            &sid,
            EntityType::Study,
            "stu-1",
            EntityType::Hypothesis,
            "hyp-1",
            Relation::RelatesTo,
        )
        .await
        .unwrap();
        svc.create_link(
            &sid,
            EntityType::Study,
            "stu-1",
            EntityType::Hypothesis,
            "hyp-2",
            Relation::RelatesTo,
        )
        .await
        .unwrap();

        let ids = svc
            .get_linked_ids(EntityType::Study, "stu-1", EntityType::Hypothesis)
            .await
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"hyp-1".to_string()));
        assert!(ids.contains(&"hyp-2".to_string()));
    }

    #[tokio::test]
    async fn link_unique_constraint() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_link(
            &sid,
            EntityType::Finding,
            "fnd-1",
            EntityType::Hypothesis,
            "hyp-1",
            Relation::Validates,
        )
        .await
        .unwrap();

        let result = svc
            .create_link(
                &sid,
                EntityType::Finding,
                "fnd-1",
                EntityType::Hypothesis,
                "hyp-1",
                Relation::Validates,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn link_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let link = svc
            .create_link(
                &sid,
                EntityType::Study,
                "stu-1",
                EntityType::Hypothesis,
                "hyp-1",
                Relation::RelatesTo,
            )
            .await
            .unwrap();

        let audits = svc
            .query_audit(&AuditFilter {
                entity_id: Some(link.id.clone()),
                action: Some(AuditAction::Linked),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(audits.len(), 1);
        let detail = audits[0].detail.as_ref().unwrap();
        let linked: LinkedDetail = serde_json::from_value(detail.clone()).unwrap();
        assert_eq!(linked.source_type, "study");
        assert_eq!(linked.target_type, "hypothesis");
        assert_eq!(linked.relation, "relates_to");
    }
}

//! Study repository — CRUD + FTS + status transitions + composite methods.
//!
//! Studies are the most complex entity: they use `entity_links` extensively
//! for hypotheses (assumptions), findings (test results), and insights (conclusions).

use chrono::Utc;

use zen_core::audit_detail::StatusChangedDetail;
use zen_core::entities::{AuditEntry, Study};
use zen_core::enums::{
    AuditAction, Confidence, EntityType, Relation, StudyMethodology, StudyStatus, TrailOp,
};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_STUDY};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::study::StudyUpdate;

fn row_to_study(row: &libsql::Row) -> Result<Study, DatabaseError> {
    Ok(Study {
        id: row.get(0)?,
        session_id: get_opt_string(row, 1)?,
        research_id: get_opt_string(row, 2)?,
        topic: row.get(3)?,
        library: get_opt_string(row, 4)?,
        methodology: parse_enum(&row.get::<String>(5)?)?,
        status: parse_enum(&row.get::<String>(6)?)?,
        summary: get_opt_string(row, 7)?,
        created_at: parse_datetime(&row.get::<String>(8)?)?,
        updated_at: parse_datetime(&row.get::<String>(9)?)?,
    })
}

const STUDY_COLS: &str = "id, session_id, research_id, topic, library, methodology, status, summary, created_at, updated_at";

/// Full study state including linked entities.
#[derive(Debug)]
pub struct StudyFullState {
    pub study: Study,
    pub assumptions: Vec<StudyHypothesis>,
    pub findings: Vec<StudyFinding>,
    pub conclusions: Vec<StudyInsight>,
}

#[derive(Debug)]
pub struct StudyHypothesis {
    pub id: String,
    pub content: String,
    pub status: String,
}

#[derive(Debug)]
pub struct StudyFinding {
    pub id: String,
    pub content: String,
    pub confidence: String,
}

#[derive(Debug)]
pub struct StudyInsight {
    pub id: String,
    pub content: String,
    pub confidence: String,
}

/// Hypothesis progress counts for a study.
#[derive(Debug)]
pub struct StudyProgress {
    pub total: i64,
    pub confirmed: i64,
    pub debunked: i64,
    pub untested: i64,
}

impl ZenService {
    pub async fn create_study(
        &self,
        session_id: &str,
        topic: &str,
        library: Option<&str>,
        methodology: StudyMethodology,
        research_id: Option<&str>,
    ) -> Result<Study, DatabaseError> {
        let id = self.db().generate_id(PREFIX_STUDY).await?;
        let now = Utc::now();

        self.db()
            .conn()
            .execute(
                "INSERT INTO studies (id, session_id, research_id, topic, library, methodology, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8)",
                libsql::params![
                    id.as_str(),
                    session_id,
                    research_id,
                    topic,
                    library,
                    methodology.as_str(),
                    now.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )
            .await?;

        let study = Study {
            id: id.clone(),
            session_id: Some(session_id.to_string()),
            research_id: research_id.map(String::from),
            topic: topic.to_string(),
            library: library.map(String::from),
            methodology,
            status: StudyStatus::Active,
            summary: None,
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Study,
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
            entity: EntityType::Study,
            id,
            data: serde_json::to_value(&study).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(study)
    }

    pub async fn get_study(&self, id: &str) -> Result<Study, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                &format!("SELECT {STUDY_COLS} FROM studies WHERE id = ?1"),
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_study(&row)
    }

    pub async fn update_study(
        &self,
        session_id: &str,
        study_id: &str,
        update: StudyUpdate,
    ) -> Result<Study, DatabaseError> {
        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref topic) = update.topic {
            sets.push(format!("topic = ?{idx}"));
            params.push(topic.as_str().into());
            idx += 1;
        }
        if let Some(ref library) = update.library {
            sets.push(format!("library = ?{idx}"));
            params.push(library.as_deref().into());
            idx += 1;
        }
        if let Some(ref methodology) = update.methodology {
            sets.push(format!("methodology = ?{idx}"));
            params.push(methodology.as_str().into());
            idx += 1;
        }
        if let Some(ref status) = update.status {
            sets.push(format!("status = ?{idx}"));
            params.push(status.as_str().into());
            idx += 1;
        }
        if let Some(ref summary) = update.summary {
            sets.push(format!("summary = ?{idx}"));
            params.push(summary.as_deref().into());
            idx += 1;
        }

        if sets.is_empty() {
            return self.get_study(study_id).await;
        }

        sets.push(format!("updated_at = ?{idx}"));
        params.push(Utc::now().to_rfc3339().into());
        idx += 1;

        let sql = format!("UPDATE studies SET {} WHERE id = ?{idx}", sets.join(", "));
        params.push(study_id.into());

        self.db()
            .conn()
            .execute(&sql, libsql::params_from_iter(params))
            .await?;

        let now = Utc::now();
        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Study,
            entity_id: study_id.to_string(),
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
            entity: EntityType::Study,
            id: study_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        self.get_study(study_id).await
    }

    pub async fn delete_study(
        &self,
        session_id: &str,
        study_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        self.db()
            .conn()
            .execute("DELETE FROM studies WHERE id = ?1", [study_id])
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Study,
            entity_id: study_id.to_string(),
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
            entity: EntityType::Study,
            id: study_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    pub async fn list_studies(&self, limit: u32) -> Result<Vec<Study>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT {STUDY_COLS} FROM studies WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(org_params)).await?;

        let mut studies = Vec::new();
        while let Some(row) = rows.next().await? {
            studies.push(row_to_study(&row)?);
        }
        Ok(studies)
    }

    pub async fn search_studies(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Study>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT s.{STUDY_COLS} FROM studies_fts
             JOIN studies s ON s.rowid = studies_fts.rowid
             WHERE studies_fts MATCH ?1 {org_filter}
             ORDER BY rank LIMIT ?2",
            STUDY_COLS = "id, s.session_id, s.research_id, s.topic, s.library, s.methodology, s.status, s.summary, s.created_at, s.updated_at"
        );
        let mut params: Vec<libsql::Value> = vec![query.into(), (limit as i64).into()];
        params.extend(org_params);
        let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(params)).await?;

        let mut studies = Vec::new();
        while let Some(row) = rows.next().await? {
            studies.push(row_to_study(&row)?);
        }
        Ok(studies)
    }

    pub async fn transition_study(
        &self,
        session_id: &str,
        study_id: &str,
        new_status: StudyStatus,
    ) -> Result<Study, DatabaseError> {
        let current = self.get_study(study_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition study {} from {} to {}",
                study_id, current.status, new_status
            )));
        }

        let now = Utc::now();
        self.db()
            .conn()
            .execute(
                "UPDATE studies SET status = ?1, updated_at = ?2 WHERE id = ?3",
                libsql::params![new_status.as_str(), now.to_rfc3339(), study_id],
            )
            .await?;

        let detail = StatusChangedDetail {
            from: current.status.as_str().to_string(),
            to: new_status.as_str().to_string(),
            reason: None,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Study,
            entity_id: study_id.to_string(),
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
            entity: EntityType::Study,
            id: study_id.to_string(),
            data: serde_json::to_value(&detail).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        self.get_study(study_id).await
    }

    /// Add a hypothesis as a study assumption.
    ///
    /// Creates a hypothesis and links it to the study via entity_links.
    /// Returns the hypothesis ID.
    pub async fn add_assumption(
        &self,
        session_id: &str,
        study_id: &str,
        content: &str,
    ) -> Result<String, DatabaseError> {
        let hyp_id = self.db().generate_id("hyp").await?;
        let now = Utc::now();

        self.db()
            .conn()
            .execute(
                "INSERT INTO hypotheses (id, session_id, content, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'unverified', ?4, ?5)",
                libsql::params![
                    hyp_id.as_str(),
                    session_id,
                    content,
                    now.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Hypothesis,
            entity_id: hyp_id.clone(),
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
            id: hyp_id.clone(),
            data: serde_json::json!({
                "id": hyp_id,
                "session_id": session_id,
                "content": content,
                "status": "unverified",
            }),
        })?;

        self.create_link(
            session_id,
            EntityType::Study,
            study_id,
            EntityType::Hypothesis,
            &hyp_id,
            Relation::RelatesTo,
        )
        .await?;

        Ok(hyp_id)
    }

    /// Record a test result (finding) for a study hypothesis.
    ///
    /// Creates a finding and links it to both the study and hypothesis.
    /// Returns the finding ID.
    pub async fn record_test_result(
        &self,
        session_id: &str,
        study_id: &str,
        hypothesis_id: &str,
        content: &str,
        confidence: Confidence,
    ) -> Result<String, DatabaseError> {
        let fnd_id = self.db().generate_id("fnd").await?;
        let now = Utc::now();

        self.db()
            .conn()
            .execute(
                "INSERT INTO findings (id, session_id, content, confidence, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                libsql::params![
                    fnd_id.as_str(),
                    session_id,
                    content,
                    confidence.as_str(),
                    now.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: fnd_id.clone(),
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
            id: fnd_id.clone(),
            data: serde_json::json!({
                "id": fnd_id,
                "session_id": session_id,
                "content": content,
                "confidence": confidence.as_str(),
            }),
        })?;

        self.create_link(
            session_id,
            EntityType::Study,
            study_id,
            EntityType::Finding,
            &fnd_id,
            Relation::RelatesTo,
        )
        .await?;

        self.create_link(
            session_id,
            EntityType::Finding,
            &fnd_id,
            EntityType::Hypothesis,
            hypothesis_id,
            Relation::Validates,
        )
        .await?;

        Ok(fnd_id)
    }

    /// Conclude a study: transition to completed, set summary, create insight.
    ///
    /// Returns the updated study.
    pub async fn conclude_study(
        &self,
        session_id: &str,
        study_id: &str,
        summary: &str,
    ) -> Result<Study, DatabaseError> {
        let study = self.get_study(study_id).await?;

        if study.status == StudyStatus::Active {
            self.transition_study(session_id, study_id, StudyStatus::Concluding)
                .await?;
        }
        self.transition_study(session_id, study_id, StudyStatus::Completed)
            .await?;

        let update = crate::updates::study::StudyUpdateBuilder::new()
            .summary(Some(summary.to_string()))
            .build();
        self.update_study(session_id, study_id, update).await?;

        let ins_id = self.db().generate_id("ins").await?;
        let now = Utc::now();

        self.db()
            .conn()
            .execute(
                "INSERT INTO insights (id, session_id, content, confidence, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'high', ?4, ?5)",
                libsql::params![
                    ins_id.as_str(),
                    session_id,
                    summary,
                    now.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Insight,
            entity_id: ins_id.clone(),
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
            id: ins_id.clone(),
            data: serde_json::json!({
                "id": ins_id,
                "session_id": session_id,
                "content": summary,
                "confidence": "high",
            }),
        })?;

        self.create_link(
            session_id,
            EntityType::Study,
            study_id,
            EntityType::Insight,
            &ins_id,
            Relation::DerivedFrom,
        )
        .await?;

        self.get_study(study_id).await
    }

    /// Get full study state including linked hypotheses, findings, insights.
    pub async fn get_study_full_state(
        &self,
        study_id: &str,
    ) -> Result<StudyFullState, DatabaseError> {
        let study = self.get_study(study_id).await?;

        let hyp_ids = self
            .get_linked_ids(EntityType::Study, study_id, EntityType::Hypothesis)
            .await?;
        let fnd_ids = self
            .get_linked_ids(EntityType::Study, study_id, EntityType::Finding)
            .await?;
        let ins_ids = self
            .get_linked_ids(EntityType::Study, study_id, EntityType::Insight)
            .await?;

        let mut assumptions = Vec::new();
        for hid in &hyp_ids {
            let mut rows = self
                .db()
                .conn()
                .query(
                    "SELECT id, content, status FROM hypotheses WHERE id = ?1",
                    [hid.as_str()],
                )
                .await?;
            if let Some(row) = rows.next().await? {
                assumptions.push(StudyHypothesis {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    status: row.get(2)?,
                });
            }
        }

        let mut findings = Vec::new();
        for fid in &fnd_ids {
            let mut rows = self
                .db()
                .conn()
                .query(
                    "SELECT id, content, confidence FROM findings WHERE id = ?1",
                    [fid.as_str()],
                )
                .await?;
            if let Some(row) = rows.next().await? {
                findings.push(StudyFinding {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    confidence: row.get(2)?,
                });
            }
        }

        let mut conclusions = Vec::new();
        for iid in &ins_ids {
            let mut rows = self
                .db()
                .conn()
                .query(
                    "SELECT id, content, confidence FROM insights WHERE id = ?1",
                    [iid.as_str()],
                )
                .await?;
            if let Some(row) = rows.next().await? {
                conclusions.push(StudyInsight {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    confidence: row.get(2)?,
                });
            }
        }

        Ok(StudyFullState {
            study,
            assumptions,
            findings,
            conclusions,
        })
    }

    /// Progress tracking: count hypotheses by status for a study.
    pub async fn study_progress(&self, study_id: &str) -> Result<StudyProgress, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT
                    COUNT(*) as total,
                    SUM(CASE WHEN h.status = 'confirmed' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN h.status = 'debunked' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN h.status = 'unverified' THEN 1 ELSE 0 END)
                 FROM entity_links el
                 JOIN hypotheses h ON h.id = el.target_id
                 WHERE el.source_type = 'study' AND el.source_id = ?1
                   AND el.target_type = 'hypothesis'",
                [study_id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(StudyProgress {
            total: row.get(0)?,
            confirmed: row.get(1)?,
            debunked: row.get(2)?,
            untested: row.get(3)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::study::StudyUpdateBuilder;

    #[tokio::test]
    async fn create_study_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(
                &sid,
                "tokio runtime",
                Some("tokio"),
                StudyMethodology::TestDriven,
                None,
            )
            .await
            .unwrap();

        assert!(study.id.starts_with("stu-"));
        assert_eq!(study.topic, "tokio runtime");
        assert_eq!(study.library.as_deref(), Some("tokio"));
        assert_eq!(study.methodology, StudyMethodology::TestDriven);
        assert_eq!(study.status, StudyStatus::Active);
        assert!(study.summary.is_none());

        let fetched = svc.get_study(&study.id).await.unwrap();
        assert_eq!(fetched.topic, "tokio runtime");
        assert_eq!(fetched.methodology, StudyMethodology::TestDriven);
    }

    #[tokio::test]
    async fn update_study_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(
                &sid,
                "original topic",
                None,
                StudyMethodology::Explore,
                None,
            )
            .await
            .unwrap();

        let update = StudyUpdateBuilder::new().topic("updated topic").build();
        let updated = svc.update_study(&sid, &study.id, update).await.unwrap();

        assert_eq!(updated.topic, "updated topic");
        assert_eq!(updated.methodology, StudyMethodology::Explore);
    }

    #[tokio::test]
    async fn delete_study() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "test", None, StudyMethodology::Explore, None)
            .await
            .unwrap();

        svc.delete_study(&sid, &study.id).await.unwrap();

        let result = svc.get_study(&study.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_studies() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        for i in 0..3 {
            svc.create_study(
                &sid,
                &format!("study {i}"),
                None,
                StudyMethodology::Explore,
                None,
            )
            .await
            .unwrap();
        }

        let studies = svc.list_studies(10).await.unwrap();
        assert_eq!(studies.len(), 3);
    }

    #[tokio::test]
    async fn search_study_fts() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_study(
            &sid,
            "tokio async runtime",
            None,
            StudyMethodology::Explore,
            None,
        )
        .await
        .unwrap();
        svc.create_study(
            &sid,
            "database comparison",
            None,
            StudyMethodology::Compare,
            None,
        )
        .await
        .unwrap();

        let results = svc.search_studies("tokio", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].topic, "tokio async runtime");
    }

    #[tokio::test]
    async fn transition_study_valid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "test", None, StudyMethodology::Explore, None)
            .await
            .unwrap();

        let updated = svc
            .transition_study(&sid, &study.id, StudyStatus::Concluding)
            .await
            .unwrap();

        assert_eq!(updated.status, StudyStatus::Concluding);
    }

    #[tokio::test]
    async fn transition_study_invalid() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "test", None, StudyMethodology::Explore, None)
            .await
            .unwrap();

        let result = svc
            .transition_study(&sid, &study.id, StudyStatus::Completed)
            .await;

        assert!(matches!(result, Err(DatabaseError::InvalidState(_))));
    }

    #[tokio::test]
    async fn study_add_assumption() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "test study", None, StudyMethodology::TestDriven, None)
            .await
            .unwrap();

        let hyp_id = svc
            .add_assumption(&sid, &study.id, "tokio supports multi-threaded runtime")
            .await
            .unwrap();

        assert!(hyp_id.starts_with("hyp-"));

        let linked = svc
            .get_linked_ids(EntityType::Study, &study.id, EntityType::Hypothesis)
            .await
            .unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0], hyp_id);
    }

    #[tokio::test]
    async fn study_record_test_result() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "test study", None, StudyMethodology::TestDriven, None)
            .await
            .unwrap();

        let hyp_id = svc
            .add_assumption(&sid, &study.id, "hypothesis content")
            .await
            .unwrap();

        let fnd_id = svc
            .record_test_result(
                &sid,
                &study.id,
                &hyp_id,
                "confirmed via testing",
                Confidence::High,
            )
            .await
            .unwrap();

        assert!(fnd_id.starts_with("fnd-"));

        let study_findings = svc
            .get_linked_ids(EntityType::Study, &study.id, EntityType::Finding)
            .await
            .unwrap();
        assert_eq!(study_findings.len(), 1);
        assert_eq!(study_findings[0], fnd_id);

        let hyp_findings = svc
            .get_links_to(EntityType::Hypothesis, &hyp_id)
            .await
            .unwrap();
        assert!(
            hyp_findings
                .iter()
                .any(|l| l.source_id == fnd_id && l.relation == Relation::Validates)
        );
    }

    #[tokio::test]
    async fn study_conclude() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "conclude test", None, StudyMethodology::Explore, None)
            .await
            .unwrap();

        svc.add_assumption(&sid, &study.id, "test assumption")
            .await
            .unwrap();

        let concluded = svc
            .conclude_study(&sid, &study.id, "Study conclusion summary")
            .await
            .unwrap();

        assert_eq!(concluded.status, StudyStatus::Completed);
        assert_eq!(
            concluded.summary.as_deref(),
            Some("Study conclusion summary")
        );

        let insight_ids = svc
            .get_linked_ids(EntityType::Study, &study.id, EntityType::Insight)
            .await
            .unwrap();
        assert_eq!(insight_ids.len(), 1);
    }

    #[tokio::test]
    async fn study_progress_tracking() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(
                &sid,
                "progress test",
                None,
                StudyMethodology::TestDriven,
                None,
            )
            .await
            .unwrap();

        svc.add_assumption(&sid, &study.id, "assumption 1")
            .await
            .unwrap();
        svc.add_assumption(&sid, &study.id, "assumption 2")
            .await
            .unwrap();

        let progress = svc.study_progress(&study.id).await.unwrap();
        assert_eq!(progress.total, 2);
        assert_eq!(progress.untested, 2);
        assert_eq!(progress.confirmed, 0);
        assert_eq!(progress.debunked, 0);
    }

    #[tokio::test]
    async fn study_full_state() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(
                &sid,
                "full state test",
                None,
                StudyMethodology::TestDriven,
                None,
            )
            .await
            .unwrap();

        let hyp_id = svc
            .add_assumption(&sid, &study.id, "test assumption")
            .await
            .unwrap();

        svc.record_test_result(&sid, &study.id, &hyp_id, "test finding", Confidence::High)
            .await
            .unwrap();

        let state = svc.get_study_full_state(&study.id).await.unwrap();
        assert_eq!(state.study.id, study.id);
        assert_eq!(state.assumptions.len(), 1);
        assert_eq!(state.assumptions[0].content, "test assumption");
        assert_eq!(state.findings.len(), 1);
        assert_eq!(state.findings[0].content, "test finding");
    }

    #[tokio::test]
    async fn study_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let study = svc
            .create_study(&sid, "audit test", None, StudyMethodology::Explore, None)
            .await
            .unwrap();

        let audits = svc
            .query_audit(&AuditFilter {
                entity_id: Some(study.id),
                action: Some(AuditAction::Created),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].entity_type, EntityType::Study);
    }
}

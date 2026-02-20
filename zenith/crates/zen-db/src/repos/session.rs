//! Session repository.
//!
//! Manages session lifecycle: start, end, list, snapshot, orphan detection.

use chrono::Utc;

use zen_core::entities::{AuditEntry, Session, SessionSnapshot};
use zen_core::enums::{AuditAction, EntityType, SessionStatus, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_SESSION};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{
    entity_type_to_table, get_opt_string, parse_datetime, parse_enum, parse_optional_datetime,
};
use crate::service::ZenService;

impl ZenService {
    /// Start a new session. Detects orphaned active sessions first.
    ///
    /// Returns the new session and any previous active session that was abandoned.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if database operations fail.
    pub async fn start_session(&self) -> Result<(Session, Option<Session>), DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_SESSION).await?;

        let orphaned = self.detect_orphan_sessions().await?;
        for orphan in &orphaned {
            self.abandon_session(&orphan.id).await?;
        }

        self.db()
            .conn()
            .execute(
                "INSERT INTO sessions (id, started_at, status, org_id) VALUES (?1, ?2, 'active', ?3)",
                libsql::params![id.as_str(), now.to_rfc3339(), self.org_id()],
            )
            .await?;

        let session = Session {
            id: id.clone(),
            started_at: now,
            ended_at: None,
            status: SessionStatus::Active,
            summary: None,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(id.clone()),
            entity_type: EntityType::Session,
            entity_id: id.clone(),
            action: AuditAction::SessionStart,
            detail: None,
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: id.clone(),
            op: TrailOp::Create,
            entity: EntityType::Session,
            id: id.clone(),
            data: serde_json::to_value(&session).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok((session, orphaned.into_iter().next()))
    }

    /// End a session (wrap-up).
    ///
    /// Validates transition: `Active` → `WrappedUp`. Sets `ended_at` and `summary`.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::InvalidState` if the session is not in `Active` status.
    pub async fn end_session(
        &self,
        session_id: &str,
        summary: &str,
    ) -> Result<Session, DatabaseError> {
        let current = self.get_session(session_id).await?;

        if !current.status.can_transition_to(SessionStatus::WrappedUp) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition session {} from {} to wrapped_up",
                session_id, current.status
            )));
        }

        let now = Utc::now();
        self.db().conn().execute(
            "UPDATE sessions SET ended_at = ?1, status = 'wrapped_up', summary = ?2 WHERE id = ?3",
            libsql::params![now.to_rfc3339(), summary, session_id],
        ).await?;

        let updated = Session {
            ended_at: Some(now),
            status: SessionStatus::WrappedUp,
            summary: Some(summary.to_string()),
            ..current
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Session,
            entity_id: session_id.to_string(),
            action: AuditAction::SessionEnd,
            detail: None,
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Transition,
            entity: EntityType::Session,
            id: session_id.to_string(),
            data: serde_json::json!({
                "from": "active",
                "to": "wrapped_up",
                "summary": summary,
            }),
        })?;

        Ok(updated)
    }

    /// Get a session by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NoResult` if the session does not exist.
    pub async fn get_session(&self, id: &str) -> Result<Session, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT id, started_at, ended_at, status, summary FROM sessions WHERE id = ?1",
                [id],
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_session(&row)
    }

    /// List sessions, optionally filtered by status.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn list_sessions(
        &self,
        status: Option<SessionStatus>,
        limit: u32,
    ) -> Result<Vec<Session>, DatabaseError> {
        let mut sessions = Vec::new();

        let mut rows = match status {
            Some(s) => {
                let (org_filter, org_params) = self.org_id_filter(3);
                let sql = format!(
                    "SELECT id, started_at, ended_at, status, summary FROM sessions
                     WHERE status = ?1 {org_filter} ORDER BY started_at DESC LIMIT ?2"
                );
                let mut params: Vec<libsql::Value> = vec![s.as_str().into(), (limit as i64).into()];
                params.extend(org_params);
                self.db().conn().query(&sql, libsql::params_from_iter(params)).await?
            }
            None => {
                let (org_filter, org_params) = self.org_id_filter(1);
                let sql = format!(
                    "SELECT id, started_at, ended_at, status, summary FROM sessions
                     WHERE 1=1 {org_filter} ORDER BY started_at DESC LIMIT {limit}"
                );
                self.db().conn().query(&sql, libsql::params_from_iter(org_params)).await?
            }
        };

        while let Some(row) = rows.next().await? {
            sessions.push(row_to_session(&row)?);
        }

        Ok(sessions)
    }

    /// Create a session snapshot aggregating project state counts.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if aggregate queries or INSERT fails.
    pub async fn create_snapshot(
        &self,
        session_id: &str,
        summary: &str,
    ) -> Result<SessionSnapshot, DatabaseError> {
        let now = Utc::now();
        let open_tasks = self.count_by_status(EntityType::Task, "open").await?;
        let in_progress_tasks = self
            .count_by_status(EntityType::Task, "in_progress")
            .await?;
        let pending_hyps = self
            .count_by_status(EntityType::Hypothesis, "unverified")
            .await?;
        let unverified_hyps = self
            .count_by_status(EntityType::Hypothesis, "analyzing")
            .await?;
        let recent_findings = self.count_recent("findings", 24).await?;
        let open_research = self.count_by_status(EntityType::Research, "open").await?;

        self.db()
            .conn()
            .execute(
                "INSERT INTO session_snapshots
             (session_id, open_tasks, in_progress_tasks, pending_hypotheses,
              unverified_hypotheses, recent_findings, open_research, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                libsql::params![
                    session_id,
                    open_tasks,
                    in_progress_tasks,
                    pending_hyps,
                    unverified_hyps,
                    recent_findings,
                    open_research,
                    summary,
                    now.to_rfc3339()
                ],
            )
            .await?;

        Ok(SessionSnapshot {
            session_id: session_id.to_string(),
            open_tasks,
            in_progress_tasks,
            pending_hypotheses: pending_hyps,
            unverified_hypotheses: unverified_hyps,
            recent_findings,
            open_research,
            summary: summary.to_string(),
            created_at: now,
        })
    }

    /// Detect sessions in 'active' status (orphans from crashed sessions).
    async fn detect_orphan_sessions(&self) -> Result<Vec<Session>, DatabaseError> {
        self.list_sessions(Some(SessionStatus::Active), 10).await
    }

    /// Mark a session as abandoned.
    pub async fn abandon_session(&self, session_id: &str) -> Result<(), DatabaseError> {
        let now = Utc::now();
        self.db()
            .conn()
            .execute(
                "UPDATE sessions SET status = 'abandoned', ended_at = ?1 WHERE id = ?2",
                libsql::params![now.to_rfc3339(), session_id],
            )
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Session,
            entity_id: session_id.to_string(),
            action: AuditAction::StatusChanged,
            detail: Some(serde_json::json!({
                "from": "active",
                "to": "abandoned",
            })),
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Transition,
            entity: EntityType::Session,
            id: session_id.to_string(),
            data: serde_json::json!({
                "from": "active",
                "to": "abandoned",
            }),
        })?;

        Ok(())
    }

    /// Re-open a wrapped-up session if strict wrap-up sync failed.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if update or audit/trail append fails.
    pub async fn reopen_session_after_sync_failure(
        &self,
        session_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();
        self.db()
            .conn()
            .execute(
                "UPDATE sessions
                 SET ended_at = NULL, status = 'active', summary = NULL
                 WHERE id = ?1 AND status = 'wrapped_up'",
                [session_id],
            )
            .await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Session,
            entity_id: session_id.to_string(),
            action: AuditAction::StatusChanged,
            detail: Some(serde_json::json!({
                "from": "wrapped_up",
                "to": "active",
                "reason": "strict_sync_failed",
            })),
            created_at: now,
        })
        .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Transition,
            entity: EntityType::Session,
            id: session_id.to_string(),
            data: serde_json::json!({
                "from": "wrapped_up",
                "to": "active",
                "reason": "strict_sync_failed",
            }),
        })?;

        Ok(())
    }

    /// Count rows matching a status in a table.
    pub(crate) async fn count_by_status(
        &self,
        entity: EntityType,
        status: &str,
    ) -> Result<i64, DatabaseError> {
        let table = entity_type_to_table(&entity);
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE status = ?1");
        let mut rows = self.db().conn().query(&sql, [status]).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<i64>(0)?)
    }

    /// Count rows created in the last N hours.
    pub(crate) async fn count_recent(&self, table: &str, hours: u32) -> Result<i64, DatabaseError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {table} WHERE created_at >= datetime('now', '-{hours} hours')"
        );
        let mut rows = self.db().conn().query(&sql, ()).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<i64>(0)?)
    }
}

/// Convert a libSQL row to a `Session` struct.
fn row_to_session(row: &libsql::Row) -> Result<Session, DatabaseError> {
    Ok(Session {
        id: row.get::<String>(0)?,
        started_at: parse_datetime(&row.get::<String>(1)?)?,
        ended_at: parse_optional_datetime(get_opt_string(row, 2)?.as_deref())?,
        status: parse_enum(&row.get::<String>(3)?)?,
        summary: get_opt_string(row, 4)?,
    })
}

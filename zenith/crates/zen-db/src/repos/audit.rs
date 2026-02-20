//! Audit trail repository.
//!
//! Append-only audit entries recording every mutation. Supports dynamic
//! filtering and FTS5 search.

use zen_core::entities::AuditEntry;
use zen_core::enums::{AuditAction, EntityType};

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum, parse_optional_json};
use crate::service::ZenService;

/// Filter criteria for audit queries.
#[derive(Debug, Default)]
pub struct AuditFilter {
    pub entity_type: Option<EntityType>,
    pub entity_id: Option<String>,
    pub action: Option<AuditAction>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
}

impl ZenService {
    /// Append an audit entry. Called by every mutation method.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the INSERT fails.
    pub async fn append_audit(&self, entry: &AuditEntry) -> Result<(), DatabaseError> {
        self.db().execute_with(
            "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            || libsql::params![
                entry.id.as_str(),
                entry.session_id.as_deref(),
                entry.entity_type.as_str(),
                entry.entity_id.as_str(),
                entry.action.as_str(),
                entry.detail.as_ref().map(std::string::ToString::to_string).as_deref(),
                entry.created_at.to_rfc3339()
            ],
        ).await?;
        Ok(())
    }

    /// Query audit entries with optional filters.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn query_audit(
        &self,
        filter: &AuditFilter,
    ) -> Result<Vec<AuditEntry>, DatabaseError> {
        let mut conditions = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();

        if let Some(ref et) = filter.entity_type {
            params.push(libsql::Value::Text(et.as_str().to_string()));
            conditions.push(format!("entity_type = ?{}", params.len()));
        }
        if let Some(ref eid) = filter.entity_id {
            params.push(libsql::Value::Text(eid.clone()));
            conditions.push(format!("entity_id = ?{}", params.len()));
        }
        if let Some(ref action) = filter.action {
            params.push(libsql::Value::Text(action.as_str().to_string()));
            conditions.push(format!("action = ?{}", params.len()));
        }
        if let Some(ref sid) = filter.session_id {
            params.push(libsql::Value::Text(sid.clone()));
            conditions.push(format!("session_id = ?{}", params.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = filter.limit.unwrap_or(100);
        let sql = format!(
            "SELECT id, session_id, entity_type, entity_id, action, detail, created_at
             FROM audit_trail {where_clause}
             ORDER BY created_at DESC LIMIT {limit}"
        );

        let mut rows = self
            .db()
            .query_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;
        let mut entries = Vec::new();

        while let Some(row) = rows.next().await? {
            entries.push(AuditEntry {
                id: row.get::<String>(0)?,
                session_id: get_opt_string(&row, 1)?,
                entity_type: parse_enum(&row.get::<String>(2)?)?,
                entity_id: row.get::<String>(3)?,
                action: parse_enum(&row.get::<String>(4)?)?,
                detail: parse_optional_json(get_opt_string(&row, 5)?.as_deref())?,
                created_at: parse_datetime(&row.get::<String>(6)?)?,
            });
        }

        Ok(entries)
    }

    /// FTS5 search across audit entries.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub async fn search_audit(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, DatabaseError> {
        let mut rows = self.db().query_with(
            "SELECT a.id, a.session_id, a.entity_type, a.entity_id, a.action, a.detail, a.created_at
             FROM audit_fts
             JOIN audit_trail a ON a.rowid = audit_fts.rowid
             WHERE audit_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
            || libsql::params![query, limit],
        ).await?;

        let mut entries = Vec::new();
        while let Some(row) = rows.next().await? {
            entries.push(AuditEntry {
                id: row.get::<String>(0)?,
                session_id: get_opt_string(&row, 1)?,
                entity_type: parse_enum(&row.get::<String>(2)?)?,
                entity_id: row.get::<String>(3)?,
                action: parse_enum(&row.get::<String>(4)?)?,
                detail: parse_optional_json(get_opt_string(&row, 5)?.as_deref())?,
                created_at: parse_datetime(&row.get::<String>(6)?)?,
            });
        }

        Ok(entries)
    }
}

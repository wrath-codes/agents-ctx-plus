//! Compatibility check repository — CRUD + verdict transitions.

use chrono::Utc;

use zen_core::entities::{AuditEntry, CompatCheck};
use zen_core::enums::{AuditAction, CompatStatus, EntityType, TrailOp};
use zen_core::ids::{PREFIX_AUDIT, PREFIX_COMPAT};
use zen_core::trail::TrailOperation;

use crate::error::DatabaseError;
use crate::helpers::{get_opt_string, parse_datetime, parse_enum};
use crate::service::ZenService;
use crate::updates::compat::CompatUpdate;

fn row_to_compat(row: &libsql::Row) -> Result<CompatCheck, DatabaseError> {
    Ok(CompatCheck {
        id: row.get::<String>(0)?,
        package_a: row.get::<String>(1)?,
        package_b: row.get::<String>(2)?,
        status: parse_enum(&row.get::<String>(3)?)?,
        conditions: get_opt_string(row, 4)?,
        finding_id: get_opt_string(row, 5)?,
        session_id: get_opt_string(row, 6)?,
        created_at: parse_datetime(&row.get::<String>(7)?)?,
        updated_at: parse_datetime(&row.get::<String>(8)?)?,
    })
}

const SELECT_COLS: &str =
    "id, package_a, package_b, status, conditions, finding_id, session_id, created_at, updated_at";

impl ZenService {
    pub async fn create_compat(
        &self,
        session_id: &str,
        package_a: &str,
        package_b: &str,
        status: CompatStatus,
        conditions: Option<&str>,
        finding_id: Option<&str>,
    ) -> Result<CompatCheck, DatabaseError> {
        let now = Utc::now();
        let id = self.db().generate_id(PREFIX_COMPAT).await?;

        self.db()
            .execute_with(
                "INSERT INTO compatibility_checks (id, package_a, package_b, status, conditions, finding_id, session_id, created_at, updated_at, org_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                || libsql::params![
                    id.as_str(),
                    package_a,
                    package_b,
                    status.as_str(),
                    conditions,
                    finding_id,
                    session_id,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                    self.org_id()
                ],
            )
            .await?;

        let compat = CompatCheck {
            id: id.clone(),
            package_a: package_a.to_string(),
            package_b: package_b.to_string(),
            status,
            conditions: conditions.map(String::from),
            finding_id: finding_id.map(String::from),
            session_id: Some(session_id.to_string()),
            created_at: now,
            updated_at: now,
        };

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Compat,
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
            entity: EntityType::Compat,
            id: id.clone(),
            data: serde_json::to_value(&compat).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(compat)
    }

    pub async fn get_compat_by_id(&self, id: &str) -> Result<CompatCheck, DatabaseError> {
        let sql = format!("SELECT {SELECT_COLS} FROM compatibility_checks WHERE id = ?1");
        let mut rows = self.db().query(&sql, [id]).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        row_to_compat(&row)
    }

    pub async fn update_compat(
        &self,
        session_id: &str,
        compat_id: &str,
        update: CompatUpdate,
    ) -> Result<CompatCheck, DatabaseError> {
        let now = Utc::now();

        let mut sets = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();

        if let Some(ref status) = update.status {
            params.push(libsql::Value::Text(status.as_str().to_string()));
            sets.push(format!("status = ?{}", params.len()));
        }
        if let Some(ref conditions) = update.conditions {
            match conditions {
                Some(c) => params.push(libsql::Value::Text(c.clone())),
                None => params.push(libsql::Value::Null),
            }
            sets.push(format!("conditions = ?{}", params.len()));
        }
        if let Some(ref fid) = update.finding_id {
            match fid {
                Some(f) => params.push(libsql::Value::Text(f.clone())),
                None => params.push(libsql::Value::Null),
            }
            sets.push(format!("finding_id = ?{}", params.len()));
        }

        params.push(libsql::Value::Text(now.to_rfc3339()));
        sets.push(format!("updated_at = ?{}", params.len()));

        params.push(libsql::Value::Text(compat_id.to_string()));
        let id_pos = params.len();
        let (org_filter, org_params) = self.org_id_filter((params.len() + 1) as u32);
        params.extend(org_params);

        let sql = format!(
            "UPDATE compatibility_checks SET {} WHERE id = ?{id_pos} {org_filter}",
            sets.join(", "),
        );

        self.db()
            .execute_with(&sql, || libsql::params_from_iter(params.clone()))
            .await?;

        let updated = self.get_compat_by_id(compat_id).await?;

        let audit_id = self.db().generate_id(PREFIX_AUDIT).await?;
        self.append_audit(&AuditEntry {
            id: audit_id,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Compat,
            entity_id: compat_id.to_string(),
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
            entity: EntityType::Compat,
            id: compat_id.to_string(),
            data: serde_json::to_value(&update).map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(updated)
    }

    pub async fn delete_compat(
        &self,
        session_id: &str,
        compat_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        let (org_filter, org_params) = self.org_id_filter(2);
        let sql = format!("DELETE FROM compatibility_checks WHERE id = ?1 {org_filter}");
        let mut del_params: Vec<libsql::Value> = vec![compat_id.into()];
        del_params.extend(org_params);
        self.db()
            .execute_with(&sql, || libsql::params_from_iter(del_params.clone()))
            .await?;

        self.trail().append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Delete,
            entity: EntityType::Compat,
            id: compat_id.to_string(),
            data: serde_json::Value::Null,
        })?;

        Ok(())
    }

    pub async fn list_compat(&self, limit: u32) -> Result<Vec<CompatCheck>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(1);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM compatibility_checks WHERE 1=1 {org_filter} ORDER BY created_at DESC LIMIT {limit}"
        );
        let mut rows = self.db().query_with(&sql, || libsql::params_from_iter(org_params.clone())).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_compat(&row)?);
        }
        Ok(results)
    }

    pub async fn get_compat_by_packages(
        &self,
        package_a: &str,
        package_b: &str,
    ) -> Result<Option<CompatCheck>, DatabaseError> {
        let (org_filter, org_params) = self.org_id_filter(3);
        let sql = format!(
            "SELECT {SELECT_COLS} FROM compatibility_checks
             WHERE ((package_a = ?1 AND package_b = ?2) OR (package_a = ?2 AND package_b = ?1))
             {org_filter}
             LIMIT 1"
        );
        let mut params: Vec<libsql::Value> = vec![package_a.into(), package_b.into()];
        params.extend(org_params);
        let mut rows = self.db().query_with(&sql, || libsql::params_from_iter(params.clone())).await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row_to_compat(&row)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::audit::AuditFilter;
    use crate::test_support::helpers::{start_test_session, test_service};
    use crate::updates::compat::CompatUpdateBuilder;

    #[tokio::test]
    async fn create_compat_roundtrip() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:tokio:1.40",
                "rust:axum:0.8",
                CompatStatus::Compatible,
                Some("requires feature flag"),
                None,
            )
            .await
            .unwrap();

        assert!(c.id.starts_with("cmp-"));
        assert_eq!(c.package_a, "rust:tokio:1.40");
        assert_eq!(c.package_b, "rust:axum:0.8");
        assert_eq!(c.status, CompatStatus::Compatible);
        assert_eq!(c.conditions.as_deref(), Some("requires feature flag"));
        assert!(c.finding_id.is_none());
        assert_eq!(c.session_id.as_deref(), Some(sid.as_str()));

        let fetched = svc.get_compat_by_id(&c.id).await.unwrap();
        assert_eq!(fetched.id, c.id);
        assert_eq!(fetched.package_a, c.package_a);
        assert_eq!(fetched.package_b, c.package_b);
        assert_eq!(fetched.status, c.status);
        assert_eq!(fetched.conditions, c.conditions);
    }

    #[tokio::test]
    async fn update_compat_partial() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:a:1",
                "rust:b:2",
                CompatStatus::Unknown,
                None,
                None,
            )
            .await
            .unwrap();

        let updated = svc
            .update_compat(
                &sid,
                &c.id,
                CompatUpdateBuilder::new()
                    .status(CompatStatus::Incompatible)
                    .build(),
            )
            .await
            .unwrap();

        assert_eq!(updated.status, CompatStatus::Incompatible);
        assert!(updated.conditions.is_none());
    }

    #[tokio::test]
    async fn update_compat_set_conditions_null() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:a:1",
                "rust:b:2",
                CompatStatus::Conditional,
                Some("needs flag"),
                None,
            )
            .await
            .unwrap();
        assert!(c.conditions.is_some());

        let updated = svc
            .update_compat(
                &sid,
                &c.id,
                CompatUpdateBuilder::new().conditions(None).build(),
            )
            .await
            .unwrap();

        assert!(updated.conditions.is_none());
    }

    #[tokio::test]
    async fn delete_compat() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:a:1",
                "rust:b:2",
                CompatStatus::Unknown,
                None,
                None,
            )
            .await
            .unwrap();

        svc.delete_compat(&sid, &c.id).await.unwrap();

        let result = svc.get_compat_by_id(&c.id).await;
        assert!(matches!(result, Err(DatabaseError::NoResult)));
    }

    #[tokio::test]
    async fn list_compat() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        svc.create_compat(
            &sid,
            "rust:a:1",
            "rust:b:1",
            CompatStatus::Compatible,
            None,
            None,
        )
        .await
        .unwrap();
        svc.create_compat(
            &sid,
            "rust:c:1",
            "rust:d:1",
            CompatStatus::Incompatible,
            None,
            None,
        )
        .await
        .unwrap();

        let list = svc.list_compat(10).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn compat_package_pair_query() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:tokio:1.40",
                "rust:axum:0.8",
                CompatStatus::Compatible,
                None,
                None,
            )
            .await
            .unwrap();

        let found = svc
            .get_compat_by_packages("rust:axum:0.8", "rust:tokio:1.40")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, c.id);
    }

    #[tokio::test]
    async fn compat_audit_on_create() {
        let svc = test_service().await;
        let sid = start_test_session(&svc).await;

        let c = svc
            .create_compat(
                &sid,
                "rust:a:1",
                "rust:b:1",
                CompatStatus::Unknown,
                None,
                None,
            )
            .await
            .unwrap();

        let audits = svc
            .query_audit(&AuditFilter {
                entity_type: Some(EntityType::Compat),
                entity_id: Some(c.id.clone()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!audits.is_empty());
        assert_eq!(audits[0].action, AuditAction::Created);
        assert_eq!(audits[0].entity_id, c.id);
    }
}

//! Project metadata and dependency repository — CRUD.

use zen_core::entities::{ProjectDependency, ProjectMeta};

use crate::error::DatabaseError;
use crate::helpers::{parse_datetime, parse_optional_datetime};
use crate::service::ZenService;

fn row_to_meta(row: &libsql::Row) -> Result<ProjectMeta, DatabaseError> {
    Ok(ProjectMeta {
        key: row.get::<String>(0)?,
        value: row.get::<String>(1)?,
        updated_at: parse_datetime(&row.get::<String>(2)?)?,
    })
}

fn row_to_dependency(row: &libsql::Row) -> Result<ProjectDependency, DatabaseError> {
    let indexed_at_str = row.get::<Option<String>>(5)?;
    Ok(ProjectDependency {
        ecosystem: row.get::<String>(0)?,
        name: row.get::<String>(1)?,
        version: row.get::<Option<String>>(2)?,
        source: row.get::<String>(3)?,
        indexed: row.get::<i64>(4)? != 0,
        indexed_at: parse_optional_datetime(indexed_at_str.as_deref())?,
    })
}

impl ZenService {
    pub async fn set_meta(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        self.db()
            .conn()
            .execute(
                "INSERT INTO project_meta (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                libsql::params![key, value],
            )
            .await?;
        Ok(())
    }

    pub async fn get_meta(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT value FROM project_meta WHERE key = ?1",
                [key],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row.get::<String>(0)?)),
            None => Ok(None),
        }
    }

    pub async fn get_all_meta(&self) -> Result<Vec<ProjectMeta>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query("SELECT key, value, updated_at FROM project_meta ORDER BY key", ())
            .await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_meta(&row)?);
        }
        Ok(results)
    }

    pub async fn delete_meta(&self, key: &str) -> Result<(), DatabaseError> {
        self.db()
            .conn()
            .execute("DELETE FROM project_meta WHERE key = ?1", [key])
            .await?;
        Ok(())
    }

    pub async fn upsert_dependency(
        &self,
        dep: &ProjectDependency,
    ) -> Result<(), DatabaseError> {
        self.db()
            .conn()
            .execute(
                "INSERT INTO project_dependencies (ecosystem, name, version, source, indexed, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(ecosystem, name) DO UPDATE SET
                   version = ?3, source = ?4, indexed = ?5, indexed_at = ?6",
                libsql::params![
                    dep.ecosystem.as_str(),
                    dep.name.as_str(),
                    dep.version.as_deref(),
                    dep.source.as_str(),
                    dep.indexed,
                    dep.indexed_at.map(|dt| dt.to_rfc3339())
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn get_dependency(
        &self,
        ecosystem: &str,
        name: &str,
    ) -> Result<Option<ProjectDependency>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT ecosystem, name, version, source, indexed, indexed_at
                 FROM project_dependencies WHERE ecosystem = ?1 AND name = ?2",
                libsql::params![ecosystem, name],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row_to_dependency(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn list_dependencies(&self) -> Result<Vec<ProjectDependency>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT ecosystem, name, version, source, indexed, indexed_at
                 FROM project_dependencies ORDER BY ecosystem, name",
                (),
            )
            .await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_dependency(&row)?);
        }
        Ok(results)
    }

    pub async fn list_unindexed_dependencies(
        &self,
    ) -> Result<Vec<ProjectDependency>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT ecosystem, name, version, source, indexed, indexed_at
                 FROM project_dependencies WHERE indexed = FALSE ORDER BY ecosystem, name",
                (),
            )
            .await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_dependency(&row)?);
        }
        Ok(results)
    }

    pub async fn mark_indexed(
        &self,
        ecosystem: &str,
        name: &str,
    ) -> Result<(), DatabaseError> {
        self.db()
            .conn()
            .execute(
                "UPDATE project_dependencies SET indexed = TRUE, indexed_at = datetime('now')
                 WHERE ecosystem = ?1 AND name = ?2",
                libsql::params![ecosystem, name],
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::helpers::test_service;

    #[tokio::test]
    async fn set_and_get_meta() {
        let svc = test_service().await;
        svc.set_meta("name", "test").await.unwrap();
        let val = svc.get_meta("name").await.unwrap();
        assert_eq!(val.as_deref(), Some("test"));
    }

    #[tokio::test]
    async fn meta_upsert() {
        let svc = test_service().await;
        svc.set_meta("name", "v1").await.unwrap();
        svc.set_meta("name", "v2").await.unwrap();
        let val = svc.get_meta("name").await.unwrap();
        assert_eq!(val.as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn get_all_meta() {
        let svc = test_service().await;
        svc.set_meta("a", "1").await.unwrap();
        svc.set_meta("b", "2").await.unwrap();
        svc.set_meta("c", "3").await.unwrap();
        let all = svc.get_all_meta().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn delete_meta() {
        let svc = test_service().await;
        svc.set_meta("key", "val").await.unwrap();
        svc.delete_meta("key").await.unwrap();
        let val = svc.get_meta("key").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn upsert_dependency() {
        let svc = test_service().await;
        let dep = ProjectDependency {
            ecosystem: "rust".to_string(),
            name: "tokio".to_string(),
            version: Some("1.40".to_string()),
            source: "Cargo.toml".to_string(),
            indexed: false,
            indexed_at: None,
        };
        svc.upsert_dependency(&dep).await.unwrap();
        let fetched = svc.get_dependency("rust", "tokio").await.unwrap().unwrap();
        assert_eq!(fetched.ecosystem, "rust");
        assert_eq!(fetched.name, "tokio");
        assert_eq!(fetched.version.as_deref(), Some("1.40"));
        assert_eq!(fetched.source, "Cargo.toml");
        assert!(!fetched.indexed);
        assert!(fetched.indexed_at.is_none());
    }

    #[tokio::test]
    async fn dependency_upsert_update() {
        let svc = test_service().await;
        let dep = ProjectDependency {
            ecosystem: "rust".to_string(),
            name: "axum".to_string(),
            version: Some("0.7".to_string()),
            source: "Cargo.toml".to_string(),
            indexed: false,
            indexed_at: None,
        };
        svc.upsert_dependency(&dep).await.unwrap();

        let updated = ProjectDependency {
            version: Some("0.8".to_string()),
            ..dep
        };
        svc.upsert_dependency(&updated).await.unwrap();

        let fetched = svc.get_dependency("rust", "axum").await.unwrap().unwrap();
        assert_eq!(fetched.version.as_deref(), Some("0.8"));
    }

    #[tokio::test]
    async fn list_dependencies() {
        let svc = test_service().await;
        for (name, ver) in [("tokio", "1.40"), ("axum", "0.8"), ("serde", "1.0")] {
            svc.upsert_dependency(&ProjectDependency {
                ecosystem: "rust".to_string(),
                name: name.to_string(),
                version: Some(ver.to_string()),
                source: "Cargo.toml".to_string(),
                indexed: false,
                indexed_at: None,
            })
            .await
            .unwrap();
        }
        let all = svc.list_dependencies().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn list_unindexed_dependencies() {
        let svc = test_service().await;
        svc.upsert_dependency(&ProjectDependency {
            ecosystem: "rust".to_string(),
            name: "tokio".to_string(),
            version: Some("1.40".to_string()),
            source: "Cargo.toml".to_string(),
            indexed: true,
            indexed_at: Some(chrono::Utc::now()),
        })
        .await
        .unwrap();
        svc.upsert_dependency(&ProjectDependency {
            ecosystem: "rust".to_string(),
            name: "axum".to_string(),
            version: Some("0.8".to_string()),
            source: "Cargo.toml".to_string(),
            indexed: false,
            indexed_at: None,
        })
        .await
        .unwrap();

        let unindexed = svc.list_unindexed_dependencies().await.unwrap();
        assert_eq!(unindexed.len(), 1);
        assert_eq!(unindexed[0].name, "axum");
    }

    #[tokio::test]
    async fn mark_indexed() {
        let svc = test_service().await;
        svc.upsert_dependency(&ProjectDependency {
            ecosystem: "rust".to_string(),
            name: "tokio".to_string(),
            version: Some("1.40".to_string()),
            source: "Cargo.toml".to_string(),
            indexed: false,
            indexed_at: None,
        })
        .await
        .unwrap();

        svc.mark_indexed("rust", "tokio").await.unwrap();

        let fetched = svc.get_dependency("rust", "tokio").await.unwrap().unwrap();
        assert!(fetched.indexed);
        assert!(fetched.indexed_at.is_some());
    }
}

//! # zen-db
//!
//! libSQL database operations for Zenith state management.
//!
//! Handles all relational state: research items, findings, hypotheses,
//! insights, tasks, sessions, audit trail, and entity links.
//! Uses libSQL embedded replicas with Turso Cloud sync on wrap-up.
//!
//! Uses the `libsql` crate (C `SQLite` fork, v0.9.29) — provides native FTS5,
//! stable API, and Turso Cloud embedded replica support.

pub mod error;
pub mod helpers;
mod migrations;
pub mod repos;
pub mod service;
pub mod trail;

use error::DatabaseError;
use libsql::Builder;

/// Central database handle for all Zenith state operations.
///
/// Wraps a libSQL database and connection. Provides ID generation
/// and will host all repository methods in Phase 2.
pub struct ZenDb {
    #[allow(dead_code)]
    db: libsql::Database,
    conn: libsql::Connection,
}

impl ZenDb {
    /// Open a local-only database at the given path (no cloud sync).
    ///
    /// Runs migrations automatically on first open.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the database cannot be opened or
    /// migrations fail.
    pub async fn open_local(path: &str) -> Result<Self, DatabaseError> {
        let db = Builder::new_local(path).build().await?;
        let conn = db.connect()?;

        // Enable foreign keys (must be per-connection in SQLite)
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(|e| DatabaseError::Migration(format!("PRAGMA foreign_keys: {e}")))?;

        let zen_db = Self { db, conn };
        zen_db.run_migrations().await?;
        Ok(zen_db)
    }

    /// Access the underlying libSQL connection for direct queries.
    ///
    /// Repo methods (Phase 2) will use this internally.
    #[must_use]
    pub const fn conn(&self) -> &libsql::Connection {
        &self.conn
    }

    /// Generate a prefixed ID via libSQL. Returns e.g., `"fnd-a3f8b2c1"`.
    ///
    /// Uses `randomblob(4)` in SQL to produce 8-char hex, then prepends the prefix.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails or returns no rows.
    pub async fn generate_id(&self, prefix: &str) -> Result<String, DatabaseError> {
        let mut rows = self
            .conn
            .query(
                &format!("SELECT '{prefix}-' || lower(hex(randomblob(4)))"),
                (),
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<String>(0)?)
    }
}

// Spike modules — kept for reference and continued test execution
#[cfg(test)]
mod spike_libsql;

#[cfg(test)]
mod spike_libsql_sync;

#[cfg(test)]
mod spike_studies;

#[cfg(test)]
mod spike_jsonl;

#[cfg(test)]
mod spike_clerk_auth;

#[cfg(test)]
mod spike_catalog_visibility;

#[cfg(test)]
mod spike_decision_traces;

// Production tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Helper to create an in-memory database for testing.
    async fn test_db() -> ZenDb {
        ZenDb::open_local(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn open_local_creates_schema() {
        let db = test_db().await;

        // Verify content tables exist
        let tables = [
            "project_meta",
            "project_dependencies",
            "sessions",
            "session_snapshots",
            "research_items",
            "findings",
            "finding_tags",
            "hypotheses",
            "insights",
            "issues",
            "tasks",
            "implementation_log",
            "studies",
            "compatibility_checks",
            "entity_links",
            "audit_trail",
        ];
        for table in &tables {
            let mut rows = db
                .conn()
                .query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                    [*table],
                )
                .await
                .unwrap();
            let row = rows.next().await.unwrap();
            assert!(row.is_some(), "table '{table}' should exist");
        }
    }

    #[tokio::test]
    async fn fts5_tables_exist() {
        let db = test_db().await;

        let fts_tables = [
            "findings_fts",
            "hypotheses_fts",
            "insights_fts",
            "research_fts",
            "tasks_fts",
            "issues_fts",
            "studies_fts",
            "audit_fts",
        ];
        for table in &fts_tables {
            let mut rows = db
                .conn()
                .query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                    [*table],
                )
                .await
                .unwrap();
            let row = rows.next().await.unwrap();
            assert!(row.is_some(), "FTS5 table '{table}' should exist");
        }
    }

    #[tokio::test]
    async fn generate_id_correct_format() {
        let db = test_db().await;
        let id = db.generate_id("fnd").await.unwrap();
        assert!(id.starts_with("fnd-"), "ID should start with 'fnd-': {id}");
        assert_eq!(
            id.len(),
            12,
            "ID should be 12 chars (3 prefix + 1 dash + 8 hex): {id}"
        );

        // Verify hex characters
        let hex_part = &id[4..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "Random part should be hex: {hex_part}"
        );
    }

    #[tokio::test]
    async fn generate_id_all_prefixes() {
        let db = test_db().await;
        for prefix in zen_core::ids::ALL_PREFIXES {
            let id = db.generate_id(prefix).await.unwrap();
            assert!(id.starts_with(&format!("{prefix}-")));
        }
    }

    #[tokio::test]
    async fn generate_id_uniqueness() {
        let db = test_db().await;
        let mut ids = HashSet::new();
        for _ in 0..100 {
            let id = db.generate_id("tst").await.unwrap();
            assert!(ids.insert(id.clone()), "Duplicate ID generated: {id}");
        }
    }

    #[tokio::test]
    async fn idempotent_migrations() {
        let db = test_db().await;
        // Run migrations again — should not fail
        db.run_migrations().await.unwrap();
    }

    #[tokio::test]
    async fn insert_and_select_session() {
        let db = test_db().await;
        let id = db.generate_id("ses").await.unwrap();

        db.conn()
            .execute(
                "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
                [id.as_str()],
            )
            .await
            .unwrap();

        let mut rows = db
            .conn()
            .query(
                "SELECT id, status FROM sessions WHERE id = ?1",
                [id.as_str()],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get::<String>(0).unwrap(), id);
        assert_eq!(row.get::<String>(1).unwrap(), "active");
    }

    #[tokio::test]
    async fn insert_and_select_finding() {
        let db = test_db().await;
        let ses_id = db.generate_id("ses").await.unwrap();
        let fnd_id = db.generate_id("fnd").await.unwrap();

        db.conn()
            .execute(
                "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
                [ses_id.as_str()],
            )
            .await
            .unwrap();

        db.conn()
            .execute(
                "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
                libsql::params![fnd_id.as_str(), ses_id.as_str(), "test finding content", "high"],
            )
            .await
            .unwrap();

        let mut rows = db
            .conn()
            .query(
                "SELECT id, content, confidence FROM findings WHERE id = ?1",
                [fnd_id.as_str()],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get::<String>(0).unwrap(), fnd_id);
        assert_eq!(row.get::<String>(1).unwrap(), "test finding content");
        assert_eq!(row.get::<String>(2).unwrap(), "high");
    }

    #[tokio::test]
    async fn fts5_search_works() {
        let db = test_db().await;
        let ses_id = db.generate_id("ses").await.unwrap();

        db.conn()
            .execute(
                "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
                [ses_id.as_str()],
            )
            .await
            .unwrap();

        let fnd_id = db.generate_id("fnd").await.unwrap();
        db.conn()
            .execute(
                "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
                libsql::params![fnd_id.as_str(), ses_id.as_str(), "tokio async runtime compatibility", "high"],
            )
            .await
            .unwrap();

        // Porter stemming: "runtime" matches "runtime", "spawning" would match "spawn"
        let mut rows = db
            .conn()
            .query(
                "SELECT f.id FROM findings_fts JOIN findings f ON f.rowid = findings_fts.rowid WHERE findings_fts MATCH 'runtime' ORDER BY rank",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap();
        assert!(row.is_some(), "FTS5 should find the finding by 'runtime'");
        assert_eq!(row.unwrap().get::<String>(0).unwrap(), fnd_id);
    }

    #[tokio::test]
    async fn fts5_trigger_populates_on_insert() {
        let db = test_db().await;

        db.conn()
            .execute(
                "INSERT INTO research_items (id, title, description, status) VALUES ('res-test1', 'HTTP Client Research', 'Compare reqwest and hyper', 'open')",
                (),
            )
            .await
            .unwrap();

        let mut rows = db
            .conn()
            .query(
                "SELECT rowid FROM research_fts WHERE research_fts MATCH 'reqwest'",
                (),
            )
            .await
            .unwrap();
        assert!(
            rows.next().await.unwrap().is_some(),
            "FTS trigger should populate on INSERT"
        );
    }

    #[tokio::test]
    async fn entity_links_unique_constraint() {
        let db = test_db().await;

        db.conn()
            .execute(
                "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-test1', 'finding', 'fnd-1', 'hypothesis', 'hyp-1', 'validates')",
                (),
            )
            .await
            .unwrap();

        // Duplicate should fail due to UNIQUE constraint
        let result = db
            .conn()
            .execute(
                "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-test2', 'finding', 'fnd-1', 'hypothesis', 'hyp-1', 'validates')",
                (),
            )
            .await;
        assert!(result.is_err(), "Duplicate entity_link should be rejected");
    }

    #[tokio::test]
    async fn insert_all_table_types() {
        let db = test_db().await;

        // Session (needed as FK for others)
        db.conn()
            .execute("INSERT INTO sessions (id) VALUES ('ses-t1')", ())
            .await
            .unwrap();

        // Project meta
        db.conn()
            .execute(
                "INSERT INTO project_meta (key, value) VALUES ('name', 'test-project')",
                (),
            )
            .await
            .unwrap();

        // Project dependency
        db.conn().execute("INSERT INTO project_dependencies (ecosystem, name, version, source) VALUES ('rust', 'tokio', '1.49', 'cargo.toml')", ()).await.unwrap();

        // Research
        db.conn().execute("INSERT INTO research_items (id, session_id, title) VALUES ('res-t1', 'ses-t1', 'Test research')", ()).await.unwrap();

        // Finding
        db.conn().execute("INSERT INTO findings (id, session_id, content) VALUES ('fnd-t1', 'ses-t1', 'Test finding')", ()).await.unwrap();

        // Finding tag
        db.conn()
            .execute(
                "INSERT INTO finding_tags (finding_id, tag) VALUES ('fnd-t1', 'verified')",
                (),
            )
            .await
            .unwrap();

        // Hypothesis
        db.conn().execute("INSERT INTO hypotheses (id, session_id, content) VALUES ('hyp-t1', 'ses-t1', 'Test hypothesis')", ()).await.unwrap();

        // Insight
        db.conn().execute("INSERT INTO insights (id, session_id, content) VALUES ('ins-t1', 'ses-t1', 'Test insight')", ()).await.unwrap();

        // Issue
        db.conn().execute("INSERT INTO issues (id, session_id, title, type) VALUES ('iss-t1', 'ses-t1', 'Test issue', 'bug')", ()).await.unwrap();

        // Task
        db.conn().execute("INSERT INTO tasks (id, session_id, title) VALUES ('tsk-t1', 'ses-t1', 'Test task')", ()).await.unwrap();

        // Implementation log
        db.conn().execute("INSERT INTO implementation_log (id, task_id, session_id, file_path) VALUES ('imp-t1', 'tsk-t1', 'ses-t1', 'src/main.rs')", ()).await.unwrap();

        // Study
        db.conn().execute("INSERT INTO studies (id, session_id, topic) VALUES ('stu-t1', 'ses-t1', 'Test study')", ()).await.unwrap();

        // Compatibility check
        db.conn().execute("INSERT INTO compatibility_checks (id, session_id, package_a, package_b) VALUES ('cmp-t1', 'ses-t1', 'rust:tokio:1.49', 'rust:axum:0.8')", ()).await.unwrap();

        // Entity link
        db.conn().execute("INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-t1', 'finding', 'fnd-t1', 'hypothesis', 'hyp-t1', 'validates')", ()).await.unwrap();

        // Audit entry
        db.conn().execute("INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES ('aud-t1', 'ses-t1', 'finding', 'fnd-t1', 'created')", ()).await.unwrap();

        // Session snapshot
        db.conn().execute("INSERT INTO session_snapshots (session_id, summary) VALUES ('ses-t1', 'Test snapshot')", ()).await.unwrap();

        // If we got here, all inserts succeeded
    }
}

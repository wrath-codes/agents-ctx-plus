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
pub mod retry;
pub mod service;
pub mod trail;
pub mod updates;

use error::DatabaseError;
use libsql::Builder;
use libsql::params::IntoParams;
use retry::RetryConfig;

/// Central database handle for all Zenith state operations.
///
/// Wraps a libSQL database and connection. Provides ID generation
/// and will host all repository methods in Phase 2.
pub struct ZenDb {
    #[allow(dead_code)]
    db: libsql::Database,
    conn: libsql::Connection,
    is_synced_replica: bool,
    retry: RetryConfig,
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

        let zen_db = Self {
            db,
            conn,
            is_synced_replica: false,
            retry: RetryConfig::default(),
        };
        zen_db.run_migrations().await?;
        Ok(zen_db)
    }

    /// Open a synced embedded replica database backed by Turso Cloud.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the replica cannot be opened, synced, or migrated.
    pub async fn open_synced(
        local_replica_path: &str,
        remote_url: &str,
        auth_token: &str,
    ) -> Result<Self, DatabaseError> {
        let db = Builder::new_remote_replica(
            local_replica_path.to_string(),
            remote_url.to_string(),
            auth_token.to_string(),
        )
        .read_your_writes(true)
        .build()
        .await?;
        db.sync().await?;

        let conn = db.connect()?;
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(|e| DatabaseError::Migration(format!("PRAGMA foreign_keys: {e}")))?;

        let zen_db = Self {
            db,
            conn,
            is_synced_replica: true,
            retry: RetryConfig::default(),
        };
        zen_db.run_migrations().await?;
        Ok(zen_db)
    }

    /// Sync embedded replica state with Turso Cloud.
    ///
    /// For databases opened with [`Self::open_local`], this is a no-op and
    /// returns `Ok(())`. Automatically retries on transient Turso infra errors.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if sync fails after retries.
    pub async fn sync(&self) -> Result<(), DatabaseError> {
        if !self.is_synced_replica {
            return Ok(());
        }
        self.retry_op(|| async { self.db.sync().await.map(|_| ()) })
            .await
    }

    /// Execute SQL with automatic retry on transient Turso errors.
    ///
    /// For synced replicas, retries with exponential backoff when Turso
    /// node recycling errors are detected. Local databases execute
    /// without retry overhead.
    ///
    /// Prefer this over `conn().execute()` for all production code.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if execution fails after retries.
    pub async fn execute<P: IntoParams + Clone>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<u64, DatabaseError> {
        self.retry_op(|| self.conn.execute(sql, params.clone()))
            .await
    }

    /// Execute SQL with a params factory (for non-Clone params like
    /// `params_from_iter`).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if execution fails after retries.
    pub async fn execute_with<P, F>(
        &self,
        sql: &str,
        mut make_params: F,
    ) -> Result<u64, DatabaseError>
    where
        F: FnMut() -> P,
        P: IntoParams,
    {
        self.retry_op(|| self.conn.execute(sql, make_params()))
            .await
    }

    /// Query with automatic retry on transient Turso errors.
    ///
    /// Prefer this over `conn().query()` for all production code.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if query fails after retries.
    pub async fn query<P: IntoParams + Clone>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<libsql::Rows, DatabaseError> {
        self.retry_op(|| self.conn.query(sql, params.clone()))
            .await
    }

    /// Query with a params factory (for non-Clone params like
    /// `params_from_iter`).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if query fails after retries.
    pub async fn query_with<P, F>(
        &self,
        sql: &str,
        mut make_params: F,
    ) -> Result<libsql::Rows, DatabaseError>
    where
        F: FnMut() -> P,
        P: IntoParams,
    {
        self.retry_op(|| self.conn.query(sql, make_params()))
            .await
    }

    /// Access the underlying libSQL connection for direct queries.
    ///
    /// Prefer `execute()` / `query()` for production code — they
    /// automatically retry on transient Turso errors.
    #[must_use]
    pub const fn conn(&self) -> &libsql::Connection {
        &self.conn
    }

    /// Returns whether this handle is backed by a synced remote replica.
    #[must_use]
    pub const fn is_synced_replica(&self) -> bool {
        self.is_synced_replica
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
            .query(
                &format!("SELECT '{prefix}-' || lower(hex(randomblob(4)))"),
                (),
            )
            .await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<String>(0)?)
    }

    /// Internal: retry an async operation with exponential backoff on
    /// transient Turso infrastructure errors. Skipped for local DBs.
    async fn retry_op<T, F, Fut>(&self, mut f: F) -> Result<T, DatabaseError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, libsql::Error>>,
    {
        if !self.is_synced_replica {
            return Ok(f().await?);
        }

        let mut delay = self.retry.base_delay;
        for attempt in 1..=self.retry.max_attempts {
            match f().await {
                Ok(v) => return Ok(v),
                Err(e) if retry::is_transient_turso_error(&e)
                    && attempt < self.retry.max_attempts =>
                {
                    tracing::warn!(
                        attempt,
                        max = self.retry.max_attempts,
                        delay_ms = delay.as_millis() as u64,
                        "Turso transient infra error, retrying: {e}"
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.retry.max_delay);
                }
                Err(e) => return Err(e.into()),
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod test_support;

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

        db.execute(
            "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
            [id.as_str()],
        )
        .await
        .unwrap();

        let mut rows = db
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

        db.execute(
            "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
            [ses_id.as_str()],
        )
        .await
        .unwrap();

        db.execute_with(
            "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
            || libsql::params![fnd_id.as_str(), ses_id.as_str(), "test finding content", "high"],
        )
        .await
        .unwrap();

        let mut rows = db
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

        db.execute(
            "INSERT INTO sessions (id, status) VALUES (?1, 'active')",
            [ses_id.as_str()],
        )
        .await
        .unwrap();

        let fnd_id = db.generate_id("fnd").await.unwrap();
        db.execute_with(
            "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
            || libsql::params![fnd_id.as_str(), ses_id.as_str(), "tokio async runtime compatibility", "high"],
        )
        .await
        .unwrap();

        // Porter stemming: "runtime" matches "runtime", "spawning" would match "spawn"
        let mut rows = db
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

        db.execute(
            "INSERT INTO research_items (id, title, description, status) VALUES ('res-test1', 'HTTP Client Research', 'Compare reqwest and hyper', 'open')",
            (),
        )
        .await
        .unwrap();

        let mut rows = db
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

        db.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-test1', 'finding', 'fnd-1', 'hypothesis', 'hyp-1', 'validates')",
            (),
        )
        .await
        .unwrap();

        // Duplicate should fail due to UNIQUE constraint
        let result = db
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
        db.execute("INSERT INTO sessions (id) VALUES ('ses-t1')", ())
            .await
            .unwrap();

        // Project meta
        db.execute(
            "INSERT INTO project_meta (key, value) VALUES ('name', 'test-project')",
            (),
        )
        .await
        .unwrap();

        // Project dependency
        db.execute("INSERT INTO project_dependencies (ecosystem, name, version, source) VALUES ('rust', 'tokio', '1.49', 'cargo.toml')", ()).await.unwrap();

        // Research
        db.execute("INSERT INTO research_items (id, session_id, title) VALUES ('res-t1', 'ses-t1', 'Test research')", ()).await.unwrap();

        // Finding
        db.execute("INSERT INTO findings (id, session_id, content) VALUES ('fnd-t1', 'ses-t1', 'Test finding')", ()).await.unwrap();

        // Finding tag
        db.execute(
            "INSERT INTO finding_tags (finding_id, tag) VALUES ('fnd-t1', 'verified')",
            (),
        )
        .await
        .unwrap();

        // Hypothesis
        db.execute("INSERT INTO hypotheses (id, session_id, content) VALUES ('hyp-t1', 'ses-t1', 'Test hypothesis')", ()).await.unwrap();

        // Insight
        db.execute("INSERT INTO insights (id, session_id, content) VALUES ('ins-t1', 'ses-t1', 'Test insight')", ()).await.unwrap();

        // Issue
        db.execute("INSERT INTO issues (id, session_id, title, type) VALUES ('iss-t1', 'ses-t1', 'Test issue', 'bug')", ()).await.unwrap();

        // Task
        db.execute("INSERT INTO tasks (id, session_id, title) VALUES ('tsk-t1', 'ses-t1', 'Test task')", ()).await.unwrap();

        // Implementation log
        db.execute("INSERT INTO implementation_log (id, task_id, session_id, file_path) VALUES ('imp-t1', 'tsk-t1', 'ses-t1', 'src/main.rs')", ()).await.unwrap();

        // Study
        db.execute("INSERT INTO studies (id, session_id, topic) VALUES ('stu-t1', 'ses-t1', 'Test study')", ()).await.unwrap();

        // Compatibility check
        db.execute("INSERT INTO compatibility_checks (id, session_id, package_a, package_b) VALUES ('cmp-t1', 'ses-t1', 'rust:tokio:1.49', 'rust:axum:0.8')", ()).await.unwrap();

        // Entity link
        db.execute("INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-t1', 'finding', 'fnd-t1', 'hypothesis', 'hyp-t1', 'validates')", ()).await.unwrap();

        // Audit entry
        db.execute("INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES ('aud-t1', 'ses-t1', 'finding', 'fnd-t1', 'created')", ()).await.unwrap();

        // Session snapshot
        db.execute("INSERT INTO session_snapshots (session_id, summary) VALUES ('ses-t1', 'Test snapshot')", ()).await.unwrap();

        // If we got here, all inserts succeeded
    }
}

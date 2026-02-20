//! Database migration runner.
//!
//! Embeds the SQL migration files at compile time and executes them on
//! database open. All statements use `IF NOT EXISTS` for idempotent re-running.

use crate::ZenDb;
use crate::error::DatabaseError;

/// Initial schema: 14 tables, 8 FTS5 virtual tables, 31 indexes, 22 triggers.
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_catalog.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_team.sql");

impl ZenDb {
    /// Run all embedded migrations in sequence.
    pub(crate) async fn run_migrations(&self) -> Result<(), DatabaseError> {
        self.conn
            .execute_batch(MIGRATION_001)
            .await
            .map_err(|e| DatabaseError::Migration(format!("001_initial: {e}")))?;
        self.conn
            .execute_batch(MIGRATION_002)
            .await
            .map_err(|e| DatabaseError::Migration(format!("002_catalog: {e}")))?;

        // 003_team: ALTER TABLE ADD COLUMN statements may fail if columns already
        // exist (re-run on existing DB). Execute each statement individually and
        // ignore "duplicate column name" errors.
        for raw_stmt in MIGRATION_003.split(';') {
            let stmt = raw_stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            // Check if the fragment has any non-comment content.
            let non_comment: String = stmt
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join(" ");
            let non_comment = non_comment.trim();
            if non_comment.is_empty() {
                continue;
            }
            match self.conn.execute(stmt, ()).await {
                Ok(_) => {}
                Err(e) if e.to_string().contains("duplicate column name") => {}
                Err(e) if e.to_string().contains("already exists") => {}
                Err(e) => return Err(DatabaseError::Migration(format!("003_team: {e}"))),
            }
        }

        Ok(())
    }
}

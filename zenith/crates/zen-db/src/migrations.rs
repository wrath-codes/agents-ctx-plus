//! Database migration runner.
//!
//! Embeds the SQL migration files at compile time and executes them on
//! database open. All statements use `IF NOT EXISTS` for idempotent re-running.

use crate::ZenDb;
use crate::error::DatabaseError;

/// Initial schema: 14 tables, 8 FTS5 virtual tables, 31 indexes, 22 triggers.
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_catalog.sql");

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
        Ok(())
    }
}

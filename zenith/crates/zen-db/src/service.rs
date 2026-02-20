//! Service layer orchestrating database mutations with audit and trail.
//!
//! `ZenService` wraps `ZenDb` (raw database access), `TrailWriter` (JSONL
//! persistence), and `SchemaRegistry` (schema validation). All repo methods
//! are implemented as `impl ZenService`.

use std::path::PathBuf;

use zen_core::identity::AuthIdentity;
use zen_schema::SchemaRegistry;

use crate::ZenDb;
use crate::error::DatabaseError;
use crate::trail::writer::TrailWriter;

/// Orchestrates database mutations with audit trail and JSONL trail.
///
/// Every mutation method follows this protocol:
/// 1. Begin transaction
/// 2. Execute SQL
/// 3. Append audit entry (inside transaction)
/// 4. Append JSONL trail operation (file I/O)
/// 5. Commit transaction
pub struct ZenService {
    db: ZenDb,
    trail: TrailWriter,
    schema: SchemaRegistry,
    identity: Option<AuthIdentity>,
}

impl ZenService {
    /// Create a new service wrapping a local database.
    ///
    /// # Arguments
    ///
    /// * `db_path` — Path to the libSQL database file, or `":memory:"` for tests.
    /// * `trail_dir` — Directory for JSONL trail files. Pass `None` to disable
    ///   trail writing (for tests that don't need trail files).
    /// * `identity` — Authenticated user identity from Clerk JWT. Pass `None`
    ///   for unauthenticated / local-only mode.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the database cannot be opened or the trail
    /// directory cannot be created.
    pub async fn new_local(
        db_path: &str,
        trail_dir: Option<PathBuf>,
        identity: Option<AuthIdentity>,
    ) -> Result<Self, DatabaseError> {
        let db = ZenDb::open_local(db_path).await?;
        let trail = match trail_dir {
            Some(dir) => TrailWriter::new(dir)?,
            None => TrailWriter::disabled(),
        };
        let schema = SchemaRegistry::new();
        Ok(Self { db, trail, schema, identity })
    }

    /// Create a service backed by a synced Turso embedded replica.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the replica cannot be opened or trail cannot be created.
    pub async fn new_synced(
        local_replica_path: &str,
        remote_url: &str,
        auth_token: &str,
        trail_dir: Option<PathBuf>,
        identity: Option<AuthIdentity>,
    ) -> Result<Self, DatabaseError> {
        let db = ZenDb::open_synced(local_replica_path, remote_url, auth_token).await?;
        let trail = match trail_dir {
            Some(dir) => TrailWriter::new(dir)?,
            None => TrailWriter::disabled(),
        };
        let schema = SchemaRegistry::new();
        Ok(Self { db, trail, schema, identity })
    }

    /// Create from an existing `ZenDb` (for testing).
    #[must_use]
    pub fn from_db(db: ZenDb, trail: TrailWriter, identity: Option<AuthIdentity>) -> Self {
        Self {
            db,
            trail,
            schema: SchemaRegistry::new(),
            identity,
        }
    }

    /// Access the underlying database handle.
    #[must_use]
    pub const fn db(&self) -> &ZenDb {
        &self.db
    }

    /// Access the trail writer mutably (e.g., to disable during rebuild).
    pub const fn trail_mut(&mut self) -> &mut TrailWriter {
        &mut self.trail
    }

    /// Access the trail writer.
    #[must_use]
    pub const fn trail(&self) -> &TrailWriter {
        &self.trail
    }

    /// Access the schema registry.
    #[must_use]
    pub const fn schema(&self) -> &SchemaRegistry {
        &self.schema
    }

    /// Authenticated user identity, if available.
    #[must_use]
    pub const fn identity(&self) -> Option<&AuthIdentity> {
        self.identity.as_ref()
    }

    /// The org_id from the authenticated identity, for entity creation.
    /// Returns `None` when unauthenticated or when no org is active.
    #[must_use]
    pub fn org_id(&self) -> Option<&str> {
        self.identity.as_ref().and_then(|i| i.org_id.as_deref())
    }

    /// The user_id from the authenticated identity.
    /// Returns `None` when unauthenticated.
    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        self.identity.as_ref().map(|i| i.user_id.as_str())
    }

    /// Build an org_id filter clause and its parameter.
    ///
    /// Returns `(sql_fragment, params)` where:
    /// - Authenticated with org: `("AND (org_id = ?N OR org_id IS NULL)", vec![org_id])`
    /// - No org / unauthenticated: `("AND org_id IS NULL", vec![])`
    pub(crate) fn org_id_filter(&self, param_index: u32) -> (String, Vec<libsql::Value>) {
        match self.org_id() {
            Some(org_id) => (
                format!("AND (org_id = ?{param_index} OR org_id IS NULL)"),
                vec![org_id.into()],
            ),
            None => ("AND org_id IS NULL".to_string(), vec![]),
        }
    }

    /// Sync the underlying database with remote cloud state.
    pub async fn sync(&self) -> Result<(), DatabaseError> {
        self.db.sync().await
    }

    /// Returns whether this service is backed by a synced Turso replica.
    #[must_use]
    pub const fn is_synced_replica(&self) -> bool {
        self.db.is_synced_replica()
    }
}

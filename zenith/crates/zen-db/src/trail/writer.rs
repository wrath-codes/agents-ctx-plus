//! JSONL trail writer.
//!
//! Appends `TrailOperation` records to per-session `.zenith/trail/{session_id}.jsonl`
//! files. Uses `serde_jsonlines::append_json_lines` for atomic per-line appends.

use std::path::{Path, PathBuf};

use zen_core::enums::EntityType;
use zen_core::trail::TrailOperation;
use zen_schema::SchemaRegistry;

use crate::error::DatabaseError;

/// Appends trail operations to per-session JSONL files.
///
/// Every mutation in `ZenService` calls `append()` before committing the DB
/// transaction. The trail is the source of truth — the DB is rebuildable from it.
pub struct TrailWriter {
    trail_dir: PathBuf,
    enabled: bool,
}

impl TrailWriter {
    /// Create a new `TrailWriter` pointing at the given directory.
    ///
    /// Creates the directory if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the directory cannot be created.
    pub fn new(trail_dir: PathBuf) -> Result<Self, DatabaseError> {
        std::fs::create_dir_all(&trail_dir).map_err(|e| DatabaseError::Other(e.into()))?;
        Ok(Self {
            trail_dir,
            enabled: true,
        })
    }

    /// Create a disabled writer (for testing or when trail is not needed).
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            trail_dir: PathBuf::new(),
            enabled: false,
        }
    }

    /// Set whether writing is enabled.
    ///
    /// Disabled during rebuild to avoid re-writing replayed operations.
    pub const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Whether trail writing is currently enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Append a trail operation to the session's JSONL file.
    ///
    /// File path: `{trail_dir}/{op.ses}.jsonl`
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the file write fails.
    pub fn append(&self, op: &TrailOperation) -> Result<(), DatabaseError> {
        if !self.enabled {
            return Ok(());
        }

        let path = self.trail_dir.join(format!("{}.jsonl", op.ses));
        serde_jsonlines::append_json_lines(&path, [op])
            .map_err(|e| DatabaseError::Other(e.into()))?;
        Ok(())
    }

    /// Append with schema validation of the `data` field.
    ///
    /// Only validates `Create` ops (full entity data). `Update` ops contain
    /// partial data and would fail required-field checks against the full
    /// entity schema. Validation is warn-only — permissive for forward-compat.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the file write fails. Schema validation
    /// failures are logged as warnings but do not prevent writing.
    pub fn append_validated(
        &self,
        op: &TrailOperation,
        schema: &SchemaRegistry,
    ) -> Result<(), DatabaseError> {
        if !self.enabled {
            return Ok(());
        }

        if op.op == zen_core::enums::TrailOp::Create {
            let schema_name = entity_type_to_schema_name(op.entity);
            if let Err(e) = schema.validate(schema_name, &op.data) {
                tracing::warn!(
                    "Trail validation failed for {} {}: {:?}",
                    op.entity,
                    op.id,
                    e
                );
            }
        }

        self.append(op)
    }

    /// The directory where trail files are stored.
    #[must_use]
    pub fn trail_dir(&self) -> &Path {
        &self.trail_dir
    }
}

/// Map `EntityType` to schema registry name.
const fn entity_type_to_schema_name(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Session => "session",
        EntityType::Research => "research_item",
        EntityType::Finding => "finding",
        EntityType::Hypothesis => "hypothesis",
        EntityType::Insight => "insight",
        EntityType::Issue => "issue",
        EntityType::Task => "task",
        EntityType::ImplLog => "impl_log",
        EntityType::Compat => "compat_check",
        EntityType::Study => "study",
        EntityType::Decision => "decision",
        EntityType::EntityLink => "entity_link",
        EntityType::Audit => "audit_entry",
    }
}

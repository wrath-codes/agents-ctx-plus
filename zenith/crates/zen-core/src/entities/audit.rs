use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{AuditAction, EntityType};

/// An append-only audit trail entry recording a mutation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditEntry {
    pub id: String,
    pub session_id: Option<String>,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub action: AuditAction,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

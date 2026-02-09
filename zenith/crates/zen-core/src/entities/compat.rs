use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::CompatStatus;

/// Tracks compatibility between two packages.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CompatCheck {
    pub id: String,
    /// Format: `ecosystem:name:version` (e.g., `rust:tokio:1.40.0`).
    pub package_a: String,
    /// Format: `ecosystem:name:version` (e.g., `rust:axum:0.8.0`).
    pub package_b: String,
    pub status: CompatStatus,
    pub conditions: Option<String>,
    pub finding_id: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::HypothesisStatus;

/// Something believed to be true that needs validation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Hypothesis {
    pub id: String,
    pub research_id: Option<String>,
    pub finding_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub status: HypothesisStatus,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

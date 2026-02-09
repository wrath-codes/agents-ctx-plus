use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::Confidence;

/// A discovered fact. Can be standalone or linked to a research item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub research_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

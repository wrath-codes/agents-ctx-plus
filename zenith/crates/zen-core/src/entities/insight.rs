use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::Confidence;

/// A conclusion drawn from multiple findings. Higher-level synthesis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Insight {
    pub id: String,
    pub research_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::ResearchStatus;

/// A research item — a question or investigation topic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResearchItem {
    pub id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: ResearchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

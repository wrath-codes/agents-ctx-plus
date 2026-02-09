use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{StudyMethodology, StudyStatus};

/// A structured learning process investigating a topic with hypotheses and findings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Study {
    pub id: String,
    pub session_id: Option<String>,
    pub research_id: Option<String>,
    pub topic: String,
    pub library: Option<String>,
    pub methodology: StudyMethodology,
    pub status: StudyStatus,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

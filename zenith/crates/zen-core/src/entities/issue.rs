use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{IssueStatus, IssueType};

/// An issue tracking bugs, features, spikes, epics, or requests.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Issue {
    pub id: String,
    pub issue_type: IssueType,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    /// Priority: 1 (highest) to 5 (lowest).
    pub priority: u8,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

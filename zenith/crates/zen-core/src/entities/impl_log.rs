use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Records where code was implemented, linked to the task that required it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ImplLog {
    pub id: String,
    pub task_id: String,
    pub session_id: Option<String>,
    pub file_path: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

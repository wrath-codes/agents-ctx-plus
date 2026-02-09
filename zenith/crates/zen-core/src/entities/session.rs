use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::SessionStatus;

/// A work session. The LLM starts a session at the beginning of a conversation
/// and wraps it up at the end.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub summary: Option<String>,
}

/// Point-in-time snapshot of project state generated at `znt wrap-up`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub open_tasks: i64,
    pub in_progress_tasks: i64,
    pub pending_hypotheses: i64,
    pub unverified_hypotheses: i64,
    pub recent_findings: i64,
    pub open_research: i64,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

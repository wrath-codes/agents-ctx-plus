use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceBackend {
    Agentfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInfo {
    pub backend: WorkspaceBackend,
    pub workspace_id: String,
    pub root: String,
    pub persistent: bool,
    pub created: bool,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceSnapshot {
    pub status: String,
    pub workspace_id: String,
    pub files_total: u64,
    pub bytes_total: u64,
    pub tool_calls_total: u64,
    pub tool_calls_success: u64,
    pub tool_calls_failed: u64,
    pub captured_at: DateTime<Utc>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceAuditEntry {
    pub id: String,
    pub session_id: String,
    pub workspace_id: String,
    pub source: String,
    pub event: String,
    pub path: Option<String>,
    pub tool: String,
    pub status: String,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceChannelStatus {
    pub status: String,
    pub error: Option<String>,
}

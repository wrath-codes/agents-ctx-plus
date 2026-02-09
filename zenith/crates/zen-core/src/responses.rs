//! CLI response types returned as JSON by `znt` commands.
//!
//! These structs define the shape of JSON output for commands like
//! `znt finding create`, `znt session start`, `znt whats-next`, `znt search`,
//! and `znt rebuild`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entities::{AuditEntry, Finding, Hypothesis, Session, Task};

/// Response from `znt finding create`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FindingCreateResponse {
    pub finding: Finding,
}

/// Response from `znt session start`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionStartResponse {
    pub session: Session,
    pub previous_session: Option<Session>,
}

/// Response from `znt whats-next`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WhatsNextResponse {
    pub last_session: Option<Session>,
    pub open_tasks: Vec<Task>,
    pub pending_hypotheses: Vec<Hypothesis>,
    pub recent_audit: Vec<AuditEntry>,
}

/// A single search result from the documentation lake.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SearchResult {
    pub package: String,
    pub ecosystem: String,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub file_path: Option<String>,
    pub line_start: Option<u32>,
    pub score: f64,
}

/// Response from `znt search`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SearchResultsResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: u32,
}

/// Response from `znt rebuild`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RebuildResponse {
    pub rebuilt: bool,
    pub trail_files: u32,
    pub operations_replayed: u32,
    pub entities_created: u32,
    pub duration_ms: u64,
}

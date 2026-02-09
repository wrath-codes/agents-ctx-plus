//! Typed audit detail payloads.
//!
//! Each audit action can carry a structured `detail` JSON blob. These types
//! provide schema validation for the most common detail shapes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Detail for `AuditAction::StatusChanged`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StatusChangedDetail {
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
}

/// Detail for `AuditAction::Linked` and `AuditAction::Unlinked`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LinkedDetail {
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
}

/// Detail for `AuditAction::Tagged` and `AuditAction::Untagged`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TaggedDetail {
    pub tag: String,
}

/// Detail for `AuditAction::Indexed`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct IndexedDetail {
    pub package: String,
    pub ecosystem: String,
    pub symbols: u32,
    pub duration_ms: u64,
}

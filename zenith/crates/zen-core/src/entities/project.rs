use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A key-value pair of project-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProjectMeta {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// A detected project dependency from a manifest file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProjectDependency {
    /// Ecosystem: `rust`, `npm`, `hex`, `pypi`, `go`.
    pub ecosystem: String,
    pub name: String,
    pub version: Option<String>,
    /// Which manifest file: `cargo.toml`, `package.json`, etc.
    pub source: String,
    pub indexed: bool,
    pub indexed_at: Option<DateTime<Utc>>,
}

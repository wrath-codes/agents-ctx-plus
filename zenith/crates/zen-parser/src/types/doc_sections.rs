use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed documentation sections from doc comments/docstrings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocSections {
    pub errors: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
    pub examples: Option<String>,
    pub args: HashMap<String, String>,
    pub returns: Option<String>,
    pub raises: HashMap<String, String>,
    pub yields: Option<String>,
    pub notes: Option<String>,
}

//! Finding update builder.

use serde::Serialize;
use zen_core::enums::Confidence;

#[derive(Debug, Clone, Default, Serialize)]
pub struct FindingUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

pub struct FindingUpdateBuilder(FindingUpdate);

impl FindingUpdateBuilder {
    pub fn new() -> Self {
        Self(FindingUpdate::default())
    }

    pub fn content(mut self, val: impl Into<String>) -> Self {
        self.0.content = Some(val.into());
        self
    }

    pub fn source(mut self, val: Option<String>) -> Self {
        self.0.source = Some(val);
        self
    }

    pub fn confidence(mut self, val: Confidence) -> Self {
        self.0.confidence = Some(val);
        self
    }

    pub fn build(self) -> FindingUpdate {
        self.0
    }
}

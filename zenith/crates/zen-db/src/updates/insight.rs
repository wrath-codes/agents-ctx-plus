//! Insight update builder.

use serde::Serialize;
use zen_core::enums::Confidence;

#[derive(Debug, Clone, Default, Serialize)]
pub struct InsightUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

pub struct InsightUpdateBuilder(InsightUpdate);

impl InsightUpdateBuilder {
    pub fn new() -> Self {
        Self(InsightUpdate::default())
    }

    pub fn content(mut self, val: impl Into<String>) -> Self {
        self.0.content = Some(val.into());
        self
    }

    pub fn confidence(mut self, val: Confidence) -> Self {
        self.0.confidence = Some(val);
        self
    }

    pub fn build(self) -> InsightUpdate {
        self.0
    }
}

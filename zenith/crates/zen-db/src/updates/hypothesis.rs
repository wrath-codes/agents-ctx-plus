//! Hypothesis update builder.

use serde::Serialize;
use zen_core::enums::HypothesisStatus;

#[derive(Debug, Clone, Default, Serialize)]
pub struct HypothesisUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<HypothesisStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Option<String>>,
}

pub struct HypothesisUpdateBuilder(HypothesisUpdate);

impl HypothesisUpdateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(HypothesisUpdate::default())
    }

    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.0.content = Some(content.into());
        self
    }

    #[must_use]
    pub fn status(mut self, status: HypothesisStatus) -> Self {
        self.0.status = Some(status);
        self
    }

    #[must_use]
    pub fn reason(mut self, reason: Option<String>) -> Self {
        self.0.reason = Some(reason);
        self
    }

    #[must_use]
    pub fn build(self) -> HypothesisUpdate {
        self.0
    }
}

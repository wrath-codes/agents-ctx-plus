//! Research item update builder.

use serde::Serialize;
use zen_core::enums::ResearchStatus;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ResearchUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ResearchStatus>,
}

pub struct ResearchUpdateBuilder(ResearchUpdate);

impl ResearchUpdateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(ResearchUpdate::default())
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.0.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn description(mut self, description: Option<String>) -> Self {
        self.0.description = Some(description);
        self
    }

    #[must_use]
    pub fn status(mut self, status: ResearchStatus) -> Self {
        self.0.status = Some(status);
        self
    }

    #[must_use]
    pub fn build(self) -> ResearchUpdate {
        self.0
    }
}

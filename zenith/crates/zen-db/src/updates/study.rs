//! Study update builder.

use serde::Serialize;
use zen_core::enums::{StudyMethodology, StudyStatus};

#[derive(Debug, Clone, Default, Serialize)]
pub struct StudyUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodology: Option<StudyMethodology>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StudyStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Option<String>>,
}

pub struct StudyUpdateBuilder(StudyUpdate);

impl StudyUpdateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(StudyUpdate::default())
    }

    #[must_use]
    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.0.topic = Some(topic.into());
        self
    }

    #[must_use]
    pub fn library(mut self, library: Option<String>) -> Self {
        self.0.library = Some(library);
        self
    }

    #[must_use]
    pub fn methodology(mut self, methodology: StudyMethodology) -> Self {
        self.0.methodology = Some(methodology);
        self
    }

    #[must_use]
    pub fn status(mut self, status: StudyStatus) -> Self {
        self.0.status = Some(status);
        self
    }

    #[must_use]
    pub fn summary(mut self, summary: Option<String>) -> Self {
        self.0.summary = Some(summary);
        self
    }

    #[must_use]
    pub fn build(self) -> StudyUpdate {
        self.0
    }
}

impl Default for StudyUpdateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

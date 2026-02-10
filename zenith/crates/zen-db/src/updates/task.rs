//! Task update builder.

use serde::Serialize;
use zen_core::enums::TaskStatus;

#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research_id: Option<Option<String>>,
}

pub struct TaskUpdateBuilder(TaskUpdate);

impl TaskUpdateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(TaskUpdate::default())
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
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.0.status = Some(status);
        self
    }

    #[must_use]
    pub fn issue_id(mut self, issue_id: Option<String>) -> Self {
        self.0.issue_id = Some(issue_id);
        self
    }

    #[must_use]
    pub fn research_id(mut self, research_id: Option<String>) -> Self {
        self.0.research_id = Some(research_id);
        self
    }

    #[must_use]
    pub fn build(self) -> TaskUpdate {
        self.0
    }
}

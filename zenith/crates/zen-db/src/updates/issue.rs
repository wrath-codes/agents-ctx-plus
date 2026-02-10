//! Issue update builder.

use serde::Serialize;
use zen_core::enums::{IssueStatus, IssueType};

#[derive(Debug, Clone, Default, Serialize)]
pub struct IssueUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<IssueStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<String>>,
}

pub struct IssueUpdateBuilder(IssueUpdate);

impl IssueUpdateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(IssueUpdate::default())
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
    pub fn status(mut self, status: IssueStatus) -> Self {
        self.0.status = Some(status);
        self
    }

    #[must_use]
    pub fn priority(mut self, priority: u8) -> Self {
        self.0.priority = Some(priority);
        self
    }

    #[must_use]
    pub fn issue_type(mut self, issue_type: IssueType) -> Self {
        self.0.issue_type = Some(issue_type);
        self
    }

    #[must_use]
    pub fn parent_id(mut self, parent_id: Option<String>) -> Self {
        self.0.parent_id = Some(parent_id);
        self
    }

    #[must_use]
    pub fn build(self) -> IssueUpdate {
        self.0
    }
}

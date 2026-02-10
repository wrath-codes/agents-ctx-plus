//! Compatibility check update builder.

use serde::Serialize;
use zen_core::enums::CompatStatus;

#[derive(Debug, Clone, Default, Serialize)]
pub struct CompatUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CompatStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding_id: Option<Option<String>>,
}

pub struct CompatUpdateBuilder(CompatUpdate);

impl CompatUpdateBuilder {
    pub fn new() -> Self {
        Self(CompatUpdate::default())
    }

    pub fn status(mut self, val: CompatStatus) -> Self {
        self.0.status = Some(val);
        self
    }

    pub fn conditions(mut self, val: Option<String>) -> Self {
        self.0.conditions = Some(val);
        self
    }

    pub fn finding_id(mut self, val: Option<String>) -> Self {
        self.0.finding_id = Some(val);
        self
    }

    pub fn build(self) -> CompatUpdate {
        self.0
    }
}

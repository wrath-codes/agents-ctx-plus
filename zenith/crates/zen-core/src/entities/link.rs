use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{EntityType, Relation};

/// A many-to-many relationship between any two entities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EntityLink {
    pub id: String,
    pub source_type: EntityType,
    pub source_id: String,
    pub target_type: EntityType,
    pub target_id: String,
    pub relation: Relation,
    pub created_at: DateTime<Utc>,
}

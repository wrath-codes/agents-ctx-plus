//! JSONL trail operation envelope.
//!
//! Every mutation in Zenith is recorded as a `TrailOperation` in per-session
//! `.zenith/trail/{session_id}.jsonl` files. The database is rebuildable from
//! these trail files (see spike 0.12).
//!
//! The `v` field supports schema versioning (spike 0.16): old trail files
//! without a `v` field deserialize with `v == 1` via `#[serde(default)]`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{EntityType, TrailOp};

/// Default trail version for backward compatibility with old JSONL files.
const fn default_trail_version() -> u32 {
    1
}

/// A single operation recorded in the JSONL trail.
///
/// This is the source-of-truth envelope. The `data` field contains the full
/// entity state for `Create` ops, changed fields for `Update`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TrailOperation {
    /// Schema version. Defaults to 1 for old trails without this field.
    #[serde(default = "default_trail_version")]
    pub v: u32,

    /// ISO 8601 timestamp of the operation.
    pub ts: String,

    /// Session ID that produced this operation.
    pub ses: String,

    /// What kind of mutation this represents.
    pub op: TrailOp,

    /// Which entity type was affected.
    pub entity: EntityType,

    /// ID of the affected entity.
    pub id: String,

    /// Operation payload. Schema depends on `op` and `entity`.
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::{EntityType, TrailOp};

    #[test]
    fn trail_op_roundtrip() {
        let op = TrailOperation {
            v: 1,
            ts: "2026-02-08T12:00:00Z".to_string(),
            ses: "ses-a3f8b2c1".to_string(),
            op: TrailOp::Create,
            entity: EntityType::Finding,
            id: "fnd-deadbeef".to_string(),
            data: serde_json::json!({"content": "test finding"}),
        };

        let json = serde_json::to_string(&op).unwrap();
        let recovered: TrailOperation = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, op);
    }

    #[test]
    fn trail_op_default_version() {
        // Old trail format without `v` field — should deserialize with v=1
        let json = r#"{"ts":"2026-01-01T00:00:00Z","ses":"ses-00000000","op":"create","entity":"finding","id":"fnd-11111111","data":{}}"#;
        let op: TrailOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.v, 1);
    }

    #[test]
    fn trail_op_explicit_version() {
        let json = r#"{"v":2,"ts":"2026-02-08T12:00:00Z","ses":"ses-00000000","op":"transition","entity":"hypothesis","id":"hyp-11111111","data":{"from":"unverified","to":"analyzing"}}"#;
        let op: TrailOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.v, 2);
        assert_eq!(op.op, TrailOp::Transition);
    }

    #[test]
    fn trail_op_all_ops_serialize() {
        for op in [
            TrailOp::Create,
            TrailOp::Update,
            TrailOp::Delete,
            TrailOp::Link,
            TrailOp::Unlink,
            TrailOp::Tag,
            TrailOp::Untag,
            TrailOp::Transition,
        ] {
            let trail = TrailOperation {
                v: 1,
                ts: String::new(),
                ses: String::new(),
                op,
                entity: EntityType::Session,
                id: String::new(),
                data: serde_json::Value::Null,
            };
            let json = serde_json::to_string(&trail).unwrap();
            let recovered: TrailOperation = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered.op, op);
        }
    }
}

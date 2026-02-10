//! Update builder types for entity mutations.
//!
//! Each builder produces an update struct with `Option` fields. Only `Some` fields
//! generate SET clauses in the dynamic UPDATE SQL. The builder output is serialized
//! as the trail `data` payload (changed fields only).

pub mod compat;
pub mod finding;
pub mod hypothesis;
pub mod insight;
pub mod issue;
pub mod research;
pub mod study;
pub mod task;

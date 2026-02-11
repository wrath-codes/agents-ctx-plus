//! Repository modules implementing CRUD operations for all Zenith entities.
//!
//! Each module adds methods to `ZenService` via `impl ZenService` blocks.

pub mod audit;
pub mod compat;
pub mod finding;
pub mod hypothesis;
pub mod impl_log;
pub mod insight;
pub mod issue;
pub mod link;
pub mod project;
pub mod research;
pub mod session;
pub mod study;
pub mod task;
pub mod whats_next;

//! Entity structs for all Zenith domain objects.
//!
//! Each entity maps to a table in the libSQL database (see `01-turso-data-model.md`).
//! All structs derive `Serialize`, `Deserialize`, and `JsonSchema` for JSON roundtrip
//! and schema validation.

mod audit;
mod compat;
mod finding;
mod hypothesis;
mod impl_log;
mod insight;
mod issue;
mod link;
mod project;
mod research;
mod session;
mod study;
mod task;

pub use audit::AuditEntry;
pub use compat::CompatCheck;
pub use finding::Finding;
pub use hypothesis::Hypothesis;
pub use impl_log::ImplLog;
pub use insight::Insight;
pub use issue::Issue;
pub use link::EntityLink;
pub use project::{ProjectDependency, ProjectMeta};
pub use research::ResearchItem;
pub use session::{Session, SessionSnapshot};
pub use study::Study;
pub use task::Task;

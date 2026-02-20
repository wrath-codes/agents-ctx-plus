use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Lightweight authenticated user identity for cross-crate passing.
///
/// Produced by `zen-auth`, consumed by `zen-cli` and (in Phase 9) `zen-db`.
/// Contains only data fields — no auth logic, no Clerk SDK calls.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthIdentity {
    /// Clerk user ID (from JWT `sub` claim).
    pub user_id: String,
    /// Clerk organization ID (from JWT `org_id` claim). `None` = personal mode.
    pub org_id: Option<String>,
    /// Clerk organization slug (from JWT `org_slug` claim).
    pub org_slug: Option<String>,
    /// Clerk organization role (from JWT `org_role` claim, e.g. `"org:admin"`).
    pub org_role: Option<String>,
}

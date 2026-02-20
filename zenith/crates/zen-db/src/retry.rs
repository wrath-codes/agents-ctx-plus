//! Turso transient error retry logic.
//!
//! Provides automatic retry with exponential backoff for transient
//! Turso cloud infrastructure errors (node recycling, shared lock
//! contention during provisioning/deletion). These errors surface as
//! HTTP 400 responses from the Hrana API and resolve on their own
//! within seconds.
//!
//! Local-only databases never encounter these errors — the retry
//! path is gated on `ZenDb::is_synced_replica`.

use std::time::Duration;

/// Configuration for retry behavior on transient Turso errors.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the initial one).
    pub max_attempts: u32,
    /// Initial delay before the first retry.
    pub base_delay: Duration,
    /// Maximum delay between retries (backoff is capped here).
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
        }
    }
}

/// Detect transient Turso infrastructure errors.
///
/// These are 400-level Hrana errors that occur when Turso cloud nodes
/// are being created, deleted, or recycled. They are not application
/// bugs and resolve on their own within seconds.
///
/// The predicate is intentionally narrow to avoid retrying genuine
/// SQL or constraint errors.
pub fn is_transient_turso_error(e: &libsql::Error) -> bool {
    let msg = e.to_string();
    msg.contains("unable to acquire shared lock")
        || msg.contains("deletion must be in progress")
}

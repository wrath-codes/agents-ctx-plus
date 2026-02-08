//! # zen-hooks
//!
//! Git hooks management and session-git integration for Zenith.
//!
//! Uses `gix` (pure Rust git implementation) for:
//! - Repository discovery and config reading
//! - Hook installation and management
//! - Session-git integration (branch/HEAD/tags)
//!
//! This crate isolates the `gix` dependency from the rest of the workspace,
//! so compile time impact is limited to this crate only.

#[cfg(test)]
mod spike_git_hooks;

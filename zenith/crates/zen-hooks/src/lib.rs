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

#![allow(clippy::case_sensitive_file_extension_comparisons)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_return)]
#![allow(clippy::struct_excessive_bools)]

pub mod checkout;
pub mod error;
pub mod installer;
pub mod merge;
pub mod repo;
pub mod scripts;
pub mod session_tags;
pub mod validator;

pub use checkout::{PostCheckoutAction, analyze_post_checkout};
pub use error::HookError;
pub use installer::{
    HookInstallMode, HookInstallStrategy, HookInstallationReport, HookStatus, HookStatusReport,
    install_hooks, status_hooks, uninstall_hooks,
};
pub use merge::{PostMergeAction, analyze_post_merge};
pub use validator::{TrailValidationError, TrailValidationReport, validate_staged_trail_files};

#[cfg(test)]
#[allow(warnings)]
mod spike_git_hooks;

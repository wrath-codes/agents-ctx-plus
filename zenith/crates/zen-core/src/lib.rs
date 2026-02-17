//! # zen-core
//!
//! Core types, ID generation, and error types for Zenith.
//!
//! This crate provides the foundational types shared across all Zenith crates:
//! - Entity structs for all domain objects (findings, hypotheses, tasks, etc.)
//! - Status enums with state machine transitions
//! - ID prefix constants and formatting helpers
//! - Cross-cutting error types
//! - Trail operation envelope for JSONL persistence
//! - CLI response types
//! - Audit detail sub-types
//! - Arrow serialization adapters for chrono types

pub mod arrow_serde;
pub mod audit_detail;
pub mod entities;
pub mod enums;
pub mod errors;
pub mod ids;
pub mod responses;
pub mod trail;
pub mod workspace;

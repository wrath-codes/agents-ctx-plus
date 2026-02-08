//! # zen-schema
//!
//! JSON Schema generation, validation, and registry for Zenith.
//!
//! This crate provides:
//! - `SchemaRegistry`: central store of all JSON Schemas in the system
//! - Validation utilities for JSONL trail operations, audit details, config, and CLI responses
//! - Schema export for external tooling (`znt schema` command, editor plugins)
//!
//! ## Architecture
//!
//! Entity types are defined in `zen-core` with `#[derive(JsonSchema)]`.
//! This crate imports those types and provides the registry, validation, and export layer.
//! Consumer crates (zen-db, zen-hooks, zen-cli) depend on zen-schema for runtime validation.

#[cfg(test)]
mod spike_schema_gen;

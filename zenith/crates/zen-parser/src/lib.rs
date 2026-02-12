#![allow(clippy::cast_possible_truncation)]
//! # zen-parser
//!
//! ast-grep-based source code parsing and API extraction for Zenith.
//!
//! Supports all 26 ast-grep built-in languages with tiered extraction:
//! - **Rich extractors** (Rust, Python, TypeScript/TSX/JS, Go, Elixir, C#):
//!   full `ParsedItem` metadata with language-specific features
//! - **Generic extractor** (all other built-in languages):
//!   kind-based extraction capturing function/class/type definitions
//!
//! Symbol taxonomy is normalized across extractors:
//! - top-level callables use `Function`
//! - member callables use `Method` or `Constructor`
//! - member data uses `Field`/`Property`/`Event`/`Indexer`
//!
//! Member-level symbols should populate `SymbolMetadata::owner_name`,
//! `SymbolMetadata::owner_kind`, and `SymbolMetadata::is_static_member`.
//!
//! Uses a two-tier fallback: ast-grep `KindMatcher` → regex.

pub mod error;
pub mod extractors;
pub mod parser;
pub mod types;

pub use error::ParserError;
pub use parser::{detect_language, parse_source};
pub use types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

#[cfg(test)]
mod spike_ast_grep;

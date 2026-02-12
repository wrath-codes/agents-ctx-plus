#![allow(clippy::cast_possible_truncation)]
//! # zen-parser
//!
//! ast-grep-based source code parsing and API extraction for Zenith.
//!
//! Supports all 26 ast-grep built-in languages with tiered extraction:
//! - **Rich extractors** (Rust, Python, TypeScript/TSX/JS, Go, Elixir):
//!   full `ParsedItem` metadata with language-specific features
//! - **Generic extractor** (all other built-in languages):
//!   kind-based extraction capturing function/class/type definitions
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

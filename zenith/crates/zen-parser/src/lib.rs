#![allow(clippy::cast_possible_truncation)]
//! # zen-parser
//!
//! ast-grep-based source code parsing and API extraction for Zenith.
//!
//! Supports all 26 ast-grep built-in languages with tiered extraction:
//! - **Rich extractors** (Rust, Python, TypeScript/TSX/JS, Go, Elixir, C#, Haskell, Java, Lua, PHP, Ruby, JSON, YAML):
//!   full `ParsedItem` metadata with language-specific features
//! - **Generic extractor** (all other built-in languages):
//!   kind-based extraction capturing function/class/type definitions
//! - **Custom language lane** (Markdown via `tree-sitter-md`, TOML via `tree-sitter-toml-ng`):
//!   parser-backed extraction using a custom ast-grep `Language`
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
pub use parser::{
    DetectedLanguage, MarkdownLang, TomlLang, detect_language, detect_language_ext,
    parse_markdown_source, parse_source, parse_toml_source,
};
pub use types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

#[cfg(test)]
mod spike_ast_grep;

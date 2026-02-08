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
//! Uses a two-tier fallback: ast-grep pattern matching -> regex.

mod spike_ast_grep;

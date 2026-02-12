//! Elixir rich extractor - `call`-node-first strategy.
//!
//! In Elixir's tree-sitter grammar, all definitions are `call` nodes.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../elixir/helpers.rs"]
mod elixir_helpers;
#[path = "../elixir/processors/mod.rs"]
mod processors;

use elixir_helpers::first_identifier_text;
use processors::{
    dedup_multi_clause, process_def, process_defdelegate, process_defexception, process_defguard,
    process_defimpl, process_defmacro, process_defmodule, process_defprotocol, process_defstruct,
    try_extract_type_attr,
};

/// Elixir definition keywords we extract at any nesting depth.
const ELIXIR_DEF_KEYWORDS: &[&str] = &[
    "def",
    "defp",
    "defmacro",
    "defmacrop",
    "defmodule",
    "defprotocol",
    "defimpl",
    "defstruct",
    "defexception",
    "defguard",
    "defguardp",
    "defdelegate",
];

/// Extract all API symbols from an Elixir source file.
///
/// # Errors
/// Returns `ParserError` if extraction fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matcher = KindMatcher::new("call", SupportLang::Elixir);

    for node in root.root().find_all(&matcher) {
        let Some(keyword) = first_identifier_text(&node) else {
            continue;
        };

        if !ELIXIR_DEF_KEYWORDS.contains(&keyword.as_str()) {
            continue;
        }

        match keyword.as_str() {
            "defmodule" => {
                if let Some(item) = process_defmodule(&node) {
                    items.push(item);
                }
            }
            "def" => {
                if let Some(item) = process_def(&node, crate::types::Visibility::Public) {
                    items.push(item);
                }
            }
            "defp" => {
                if let Some(item) = process_def(&node, crate::types::Visibility::Private) {
                    items.push(item);
                }
            }
            "defmacro" => {
                if let Some(item) = process_defmacro(&node, crate::types::Visibility::Public) {
                    items.push(item);
                }
            }
            "defmacrop" => {
                if let Some(item) = process_defmacro(&node, crate::types::Visibility::Private) {
                    items.push(item);
                }
            }
            "defprotocol" => {
                if let Some(item) = process_defprotocol(&node) {
                    items.push(item);
                }
            }
            "defimpl" => {
                if let Some(item) = process_defimpl(&node) {
                    items.push(item);
                }
            }
            "defstruct" => items.push(process_defstruct(&node)),
            "defexception" => items.push(process_defexception(&node)),
            "defguard" => {
                if let Some(item) = process_defguard(&node, crate::types::Visibility::Public) {
                    items.push(item);
                }
            }
            "defguardp" => {
                if let Some(item) = process_defguard(&node, crate::types::Visibility::Private) {
                    items.push(item);
                }
            }
            "defdelegate" => {
                if let Some(item) = process_defdelegate(&node) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // Second pass: extract @type/@typep/@opaque from unary_operator nodes
    let unary_matcher = KindMatcher::new("unary_operator", SupportLang::Elixir);
    for node in root.root().find_all(&unary_matcher) {
        if let Some(item) = try_extract_type_attr(&node) {
            items.push(item);
        }
    }

    dedup_multi_clause(&mut items);
    Ok(items)
}

#[cfg(test)]
#[path = "../elixir/tests/mod.rs"]
mod tests;

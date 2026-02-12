//! HTML rich extractor.
//!
//! Extracts structurally significant elements from HTML documents:
//! custom elements (web components), elements with `id` attributes,
//! `<template>` elements, `<form>` elements, `<script>`/`<link>` resource
//! references, and semantic landmarks.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../html/helpers.rs"]
mod html_helpers;
#[path = "../html/processors.rs"]
mod processors;

/// Extract all significant elements from an HTML document.
///
/// Walks the entire document tree collecting:
/// - Custom elements (tag names containing `-`)
/// - Elements with `id` attributes
/// - `<template>`, `<form>`, `<dialog>`, `<details>` elements
/// - `<script>` and `<link>` resource references
/// - `<meta>` tags with `name` attribute
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    processors::collect_elements(&root_node, &mut items);
    Ok(items)
}

#[cfg(test)]
#[path = "../html/tests/mod.rs"]
mod tests;

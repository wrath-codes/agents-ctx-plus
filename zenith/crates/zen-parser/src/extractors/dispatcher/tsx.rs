//! TSX/React rich extractor.
//!
//! Delegates to the [`typescript`](super::typescript) extractor for base
//! symbol extraction (functions, classes, interfaces, enums, etc.), then
//! enriches each item with React/JSX-specific metadata.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../tsx/processors/mod.rs"]
mod processors;
#[path = "../tsx/helpers.rs"]
mod tsx_helpers;

/// Extract all API symbols from a TSX source file with React metadata.
///
/// Runs the TypeScript extractor first, then walks the original AST a
/// second time to attach JSX/React information to each item.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    lang: SupportLang,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = super::typescript::extract(root, lang)?;
    let root_node = root.root();
    processors::enrich_items(&root_node, &mut items);
    Ok(items)
}

#[cfg(test)]
#[path = "../tsx/tests/mod.rs"]
mod tests;

//! `CSS` rich extractor.
//!
//! Extracts structurally significant elements from CSS stylesheets.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../css/helpers.rs"]
mod css_helpers;
#[path = "../css/processors.rs"]
mod processors;

/// Extract all significant elements from a CSS stylesheet.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    processors::collect_nodes(&root_node, &mut items, None);
    Ok(items)
}

#[cfg(test)]
#[path = "../css/tests/mod.rs"]
mod tests;

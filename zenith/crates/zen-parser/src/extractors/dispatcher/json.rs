//! JSON rich extractor.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../json/helpers.rs"]
mod json_helpers;
#[path = "../json/processors.rs"]
mod processors;

/// Extract all significant JSON symbols from a document.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    Ok(processors::extract_document(&root.root()))
}

#[cfg(test)]
#[path = "../json/tests/mod.rs"]
mod tests;

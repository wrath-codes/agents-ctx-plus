//! YAML rich extractor.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../yaml/helpers.rs"]
mod yaml_helpers;
#[path = "../yaml/processors.rs"]
mod processors;

/// Extract significant YAML symbols from a document stream.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    Ok(processors::extract_stream(&root.root()))
}

#[cfg(test)]
#[path = "../yaml/tests/mod.rs"]
mod tests;

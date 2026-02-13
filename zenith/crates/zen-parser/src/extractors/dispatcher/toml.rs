//! TOML extractor powered by custom `tree-sitter-toml-ng` language support.

use crate::types::ParsedItem;

#[path = "../toml/processors.rs"]
mod processors;
#[path = "../toml/helpers.rs"]
mod toml_helpers;

/// Extract significant TOML symbols from a document.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    Ok(processors::extract_document(&root.root()))
}

#[cfg(test)]
#[path = "../toml/tests/mod.rs"]
mod tests;

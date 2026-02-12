//! C++ rich extractor.

use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../cpp/processors.rs"]
mod processors;

/// Extract all significant elements from a C++ source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    processors::extract(root, source)
}

#[cfg(test)]
#[path = "../cpp/tests/mod.rs"]
mod tests;

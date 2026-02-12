//! Haskell rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../haskell/helpers.rs"]
mod hs_helpers;
#[path = "../haskell/processors/mod.rs"]
mod processors;

const HASKELL_TOP_KINDS: &[&str] = &[
    "header",
    "module",
    "import",
    "function",
    "bind",
    "signature",
    "class",
    "data_type",
    "newtype",
    "type_family",
    "type_instance",
    "foreign_import",
    "foreign_export",
];

/// Extract all API symbols from a Haskell source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = HASKELL_TOP_KINDS
        .iter()
        .map(|kind| KindMatcher::new(kind, SupportLang::Haskell))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "header" | "module" => {
                if let Some(item) = processors::process_module(&node) {
                    items.push(item);
                }
            }
            "import" => {
                if let Some(item) = processors::process_import(&node) {
                    items.push(item);
                }
            }
            "function" | "bind" | "signature" | "foreign_import" | "foreign_export" => {
                if let Some(item) = processors::process_function_like(&node) {
                    items.push(item);
                }
            }
            "class" => {
                if let Some(item) = processors::process_class_decl(&node) {
                    items.push(item);
                }
            }
            "data_type" | "newtype" | "type_family" | "type_instance" => {
                if let Some(item) = processors::process_type_decl(&node) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    Ok(processors::dedupe_and_merge(items))
}

#[cfg(test)]
#[path = "../haskell/tests/mod.rs"]
mod tests;

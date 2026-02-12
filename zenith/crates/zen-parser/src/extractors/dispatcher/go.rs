//! Go rich extractor - `KindMatcher`-first strategy.
//!
//! Extracts functions, methods, type declarations (struct, interface,
//! type alias, function type), constants, and variables with Go-specific
//! metadata including receiver types, exported detection, and doc comments.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../go/helpers.rs"]
mod go_helpers;
#[path = "../go/processors.rs"]
mod processors;

const GO_TOP_KINDS: &[&str] = &[
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "const_declaration",
    "var_declaration",
];

/// Extract all API symbols from a Go source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = GO_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Go))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "function_declaration" => {
                if let Some(item) = processors::process_function(&node) {
                    items.push(item);
                }
            }
            "method_declaration" => {
                if let Some(item) = processors::process_method(&node) {
                    items.push(item);
                }
            }
            "type_declaration" => {
                items.extend(processors::process_type_declaration(&node));
            }
            "const_declaration" => {
                items.extend(processors::process_const_declaration(&node));
            }
            "var_declaration" => {
                items.extend(processors::process_var_declaration(&node));
            }
            _ => {}
        }
    }
    Ok(items)
}

#[cfg(test)]
#[path = "../go/tests/mod.rs"]
mod tests;

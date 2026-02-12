//! JavaScript rich extractor.
//!
//! Handles plain JavaScript (ES2015+) including generator functions,
//! classes with getters/setters/static methods, arrow functions,
//! `export` statements, and `JSDoc` comment extraction.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../javascript/helpers.rs"]
mod js_helpers;
#[path = "../javascript/processors/mod.rs"]
mod processors;

const JS_TOP_KINDS: &[&str] = &[
    "export_statement",
    "function_declaration",
    "generator_function_declaration",
    "class_declaration",
    "lexical_declaration",
    "variable_declaration",
];

/// Extract all API symbols from a JavaScript source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = JS_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::JavaScript))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "export_statement" => {
                items.extend(processors::process_export_statement(&node));
            }
            "function_declaration" => {
                if let Some(item) = processors::process_function(&node, &node, false, false) {
                    items.push(item);
                }
            }
            "generator_function_declaration" => {
                if let Some(item) = processors::process_generator_function(&node, &node, false) {
                    items.push(item);
                }
            }
            "class_declaration" => {
                if let Some(item) = processors::process_class(&node, &node, false, false) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(processors::process_lexical_declaration(&node, &node, false));
            }
            "variable_declaration" => {
                items.extend(processors::process_variable_declaration(
                    &node, &node, false,
                ));
            }
            _ => {}
        }
    }
    Ok(items)
}

#[cfg(test)]
#[path = "../javascript/tests/mod.rs"]
mod tests;

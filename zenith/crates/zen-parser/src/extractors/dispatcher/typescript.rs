//! TypeScript/JavaScript/TSX rich extractor.
//!
//! Shared extractor for TypeScript, JavaScript, and TSX. Extracts
//! functions, classes, interfaces, type aliases, enums, and arrow
//! functions with `JSDoc` support and export detection.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../typescript/processors/mod.rs"]
mod processors;
#[path = "../typescript/helpers.rs"]
mod ts_helpers;

const TS_TOP_KINDS: &[&str] = &[
    "export_statement",
    "function_declaration",
    "class_declaration",
    "abstract_class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
    "lexical_declaration",
    "variable_declaration",
    "ambient_declaration",
    "internal_module",
    "function_signature",
];

/// Extract all API symbols from a TypeScript/JavaScript/TSX source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    lang: SupportLang,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = TS_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, lang))
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
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(item) = processors::process_class(&node, &node, false, false) {
                    items.push(item);
                }
                items.extend(processors::process_class_members(&node, false));
            }
            "interface_declaration" => {
                if let Some(item) = processors::process_interface(&node, &node, false) {
                    items.push(item);
                }
                items.extend(processors::process_interface_members(&node, false));
            }
            "type_alias_declaration" => {
                if let Some(item) = processors::process_type_alias(&node, &node, false) {
                    items.push(item);
                }
            }
            "enum_declaration" => {
                if let Some(item) = processors::process_enum(&node, &node, false) {
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
            "ambient_declaration" => {
                items.extend(processors::process_ambient_declaration(&node));
            }
            "internal_module" => {
                if let Some(item) = processors::process_namespace(&node, &node, false) {
                    items.push(item);
                }
            }
            "function_signature" => {
                if let Some(item) = processors::process_function_signature(&node, &node) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    Ok(items)
}

#[cfg(test)]
#[path = "../typescript/tests/mod.rs"]
mod tests;

//! PHP rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../php/helpers.rs"]
mod php_helpers;
#[path = "../php/processors/mod.rs"]
mod processors;

const PHP_TOP_KINDS: &[&str] = &[
    "namespace_definition",
    "namespace_use_declaration",
    "use_declaration",
    "function_definition",
    "anonymous_function",
    "arrow_function",
    "object_creation_expression",
    "class_declaration",
    "interface_declaration",
    "trait_declaration",
    "enum_declaration",
    "method_declaration",
    "property_declaration",
    "const_declaration",
    "property_promotion_parameter",
    "enum_case",
    "global_declaration",
    "function_static_declaration",
];

/// Extract all API symbols from a PHP source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = PHP_TOP_KINDS
        .iter()
        .map(|kind| KindMatcher::new(kind, SupportLang::Php))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "namespace_definition" => {
                if let Some(item) = processors::process_module_like(&node) {
                    items.push(item);
                }
            }
            "namespace_use_declaration" => {
                items.extend(processors::process_namespace_use_declaration(&node));
            }
            "function_definition" => {
                if let Some(item) = processors::process_function_definition(&node) {
                    items.push(item);
                }
            }
            "anonymous_function" | "arrow_function" | "object_creation_expression" => {
                if let Some(item) = processors::process_inline_symbol(&node) {
                    items.push(item);
                }
            }
            "class_declaration"
            | "interface_declaration"
            | "trait_declaration"
            | "enum_declaration" => {
                if let Some(item) = processors::process_type_declaration(&node) {
                    items.push(item);
                }
            }
            "method_declaration"
            | "property_declaration"
            | "const_declaration"
            | "property_promotion_parameter"
            | "enum_case"
            | "use_declaration"
            | "global_declaration"
            | "function_static_declaration" => items.extend(processors::process_member_like(&node)),
            _ => {}
        }
    }

    Ok(processors::dedupe(items))
}

#[cfg(test)]
#[path = "../php/tests/mod.rs"]
mod tests;

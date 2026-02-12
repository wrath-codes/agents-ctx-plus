//! Java rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../java/helpers.rs"]
mod java_helpers;
#[path = "../java/processors/mod.rs"]
mod processors;

const JAVA_TOP_KINDS: &[&str] = &[
    "package_declaration",
    "import_declaration",
    "module_declaration",
    "requires_module_directive",
    "exports_module_directive",
    "opens_module_directive",
    "uses_module_directive",
    "provides_module_directive",
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
    "method_declaration",
    "annotation_type_element_declaration",
    "constructor_declaration",
    "compact_constructor_declaration",
    "field_declaration",
    "constant_declaration",
];

/// Extract all API symbols from a Java source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = JAVA_TOP_KINDS
        .iter()
        .map(|kind| KindMatcher::new(kind, SupportLang::Java))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "package_declaration" | "import_declaration" | "module_declaration" => {
                if let Some(item) = processors::process_module_like(&node) {
                    items.push(item);
                }
            }
            "requires_module_directive"
            | "exports_module_directive"
            | "opens_module_directive"
            | "uses_module_directive"
            | "provides_module_directive" => {
                if let Some(item) = processors::process_module_directive(&node) {
                    items.push(item);
                }
            }
            "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration" => {
                items.extend(processors::process_type_declaration(&node));
            }
            _ => items.extend(processors::process_member_declaration(&node)),
        }
    }

    Ok(processors::dedupe(items))
}

#[cfg(test)]
#[path = "../java/tests/mod.rs"]
mod tests;

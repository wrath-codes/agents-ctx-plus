//! C# rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../csharp/helpers.rs"]
mod cs_helpers;
#[path = "../csharp/processors/mod.rs"]
mod processors;

const CSHARP_TOP_KINDS: &[&str] = &[
    "using_directive",
    "namespace_declaration",
    "file_scoped_namespace_declaration",
    "class_declaration",
    "record_declaration",
    "struct_declaration",
    "interface_declaration",
    "enum_declaration",
    "delegate_declaration",
    "method_declaration",
    "constructor_declaration",
    "property_declaration",
    "field_declaration",
    "event_declaration",
    "event_field_declaration",
    "indexer_declaration",
    "operator_declaration",
    "conversion_operator_declaration",
    "destructor_declaration",
];

/// Extract all API symbols from a C# source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = CSHARP_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::CSharp))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "using_directive" => {
                if let Some(item) = processors::process_using_directive(&node) {
                    items.push(item);
                }
            }
            "namespace_declaration" | "file_scoped_namespace_declaration" => {
                if let Some(item) = processors::process_namespace(&node) {
                    items.push(item);
                }
            }
            "class_declaration"
            | "record_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "delegate_declaration" => {
                if let Some(item) = processors::process_type_declaration(&node) {
                    items.push(item);
                }
            }
            _ => items.extend(processors::process_member_declaration(&node)),
        }
    }

    Ok(items)
}

#[cfg(test)]
#[path = "../csharp/tests/mod.rs"]
mod tests;

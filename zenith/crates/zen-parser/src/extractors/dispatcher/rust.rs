//! Rust rich extractor — `KindMatcher`-first strategy (spike 0.8 validated).
//!
//! Extracts functions, structs, enums, traits, impl blocks, type aliases,
//! modules, consts, statics, macros, and unions with full metadata.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../rust/processors.rs"]
mod processors;

const RUST_ITEM_KINDS: &[&str] = &[
    "function_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "impl_item",
    "type_item",
    "mod_item",
    "const_item",
    "static_item",
    "macro_definition",
    "union_item",
    "foreign_mod_item",
    "use_declaration",
    "extern_crate_declaration",
    "macro_invocation",
];

/// Extract all API symbols from a Rust source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = RUST_ITEM_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Rust))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        items.extend(processors::process_match_node(&node, source));
    }
    Ok(items)
}

#[cfg(test)]
#[path = "../rust/tests/mod.rs"]
mod tests;

//! Ruby rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../ruby/processors/mod.rs"]
mod processors;
#[path = "../ruby/helpers.rs"]
mod ruby_helpers;

const RUBY_TOP_KINDS: &[&str] = &[
    "class",
    "module",
    "method",
    "singleton_method",
    "assignment",
    "call",
];

/// Extract all API symbols from a Ruby source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = RUBY_TOP_KINDS
        .iter()
        .map(|kind| KindMatcher::new(kind, SupportLang::Ruby))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "class" | "module" => {
                if let Some(item) = processors::process_type_declaration(&node) {
                    items.push(item);
                }
            }
            "method" | "singleton_method" => {
                if let Some(item) = processors::process_method_declaration(&node) {
                    items.push(item);
                }
            }
            "assignment" => {
                if let Some(item) = processors::process_assignment(&node) {
                    items.push(item);
                }
            }
            "call" => items.extend(processors::process_call(&node)),
            _ => {}
        }
    }

    Ok(processors::dedupe(items))
}

#[cfg(test)]
#[path = "../ruby/tests/mod.rs"]
mod tests;

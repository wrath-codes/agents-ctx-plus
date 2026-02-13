//! Lua rich extractor.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

#[path = "../lua/helpers.rs"]
mod lua_helpers;
#[path = "../lua/processors/mod.rs"]
mod processors;

const LUA_TOP_KINDS: &[&str] = &[
    "function_declaration",
    "variable_declaration",
    "assignment_statement",
];

/// Extract all API symbols from a Lua source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let matchers: Vec<KindMatcher> = LUA_TOP_KINDS
        .iter()
        .map(|kind| KindMatcher::new(kind, SupportLang::Lua))
        .collect();
    let matcher = Any::new(matchers);

    let mut items = Vec::new();
    for node in root.root().find_all(&matcher) {
        match node.kind().as_ref() {
            "function_declaration" => {
                if let Some(item) = processors::process_function_declaration(&node) {
                    items.push(item);
                }
            }
            "variable_declaration" => {
                items.extend(processors::process_variable_declaration(&node));
            }
            "assignment_statement" => {
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind().as_ref() == "variable_declaration")
                {
                    continue;
                }
                items.extend(processors::process_assignment_statement(&node));
            }
            _ => {}
        }
    }

    Ok(processors::dedupe(items))
}

#[cfg(test)]
#[path = "../lua/tests/mod.rs"]
mod tests;

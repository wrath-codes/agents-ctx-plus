use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{ParsedItem, SymbolKind, Visibility};

mod anonymous_and_inline;
mod callable_context_edge_cases;
mod docs_signatures_lines;
mod imports_and_aliases;
mod members;
mod ownership_edge_cases;
mod phpdoc_and_attributes;
mod phpdoc_edge_cases;
mod trait_adaptation_edge_cases;
mod type_canonicalization_edge_cases;
mod types_and_modules;
mod types_and_signatures;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Php.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.php");
    parse_and_extract(source)
}
